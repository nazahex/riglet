//! Convention cache management and resolution.
//!
//! Implements minimal functions to:
//! - Parse `conv:` index strings (`conv:name@ver[:subpath]`)
//! - Resolve cache path under `.rigra/conv/name@ver/subpath`
//! - Install conventions from sources: `gh:owner/repo@tag` or `file:/abs/path`
//! - List and prune cache

use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ConvRef {
    pub name: String,
    pub ver: String,
    pub subpath: String, // defaults to index.toml when parsed
}

pub fn parse_conv_ref(s: &str) -> Option<ConvRef> {
    if !s.starts_with("conv:") {
        return None;
    }
    let body = &s[5..];
    // name@ver(:subpath)?
    let (nv, sp) = match body.split_once(':') {
        Some((nv, sp)) => (nv, Some(sp.to_string())),
        None => (body, None),
    };
    // Support scoped names like @owner/name by splitting at the LAST '@'
    let (name, ver) = nv.rsplit_once('@')?;
    Some(ConvRef {
        name: name.to_string(),
        ver: ver.to_string(),
        subpath: sp.unwrap_or_else(|| "index.toml".to_string()),
    })
}

pub fn cache_root(repo_root: &Path) -> PathBuf {
    repo_root.join(".rigra").join("conv")
}

pub fn resolve_path(repo_root: &Path, cr: &ConvRef) -> PathBuf {
    cache_root(repo_root)
        .join(cache_key(&cr.name, &cr.ver))
        .join(&cr.subpath)
}

#[derive(Debug, Clone)]
pub enum Source {
    Gh {
        owner: String,
        repo: String,
        tag: String,
    },
    File {
        path: String,
    },
}

pub fn parse_source(s: &str) -> Option<Source> {
    if let Some(rest) = s.strip_prefix("gh:") {
        // gh:owner/repo@tag
        let (or, tag) = rest.split_once('@')?;
        let (owner, repo) = or.split_once('/')?;
        return Some(Source::Gh {
            owner: owner.to_string(),
            repo: repo.to_string(),
            tag: tag.to_string(),
        });
    }
    if let Some(rest) = s.strip_prefix("file:") {
        return Some(Source::File {
            path: rest.to_string(),
        });
    }
    None
}

/// Install a convention into repo cache.
/// Uses system `curl` and `tar` to keep binary small.
pub fn install(repo_root: &Path, name_ver: &str, source_str: &str) -> Result<PathBuf, String> {
    let src = parse_source(source_str).ok_or_else(|| "invalid source".to_string())?;
    let (name, ver) = name_ver
        .rsplit_once('@')
        .ok_or_else(|| "name must be in form name@version".to_string())?;
    let dest_root = cache_root(repo_root).join(cache_key(name, ver));
    if dest_root.exists() {
        return Ok(dest_root);
    }
    fs::create_dir_all(&dest_root).map_err(|e| format!("create cache dir: {}", e))?;
    match src {
        Source::Gh { owner, repo, tag } => {
            let url = format!(
                "https://github.com/{}/{}/archive/refs/tags/{}.tar.gz",
                owner, repo, tag
            );
            let tmp = repo_root
                .join(".rigra")
                .join("tmp")
                .join(format!("{}-{}-{}.tar.gz", owner, repo, tag));
            let tmp_parent = tmp.parent().unwrap_or(Path::new("."));
            fs::create_dir_all(tmp_parent).map_err(|e| format!("prepare tmp: {}", e))?;
            let mut cmd = std::process::Command::new("curl");
            let st = cmd
                .args(["-fsSL", &url, "-o"])
                .arg(&tmp)
                .status()
                .map_err(|e| format!("curl exec failed: {}", e))?;
            if !st.success() {
                return Err(format!("curl download failed: exit {}", st));
            }
            let mut tar = std::process::Command::new("tar");
            let st = tar
                .arg("-xzf")
                .arg(&tmp)
                .arg("-C")
                .arg(&dest_root)
                .arg("--strip-components")
                .arg("1")
                .status()
                .map_err(|e| format!("tar exec failed: {}", e))?;
            if !st.success() {
                return Err(format!("tar extract failed: exit {}", st));
            }
            Ok(dest_root)
        }
        Source::File { path } => {
            let mut tar = std::process::Command::new("tar");
            let st = tar
                .arg("-xzf")
                .arg(&path)
                .arg("-C")
                .arg(&dest_root)
                .arg("--strip-components")
                .arg("1")
                .status()
                .map_err(|e| format!("tar exec failed: {}", e))?;
            if !st.success() {
                return Err(format!("tar extract failed: exit {}", st));
            }
            Ok(dest_root)
        }
    }
}

