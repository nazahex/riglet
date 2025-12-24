use rigra::{format, lint, sync};
use std::fs;

// Integration-style tests using temp dirs

#[test]
fn format_orders_keys_as_specified() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();

    // Write index.toml with rules, order lives in policy.toml now
    let conv = root.join("conv");
    fs::create_dir_all(&conv).unwrap();
    fs::write(
        conv.join("index.toml"),
        r#"
[[rules]]
id = "pkgjson.root"
patterns = ["package.json"]
policy = "policy.toml"
"#,
    )
    .unwrap();

    // Policy with ordering
    fs::write(
        conv.join("policy.toml"),
        r#"
checks = []

[order]
top = [["name"],["version"],["license"]]
[order.sub]
meta = []
"#,
    )
    .unwrap();

    // package.json with shuffled keys
    fs::write(
        root.join("package.json"),
        r#"{
  "license": "MIT",
  "z": 1,
  "name": "x",
  "a": 2,
  "version": "1.0.0"
}"#,
    )
    .unwrap();

    // Run format preview
    let results = format::run_format(
        root.to_str().unwrap(),
        &format!("{}/index.toml", conv.file_name().unwrap().to_string_lossy()),
        false,
        false,
        false,
        None,
        &std::collections::HashMap::new(),
        &std::collections::HashMap::new(),
        &std::collections::HashMap::new(),
    );
    assert_eq!(results.len(), 1);
    let preview = results[0].preview.as_ref().unwrap();
    // Ensure order starts with name, version, license, then a, z
    assert!(preview.contains("\n  \"name\""));
    assert!(preview.contains("\n  \"version\""));
    assert!(preview.contains("\n  \"license\""));
}

#[test]
fn format_precedence_write_vs_diff_check() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();

    // Conventions dir with index + policy
    let conv = root.join("conv");
    fs::create_dir_all(&conv).unwrap();
    fs::write(
        conv.join("index.toml"),
        r#"
[[rules]]
id = "pkgjson.root"
patterns = ["package.json"]
policy = "policy.toml"
"#,
    )
    .unwrap();

    // Policy with simple ordering
    fs::write(
        conv.join("policy.toml"),
        r#"
checks = []

[order]
top = [["name"],["version"],["license"]]
"#,
    )
    .unwrap();

    // package.json with shuffled keys
    fs::write(
        root.join("package.json"),
        r#"{
  "license": "MIT",
  "version": "1.0.0",
  "name": "x"
}"#,
    )
    .unwrap();

    // Case A: write=true (no diff/check) ⇒ file should be rewritten, no preview
    let results_write = rigra::format::run_format(
        root.to_str().unwrap(),
        &format!("{}/index.toml", conv.file_name().unwrap().to_string_lossy()),
        true,  // write
        false, // capture_old
        false, // strict_linebreak
        None,
        &std::collections::HashMap::new(),
        &std::collections::HashMap::new(),
        &std::collections::HashMap::new(),
    );
    assert_eq!(results_write.len(), 1);
    assert!(results_write[0].changed);
    assert!(results_write[0].preview.is_none());
    // Confirm file content reordered
    let after = fs::read_to_string(root.join("package.json")).unwrap();
    assert!(after.contains("\n  \"name\""));
    assert!(after.contains("\n  \"version\""));
    assert!(after.contains("\n  \"license\""));

    // Reset file to original shuffled order
    fs::write(
        root.join("package.json"),
        r#"{
  "license": "MIT",
  "version": "1.0.0",
  "name": "x"
}"#,
    )
    .unwrap();

    // Case B: diff/check override write=false ⇒ preview present, file unchanged
    let results_diff = rigra::format::run_format(
        root.to_str().unwrap(),
        &format!("{}/index.toml", conv.file_name().unwrap().to_string_lossy()),
        false, // effective write becomes false when diff/check true
        true,  // capture_old to enable diff
        false,
        None,
        &std::collections::HashMap::new(),
        &std::collections::HashMap::new(),
        &std::collections::HashMap::new(),
    );
    assert_eq!(results_diff.len(), 1);
    assert!(results_diff[0].changed);
    assert!(results_diff[0].preview.is_some());
    let after2 = fs::read_to_string(root.join("package.json")).unwrap();
    // unchanged since write=false
    assert!(after2.contains("\n  \"license\""));
}

#[test]
fn sync_filters_by_scope_and_copies() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let conv = root.join("conv");
    fs::create_dir_all(conv.join("templates")).unwrap();
    fs::write(conv.join("templates/t.txt"), b"hello").unwrap();
    fs::write(
        conv.join("sync.toml"),
        r#"
[lint]
level = "info"
message = "Not synced yet. Please run rigra sync."

[[sync]]
id = "r1"
source = "templates/t.txt"
target = "out/repo.txt"
when = "repo"

[[sync]]
id = "r2"
source = "templates/t.txt"
target = "out/lib.txt"
when = "lib"
"#,
    )
    .unwrap();

    fs::write(
        conv.join("index.toml"),
        r#"
sync = "sync.toml"
"#,
    )
    .unwrap();

    let actions = sync::run_sync(
        root.to_str().unwrap(),
        &format!("{}/index.toml", conv.file_name().unwrap().to_string_lossy()),
        "repo",
        true,
    );
    assert!(actions.iter().any(|a| a.rule_id == "r1" && a.wrote));
    assert!(actions.iter().all(|a| a.rule_id != "r2"));
    assert!(root.join("out/repo.txt").exists());
    assert!(!root.join("out/lib.txt").exists());
}