pub fn list(repo_root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let root = cache_root(repo_root);
    if let Ok(rd) = fs::read_dir(root) {
        for e in rd.flatten() {
            if let Ok(md) = e.metadata() {
                if md.is_dir() {
                    if let Some(name) = e.file_name().to_str() {
                        out.push(name.to_string());
                    }
                }
            }
        }
    }
    out.sort();
    out
}

pub fn prune(repo_root: &Path) -> Result<(), String> {
    let root = cache_root(repo_root);
    if root.exists() {
        fs::remove_dir_all(&root).map_err(|e| format!("prune failed: {}", e))?;
    }
    Ok(())
}

fn cache_key(name: &str, ver: &str) -> String {
    // Sanitize folder name: keep '@' but replace '/' with '__'
    let safe = name.replace('/', "__");
    format!("{}@{}", safe, ver)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_parse_conv_ref_with_and_without_subpath() {
        let a = parse_conv_ref("conv:hyper@v1.2.3").unwrap();
        assert_eq!(a.name, "hyper");
        assert_eq!(a.ver, "v1.2.3");
        assert_eq!(a.subpath, "index.toml");

        let b = parse_conv_ref("conv:hyper@v1.2.3:foo/bar.toml").unwrap();
        assert_eq!(b.subpath, "foo/bar.toml");
    }

    #[test]
    fn test_parse_source_gh_and_file() {
        match parse_source("gh:org/repo@v0.1.0").unwrap() {
            Source::Gh { owner, repo, tag } => {
                assert_eq!(owner, "org");
                assert_eq!(repo, "repo");
                assert_eq!(tag, "v0.1.0");
            }
            _ => panic!("expected gh source"),
        }
        match parse_source("file:/tmp/a.tar.gz").unwrap() {
            Source::File { path } => assert_eq!(path, "/tmp/a.tar.gz"),
            _ => panic!("expected file source"),
        }
    }

    #[test]
    fn test_resolve_path_list_and_prune() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        let cr = ConvRef {
            name: "hx".into(),
            ver: "v0".into(),
            subpath: "index.toml".into(),
        };
        let p = resolve_path(root, &cr);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        let mut f = fs::File::create(&p).unwrap();
        writeln!(f, "{}", "# index").unwrap();

        let items = list(root);
        assert_eq!(items, vec!["hx@v0".to_string()]);

        prune(root).unwrap();
        assert!(list(root).is_empty());
    }

    #[test]
    fn test_install_from_local_tarball() {
        // Prepare a staged directory to tar
        let dir = tempdir().unwrap();
        let root = dir.path();
        let staged = root.join("staged");
        fs::create_dir_all(staged.join("nested")).unwrap();
        fs::write(staged.join("index.toml"), "# idx").unwrap();
        fs::write(staged.join("nested/file.txt"), "data").unwrap();

        // Create tar.gz using system tar; if tar missing, this test will fail.
        let tgz = root.join("archive.tar.gz");
        let status = std::process::Command::new("tar")
            .current_dir(&staged)
            .args(["-czf", tgz.to_str().unwrap(), "."])
            .status()
            .expect("tar exec");
        assert!(status.success());

        // Install into cache
        let dest = install(
            root,
            "myconv@v0.1.0",
            &format!("file:{}", tgz.to_string_lossy()),
        )
        .unwrap();
        assert!(dest.join("index.toml").exists());
        assert!(dest.join("nested/file.txt").exists());
    }

    #[test]
    fn test_parse_conv_ref_scoped_name_and_cache_key() {
        let cr = parse_conv_ref("conv:@nazahex/conv-lib-ts-mono@v0.1.0").unwrap();
        assert_eq!(cr.name, "@nazahex/conv-lib-ts-mono");
        assert_eq!(cr.ver, "v0.1.0");
        let p = resolve_path(Path::new("/tmp"), &cr);
        let s = p.to_string_lossy();
        assert!(s.contains("@nazahex__conv-lib-ts-mono@v0.1.0"));
    }
}