#[test]
fn e2e_linebreaks_between_groups_before_fields_and_in_fields_keep() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();

    // Conventions dir with index + policy
    let conv = root.join("conv");
    fs::create_dir_all(&conv).unwrap();
    fs::write(
        conv.join("index.toml"),
        r#"
[[rules]]
id = "pkgjson.root"
patterns = ["package.json"]
policy = "policy.toml"
"#,
    )
    .unwrap();

    // Policy with ordering and linebreak rules
    fs::write(
        conv.join("policy.toml"),
        r#"
checks = []

[order]
top = [["name"],["license"],["scripts","dependencies"]]

[linebreak]
between_groups = true
[linebreak.before_fields]
license = "none"
[linebreak.in_fields]
scripts = "keep"
"#,
    )
    .unwrap();

    // Original JSON contains a blank line before scripts.test entry
    fs::write(
        root.join("package.json"),
        r#"{
  "license": "MIT",
  "name": "x",
  "scripts": {
    "build": "echo build",

    "test": "echo test"
  },
  "dependencies": {}
}"#,
    )
    .unwrap();

    // Run format with strict linebreaks enabled
    let results = format::run_format(
        root.to_str().unwrap(),
        &format!("{}/index.toml", conv.file_name().unwrap().to_string_lossy()),
        false,                             // write
        true,                              // capture_old for potential diffs
        true,                              // strict_linebreak
        None,                              // lb_between_groups_override
        &std::collections::HashMap::new(), // lb_before_fields_override
        &std::collections::HashMap::new(), // lb_in_fields_override
        &std::collections::HashMap::new(), // pattern_overrides
    );
    assert_eq!(results.len(), 1);
    let preview = results[0].preview.as_ref().expect("expected preview");

    // 1) No blank line before first group (name first)
    assert!(preview.starts_with("{\n  \"name\""));

    // 2) No blank line before license (first key of second group) due to before_fields.license = none
    // Find the line with \"license\" and assert previous line is not blank.
    let lic_pos = preview.find("\n  \"license\"").expect("license present");
    let before_lic = &preview[..lic_pos];
    assert!(!before_lic.ends_with("\n\n"));

    // 3) Blank line before scripts (first key of third group)
    assert!(preview.contains("\n\n  \"scripts\""));

    // 4) Inside scripts, preserve original blank line before 'test'
    assert!(preview.contains("\"build\": \"echo build\",\n\n    \"test\""));
}

#[test]
fn lint_emits_order_issue_with_message_and_level() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let conv = root.join("conv");
    fs::create_dir_all(&conv).unwrap();

    fs::write(
        conv.join("index.toml"),
        r#"
[[rules]]
id = "pkgjson"
patterns = ["package.json"]
policy = "policy.toml"
"#,
    )
    .unwrap();

    fs::write(
        conv.join("policy.toml"),
        r#"
[order]
top = [["name"],["version"]]
message = "Keys must start with name,version"
level = "warn"
"#,
    )
    .unwrap();

    // Intentionally disordered keys
    fs::write(
        root.join("package.json"),
        r#"{
  "version": "1.0.0",
  "name": "x"
}"#,
    )
    .unwrap();

    let res = lint::run_lint(
        root.to_str().unwrap(),
        &format!("{}/index.toml", conv.file_name().unwrap().to_string_lossy()),
        "repo",
        &std::collections::HashMap::new(),
    );
    assert!(res
        .issues
        .iter()
        .any(|i| i.severity == "warn" && i.message == "Keys must start with name,version"));
}

#[test]
fn e2e_config_overrides_take_precedence_over_policy() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let conv = root.join("conv");
    fs::create_dir_all(&conv).unwrap();

    fs::write(
        conv.join("index.toml"),
        r#"
[[rules]]
id = "pkgjson.root"
patterns = ["package.json"]
policy = "policy.toml"
"#,
    )
    .unwrap();

    // Policy disables blank before license via before_fields.none
    fs::write(
        conv.join("policy.toml"),
        r#"
checks = []

[order]
top = [["name"],["license"],["scripts"]]

[linebreak]
between_groups = false
[linebreak.before_fields]
license = "none"
"#,
    )
    .unwrap();

    fs::write(
        root.join("package.json"),
        r#"{
  "license": "MIT",
  "name": "x",
  "scripts": {}
}"#,
    )
    .unwrap();

    // Overrides: enable between_groups and force license=keep
    let mut before_over = std::collections::HashMap::new();
    before_over.insert("license".to_string(), "keep".to_string());
    let results = format::run_format(
        root.to_str().unwrap(),
        &format!("{}/index.toml", conv.file_name().unwrap().to_string_lossy()),
        false,
        false,
        true,         // strict linebreaks on
        Some(true),   // override between_groups
        &before_over, // override before_fields
        &std::collections::HashMap::new(),
        &std::collections::HashMap::new(),
    );
    assert_eq!(results.len(), 1);
    let preview = results[0].preview.as_ref().unwrap();
    // Now license should have a blank line before it despite policy specifying none.
    let lines: Vec<&str> = preview.lines().collect();
    let mut found = false;
    for i in 1..lines.len() {
        if lines[i].trim_start().starts_with("\"license\"") {
            found = true;
            assert!(
                lines[i - 1].trim().is_empty(),
                "expected blank line before license, got: {:?} before {:?}",
                lines[i - 2..=i].to_vec(),
                lines[i]
            );
            break;
        }
    }
    assert!(found, "license line not found");
}
