package main

import (
	"bytes"
	"encoding/json"
	"flag"
	"fmt"
	"os"
	"path/filepath"
	"strings"

	"github.com/nazahex/riglet/configs"
	pkgjson "github.com/nazahex/riglet/configs/pkgjson"
	"github.com/nazahex/riglet/engine"
	yaml "gopkg.in/yaml.v3"
)

const version = "0.1.0"

func main() {
	if len(os.Args) < 2 {
		usage()
		os.Exit(2)
	}

	cmd := os.Args[1]
	switch cmd {
	case "version":
		fmt.Println(version)
	case "validate":
		validateCmd(os.Args[2:])
	case "lint":
		lintCmd(os.Args[2:])
	case "format":
		formatCmd(os.Args[2:])
	case "sync":
		syncCmd(os.Args[2:])
	case "check":
		checkCmd(os.Args[2:])
	default:
		usage()
		os.Exit(2)
	}
}

func usage() {
	fmt.Println("riglet commands:")
	fmt.Println("  version                       Show version")
	fmt.Println("  validate --schema <cue> --input <json|yaml>  Validate input against CUE schema")
	fmt.Println("  lint [--repo-root <path>] [--scope repo|workspace|all] [--config <riglet.yaml>]  Lint configs by conventions")
	fmt.Println("  format [--repo-root <path>] [--scope repo|workspace|all] [--write]             Format configs deterministically")
	fmt.Println("  sync   [--repo-root <path>] [--scope repo|workspace|all] [--dry-run] [--config <riglet.yaml>]  Apply templates and format configs")
	fmt.Println("  check  [--repo-root <path>] [--scope repo|workspace|all]                  Run format (check) + lint and fail on issues")
}

func validateCmd(args []string) {
	fs := flag.NewFlagSet("validate", flag.ExitOnError)
	schemaPath := fs.String("schema", "", "Path to CUE schema file")
	inputPath := fs.String("input", "", "Path to JSON or YAML input file")
	_ = fs.Parse(args)

	if *schemaPath == "" || *inputPath == "" {
		fmt.Fprintln(os.Stderr, "validate: --schema and --input are required")
		os.Exit(2)
	}

	sch, err := engine.LoadSchema([]string{*schemaPath})
	if err != nil {
		fmt.Fprintf(os.Stderr, "failed to load schema: %v\n", err)
		os.Exit(1)
	}

	data, err := os.ReadFile(*inputPath)
	if err != nil {
		fmt.Fprintf(os.Stderr, "failed to read input: %v\n", err)
		os.Exit(1)
	}

	ext := filepath.Ext(*inputPath)
	var val any
	switch ext {
	case ".json":
		if err := json.Unmarshal(data, &val); err != nil {
			fmt.Fprintf(os.Stderr, "invalid JSON: %v\n", err)
			os.Exit(1)
		}
	case ".yaml", ".yml":
		v, err := engine.DecodeYAML(data)
		if err != nil {
			fmt.Fprintf(os.Stderr, "invalid YAML: %v\n", err)
			os.Exit(1)
		}
		val = v
	default:
		fmt.Fprintf(os.Stderr, "unsupported input extension: %s\n", ext)
		os.Exit(2)
	}

	if err := sch.Validate(val); err != nil {
		fmt.Fprintf(os.Stderr, "validation failed: %v\n", err)
		os.Exit(1)
	}
	fmt.Println("OK")
}

func lintCmd(args []string) {
	fs := flag.NewFlagSet("lint", flag.ExitOnError)
	repoRoot := fs.String("repo-root", "", "Repository root (defaults to current directory)")
	cfgPath := fs.String("config", "", "Optional path to riglet.yaml")
	scope := fs.String("scope", "repo", "Scope for lint: repo|workspace|all (default: repo)")
	output := fs.String("output", "human", "Output format: human|json|table")
	fix := fs.Bool("fix", false, "Apply autofix for fixable rules (e.g., pkgjson.format)")
	_ = fs.Parse(args)

	root := *repoRoot
	if root == "" {
		wd, _ := os.Getwd()
		root = wd
	}

	// Build registry
	reg := buildRegistry(root, *cfgPath)
	if *scope == "repo" {
		reg.SetRepoRoot(root)
	}

	// Build ignore matcher (.rigletignore + defaults)
	ign := engine.LoadRigletIgnore(root)

	type Issue struct {
		File         string `json:"file"`
		ConventionID string `json:"conventionID"`
		RuleID       string `json:"ruleID,omitempty"`
		Severity     string `json:"severity"`
		Path         string `json:"path,omitempty"`
		Message      string `json:"message"`
	}
	type LintResult struct {
		Issues  []Issue `json:"issues"`
		Summary struct {
			Errors   int `json:"errors"`
			Warnings int `json:"warnings"`
			Infos    int `json:"infos"`
			Files    int `json:"files"`
		} `json:"summary"`
	}
	var failed int
	res := LintResult{}
	// Prepare optional workspace path set
	var workspaceSet map[string]struct{}
	if *scope == "workspace" {
		ws, _ := resolveWorkspacePackageJSONs(root)
		// ensure non-nil map for consistent checks
		if ws == nil {
			ws = make(map[string]struct{})
		}
		workspaceSet = ws
	}

	err := engine.WalkFilesWithIgnore(root, ign, func(path string) error {
		// Scope-level filtering for repo: only root package.json and packages/*/package.json
		switch *scope {
		case "repo":
			base := filepath.ToSlash(filepath.Clean(path))
			rr := filepath.ToSlash(filepath.Clean(root))
			if !(base == rr || base == rr+"/package.json" || strings.HasPrefix(base, rr+"/packages/")) {
				return nil
			}
			// If under repoRoot but not package.json, skip
			rel := strings.TrimPrefix(base, rr+"/")
			if rel != "package.json" && filepath.Base(base) != "package.json" {
				return nil
			}
		case "workspace":
			if filepath.Base(path) != "package.json" {
				return nil
			}
			if workspaceSet != nil {
				p := filepath.ToSlash(filepath.Clean(path))
				if _, ok := workspaceSet[p]; !ok {
					return nil
				}
			}
		}
		matches := reg.Match(path)
		if len(matches) == 0 {
			return nil
		}
		for _, c := range matches {
			schema := c.SchemaFor(path, root)
			if schema == "" {
				fmt.Fprintf(os.Stderr, "No convention schema for %s (%s). Configure a convention via riglet.yaml or install a convention package.\n", path, c.ID())
				failed++
				continue
			}
			sch, err := engine.LoadSchema([]string{schema})
			if err != nil {
				fmt.Fprintf(os.Stderr, "schema load error for %s (%s): %v\nHint: Add riglet.yaml with 'convention: <package-or-path>' or provide a local convention containing riglet.json and schemas.\n", path, c.ID(), err)
				failed++
				continue
			}
			data, err := os.ReadFile(path)
			if err != nil {
				fmt.Fprintf(os.Stderr, "read error for %s: %v\n", path, err)
				failed++
				continue
			}
			ext := filepath.Ext(path)
			var val any
			switch ext {
			case ".json":
				if err := json.Unmarshal(data, &val); err != nil {
					fmt.Fprintf(os.Stderr, "invalid JSON in %s: %v\n", path, err)
					failed++
					continue
				}
			case ".yaml", ".yml":
				v, err := engine.DecodeYAML(data)
				if err != nil {
					fmt.Fprintf(os.Stderr, "invalid YAML in %s: %v\n", path, err)
					failed++
					continue
				}
				val = v
			default:
				// Skip unsupported extensions for this convention
				continue
			}
			if err := sch.Validate(val); err != nil {
				// For now, collect a single error issue per file with schema error message.
				res.Issues = append(res.Issues, Issue{
					File:         path,
					ConventionID: c.ID(),
					Severity:     "error",
					Message:      err.Error(),
				})
				failed++
				return nil
			}
			// In human mode, show OK lines; JSON mode will summarize only.
			// Additional formatting diagnostics for package.json
			if filepath.Base(path) == "package.json" {
				rr := filepath.ToSlash(filepath.Clean(root))
				p := filepath.ToSlash(filepath.Clean(path))
				formatted, ferr := pkgjson.FormatFile(path, pkgjson.Options{IsRoot: p == rr+"/package.json"})
				if ferr == nil {
					current, _ := os.ReadFile(path)
					if !bytes.Equal(bytes.TrimSpace(current), bytes.TrimSpace(formatted)) {
						// Report formatting deviation as a warning with ruleID
						res.Issues = append(res.Issues, Issue{
							File:         path,
							ConventionID: c.ID(),
							RuleID:       "pkgjson.format",
							Severity:     "warning",
							Path:         "$",
							Message:      "File is not in canonical pretty format; run 'riglet format --write'",
						})
					}
					// autofix formatting when --fix set
					if *fix {
						if werr := os.WriteFile(path, formatted, 0o644); werr == nil {
							fmt.Printf("FIXED: %s (pkgjson.format)\n", path)
						}
					}

					// Rule checks: top-level order and dependency maps sort
					var obj map[string]any
					if json.Unmarshal(current, &obj) == nil {
						// Top-level canonical order subsequence check
						canon := pkgjson.OrderKeys()
						seen := make([]string, 0, len(obj))
						dec := json.NewDecoder(bytes.NewReader(current))
						if tok, _ := dec.Token(); tok == json.Delim('{') {
							for dec.More() {
								tk, _ := dec.Token()
								if k, ok := tk.(string); ok {
									seen = append(seen, k)
									_ = skipJSONValue(dec)
								} else {
									break
								}
							}
						}
						mismatch := false
						last := -1
						for _, k := range canon {
							for idx, sk := range seen {
								if sk == k {
									if idx < last {
										mismatch = true
									}
									last = idx
									break
								}
							}
							if mismatch {
								break
							}
						}
						if mismatch {
							res.Issues = append(res.Issues, Issue{
								File:         path,
								ConventionID: c.ID(),
								RuleID:       "pkgjson.order.top-level",
								Severity:     "warning",
								Path:         "$",
								Message:      "Top-level keys out of canonical order.",
							})
						}

						// Dependency maps: ensure keys sorted
						for _, field := range []string{"dependencies", "devDependencies", "peerDependencies", "optionalDependencies"} {
							keys := []string{}
							dec2 := json.NewDecoder(bytes.NewReader(current))
							if tok, _ := dec2.Token(); tok == json.Delim('{') {
								for dec2.More() {
									tk, _ := dec2.Token()
									if k, ok := tk.(string); ok {
										if k == field {
											if tok2, _ := dec2.Token(); tok2 == json.Delim('{') {
												for dec2.More() {
													kt, _ := dec2.Token()
													if ks, ok := kt.(string); ok {
														keys = append(keys, ks)
														_ = skipJSONValue(dec2)
													} else {
														break
													}
												}
												_, _ = dec2.Token()
											}
											break
										} else {
											_ = skipJSONValue(dec2)
										}
									} else {
										break
									}
								}
							}
							if len(keys) > 1 {
								sorted := true
								for i := 1; i < len(keys); i++ {
									if strings.Compare(keys[i-1], keys[i]) > 0 {
										sorted = false
										break
									}
								}
								if !sorted {
									res.Issues = append(res.Issues, Issue{
										File:         path,
										ConventionID: c.ID(),
										RuleID:       "pkgjson.maps.sort",
										Severity:     "warning",
										Path:         "$." + field,
										Message:      field + " is not sorted alphabetically by key.",
									})
								}
							}
						}
					}
				}
			}
			if *output == "human" {
				fmt.Printf("OK: %s (%s)\n", path, c.ID())
			}
			res.Summary.Files++
		}
		return nil
	})
	if err != nil {
		fmt.Fprintf(os.Stderr, "walk error: %v\n", err)
		os.Exit(1)
	}
	// Emit results
	if *output == "json" {
		// count severities
		for _, is := range res.Issues {
			switch is.Severity {
			case "error":
				res.Summary.Errors++
			case "warning":
				res.Summary.Warnings++
			default:
				res.Summary.Infos++
			}
		}
		enc := json.NewEncoder(os.Stdout)
		enc.SetIndent("", "  ")
		_ = enc.Encode(res)
	} else if *output == "table" {
		// human table output
		fmt.Println("File | Rule | Severity | Path | Message")
		fmt.Println("-----|------|----------|------|--------")
		for _, is := range res.Issues {
			rule := is.RuleID
			if rule == "" { rule = cullRuleFromMessage(is.Message) }
			fmt.Printf("%s | %s | %s | %s | %s\n", is.File, rule, is.Severity, is.Path, is.Message)
		}
		// summary
		fmt.Printf("Summary: errors=%d warnings=%d infos=%d files=%d\n", res.Summary.Errors, res.Summary.Warnings, res.Summary.Infos, res.Summary.Files)
	} else {
		for _, is := range res.Issues {
			fmt.Fprintf(os.Stderr, "ERROR %s (%s): %s\n", is.File, is.ConventionID, is.Message)
		}
	}
	if failed > 0 {
		os.Exit(1)
	}
}

func formatCmd(args []string) {
	fs := flag.NewFlagSet("format", flag.ExitOnError)
	repoRoot := fs.String("repo-root", "", "Repository root (defaults to current directory)")
	scope := fs.String("scope", "repo", "Scope for format: repo|workspace|all (default: repo)")
	write := fs.Bool("write", false, "Write changes back to files")
	_ = fs.Parse(args)

	root := *repoRoot
	if root == "" {
		wd, _ := os.Getwd()
		root = wd
	}

	// ignore matcher
	ign := engine.LoadRigletIgnore(root)

	// optional workspace set
	var workspaceSet map[string]struct{}
	if *scope == "workspace" {
		ws, _ := resolveWorkspacePackageJSONs(root)
		if ws == nil {
			ws = make(map[string]struct{})
		}
		workspaceSet = ws
	}

	// run
	var changed int
	err := engine.WalkFilesWithIgnore(root, ign, func(path string) error {
		// Scope filtering similar to lint
		base := filepath.Base(path)
		if base != "package.json" {
			return nil
		}
		p := filepath.ToSlash(filepath.Clean(path))
		rr := filepath.ToSlash(filepath.Clean(root))
		switch *scope {
		case "repo":
			if !(p == rr+"/package.json" || strings.HasPrefix(p, rr+"/packages/")) {
				return nil
			}
		case "workspace":
			if _, ok := workspaceSet[p]; !ok {
				return nil
			}
		}

		// Determine if root package.json
		isRoot := p == rr+"/package.json"

		// Format package.json
		out, err := pkgjson.FormatFile(path, pkgjson.Options{IsRoot: isRoot})
		if err != nil {
			fmt.Fprintf(os.Stderr, "format error for %s: %v\n", path, err)
			return nil
		}
		// Compare with current
		current, _ := os.ReadFile(path)
		if !bytes.Equal(bytes.TrimSpace(current), bytes.TrimSpace(out)) {
			if *write {
				if err := os.WriteFile(path, out, 0o644); err != nil {
					fmt.Fprintf(os.Stderr, "write error for %s: %v\n", path, err)
				} else {
					fmt.Printf("WROTE: %s\n", path)
				}
			} else {
				fmt.Printf("NEEDS-FORMAT: %s\n", path)
			}
			changed++
		} else {
			fmt.Printf("OK: %s\n", path)
		}
		return nil
	})
	if err != nil {
		fmt.Fprintf(os.Stderr, "walk error: %v\n", err)
		os.Exit(1)
	}
	if changed > 0 && !*write {
		// in check mode, exit non-zero if changes needed
		os.Exit(1)
	}
}

func syncCmd(args []string) {
	fs := flag.NewFlagSet("sync", flag.ExitOnError)
	repoRoot := fs.String("repo-root", "", "Repository root (defaults to current directory)")
	scope := fs.String("scope", "all", "Scope for sync: repo|workspace|all (default: all)")
	dry := fs.Bool("dry-run", false, "Only print actions without writing")
	cfgPath := fs.String("config", "", "Optional path to riglet.yaml")
	_ = fs.Parse(args)

	root := *repoRoot
	if root == "" {
		wd, _ := os.Getwd()
		root = wd
	}

	ign := engine.LoadRigletIgnore(root)

	// Always format package.json as part of sync
	fmtArgs := []string{"--repo-root", root, "--scope", *scope, "--write"}
	formatCmd(fmtArgs)

	// Load config
	var cfg *configs.RigletConfig
	if *cfgPath != "" {
		if c, err := configs.LoadRigletConfig(filepath.Dir(*cfgPath)); err == nil {
			cfg = c
		}
	} else {
		if c, err := configs.LoadRigletConfig(root); err == nil {
			cfg = c
		}
	}
	if cfg == nil || len(cfg.Sync) == 0 {
		return
	}

	// workspace set (for packages scope)
	wsSet, _ := resolveWorkspacePackageJSONs(root)

	// Execute sync rules
	for _, rule := range cfg.Sync {
		when := rule.When
		if when == "" {
			when = "all"
		}
		switch when {
		case "root":
			applySyncRule(root, ign, rule, root, cfg.NoOverwrite, *dry)
		case "packages":
			for pj := range wsSet {
				dir := filepath.Dir(pj)
				applySyncRule(root, ign, rule, dir, cfg.NoOverwrite, *dry)
			}
		case "all":
			applySyncRule(root, ign, rule, root, cfg.NoOverwrite, *dry)
			for pj := range wsSet {
				dir := filepath.Dir(pj)
				applySyncRule(root, ign, rule, dir, cfg.NoOverwrite, *dry)
			}
		default:
			// unknown when: skip
		}
	}
}

func checkCmd(args []string) {
	fs := flag.NewFlagSet("check", flag.ExitOnError)
	repoRoot := fs.String("repo-root", "", "Repository root (defaults to current directory)")
	scope := fs.String("scope", "repo", "Scope for check: repo|workspace|all (default: repo)")
	_ = fs.Parse(args)

	root := *repoRoot
	if root == "" {
		wd, _ := os.Getwd()
		root = wd
	}

	// Run format in check mode
	fmtArgs := []string{"--repo-root", root, "--scope", *scope}
	formatCmd(fmtArgs)

	// Run lint; if it fails, process will exit with non-zero
	lintArgs := []string{"--repo-root", root, "--scope", *scope}
	lintCmd(lintArgs)
}

func applySyncRule(repoRoot string, ign engine.IgnoreMatcher, rule configs.SyncRule, destBase string, noOverwrite string, dry bool) {
	rr := filepath.ToSlash(filepath.Clean(repoRoot))
	srcPattern := filepath.ToSlash(filepath.Join(rr, rule.Source))
	matches, _ := filepath.Glob(srcPattern)
	if len(matches) == 0 {
		return
	}
	for _, src := range matches {
		fi, err := os.Stat(src)
		if err != nil {
			continue
		}
		// Compute target path
		target := filepath.Join(destBase, rule.Target)
		// If source is a directory, copy its contents into target dir
		if fi.IsDir() {
			// ensure target directory exists
			_ = os.MkdirAll(target, 0o755)
			walkErr := filepath.WalkDir(src, func(p string, d os.DirEntry, err error) error {
				if err != nil {
					return err
				}
				rel, _ := filepath.Rel(src, p)
				if rel == "." {
					return nil
				}
				isDir := d.IsDir()
				relSlash := filepath.ToSlash(rel)
				if ign != nil && ign.ShouldSkip(relSlash, isDir) {
					if isDir {
						return filepath.SkipDir
					}
					return nil
				}
				dst := filepath.Join(target, rel)
				if isDir {
					return os.MkdirAll(dst, 0o755)
				}
				return copyOneWithTokens(repoRoot, destBase, p, dst, rule.Overwrite, noOverwrite, dry)
			})
			if walkErr != nil {
				fmt.Fprintf(os.Stderr, "sync error: %v\n", walkErr)
			}
		} else {
			// Single file copy
			// Ensure parent dir
			_ = os.MkdirAll(filepath.Dir(target), 0o755)
			_ = copyOneWithTokens(repoRoot, destBase, src, target, rule.Overwrite, noOverwrite, dry)
		}
	}
}

func copyOne(src, dst string, overwrite, dry bool) error {
	if !overwrite {
		if _, err := os.Stat(dst); err == nil {
			// exists, skip
			fmt.Printf("SKIP (exists): %s\n", dst)
			return nil
		}
	}
	if dry {
		fmt.Printf("COPY %s -> %s\n", src, dst)
		return nil
	}
	b, err := os.ReadFile(src)
	if err != nil {
		return err
	}
	if err := os.WriteFile(dst, b, 0o644); err != nil {
		return err
	}
	fmt.Printf("WROTE: %s\n", dst)
	return nil
}

// copyOneWithTokens performs copy with simple token substitution for text files.
// Supported tokens: {{repo.name}}, {{package.name}}
func copyOneWithTokens(repoRoot, destBase, src, dst string, overwrite bool, noOverwrite string, dry bool) error {
	// If binary? We treat everything as text for simplicity; token replacement on UTF-8
	// respect global noOverwrite pattern
	if noOverwrite != "" {
		if matchNoOverwrite(noOverwrite, destBase, dst) {
			overwrite = false
		}
	}
	if !overwrite {
		if _, err := os.Stat(dst); err == nil {
			fmt.Printf("SKIP (exists): %s\n", dst)
			return nil
		}
	}
	if dry {
		fmt.Printf("COPY %s -> %s\n", src, dst)
		return nil
	}
	b, err := os.ReadFile(src)
	if err != nil {
		return err
	}
	// Gather tokens
	tokens := map[string]string{
		"{{repo.name}}":    readJSONString(filepath.Join(repoRoot, "package.json"), "name"),
		"{{package.name}}": readJSONString(filepath.Join(destBase, "package.json"), "name"),
	}
	s := string(b)
	for k, v := range tokens {
		if v != "" {
			s = strings.ReplaceAll(s, k, v)
		}
	}
	if err := os.MkdirAll(filepath.Dir(dst), 0o755); err != nil {
		return err
	}
	if err := os.WriteFile(dst, []byte(s), 0o644); err != nil {
		return err
	}
	fmt.Printf("WROTE: %s\n", dst)
	return nil
}

func matchNoOverwrite(pattern, destBase, dst string) bool {
	// Try to match against relative path to destBase and basename
	rel, err := filepath.Rel(destBase, dst)
	relSlash := rel
	if err == nil {
		relSlash = filepath.ToSlash(rel)
		if ok, _ := filepath.Match(pattern, relSlash); ok {
			return true
		}
	}
	base := filepath.Base(dst)
	if ok, _ := filepath.Match(pattern, base); ok {
		return true
	}
	// Also try matching full slashed path
	if ok, _ := filepath.Match(pattern, filepath.ToSlash(dst)); ok {
		return true
	}
	return false
}

func readJSONString(path, key string) string {
	b, err := os.ReadFile(path)
	if err != nil {
		return ""
	}
	var m map[string]any
	if json.Unmarshal(b, &m) != nil {
		return ""
	}
	if v, ok := m[key].(string); ok {
		return v
	}
	return ""
}

func buildRegistry(repoRoot string, cfgPath string) *configs.Registry {
	r := configs.NewRegistry()
	configs.RegisterBuiltins(r)
	// load custom conventions
	var cfg *configs.RigletConfig
	if cfgPath != "" {
		if c, err := configs.LoadRigletConfig(filepath.Dir(cfgPath)); err == nil {
			cfg = c
		}
	} else {
		if c, err := configs.LoadRigletConfig(repoRoot); err == nil {
			cfg = c
		}
	}
	// Allow selecting a convention package folder name (zero-autodiscovery; user explicitly names it)
	selected := configs.LoadSelectedProfile(repoRoot, cfg)
	if updated, err := configs.RegisterConventionPackage(r, repoRoot, selected, cfg); err == nil {
		cfg = updated
	}
	configs.RegisterCustom(r, cfg)
	return r
}

// cullRuleFromMessage is a placeholder to derive a short rule name from a raw message.
// In future, messages will be mapped to explicit ruleIDs.
func cullRuleFromMessage(msg string) string {
	if strings.Contains(msg, "repository") {
		return "pkgjson.repository"
	}
	return "schema"
}

// skipJSONValue consumes the next JSON value in the decoder to advance tokens.
func skipJSONValue(dec *json.Decoder) error {
	tok, err := dec.Token()
	if err != nil {
		return err
	}
	if d, ok := tok.(json.Delim); ok {
		switch d {
		case '{':
			for dec.More() {
				if _, err := dec.Token(); err != nil {
					return err
				}
				if err := skipJSONValue(dec); err != nil {
					return err
				}
			}
			_, _ = dec.Token() // consume '}'
		case '[':
			for dec.More() {
				if err := skipJSONValue(dec); err != nil {
					return err
				}
			}
			_, _ = dec.Token() // consume ']'
		}
	}
	return nil
}

// resolveWorkspacePackageJSONs discovers workspace package.json files using
// root package.json "workspaces" or a fallback to packages/*.
func resolveWorkspacePackageJSONs(repoRoot string) (map[string]struct{}, error) {
	rr := filepath.ToSlash(filepath.Clean(repoRoot))
	rootPkg := filepath.Join(rr, "package.json")
	b, err := os.ReadFile(rootPkg)
	if err != nil {
		return nil, err
	}
	var root any
	if err := json.Unmarshal(b, &root); err != nil {
		return nil, err
	}
	// helper to collect paths from patterns
	collect := func(patterns []string) map[string]struct{} {
		out := make(map[string]struct{})
		for _, pat := range patterns {
			pat = strings.TrimSpace(pat)
			if pat == "" {
				continue
			}
			abs := filepath.ToSlash(filepath.Join(rr, pat))
			// Try glob expansion; note Go's Glob doesn't support **, but supports * which is common for packages/*
			matches, _ := filepath.Glob(abs)
			if len(matches) == 0 {
				// Fallback: if pattern points to a directory, try it directly
				if fi, err := os.Stat(abs); err == nil && fi.IsDir() {
					matches = []string{abs}
				}
			}
			for _, m := range matches {
				pj := filepath.ToSlash(filepath.Join(m, "package.json"))
				if _, err := os.Stat(pj); err == nil {
					out[pj] = struct{}{}
				}
			}
		}
		return out
	}

	// Try workspaces formats
	if obj, ok := root.(map[string]any); ok {
		if ws, ok := obj["workspaces"]; ok {
			switch v := ws.(type) {
			case []any:
				var pats []string
				for _, it := range v {
					if s, ok := it.(string); ok {
						pats = append(pats, s)
					}
				}
				set := collect(pats)
				if len(set) > 0 {
					return set, nil
				}
			case map[string]any:
				if pk, ok := v["packages"].([]any); ok {
					var pats []string
					for _, it := range pk {
						if s, ok := it.(string); ok {
							pats = append(pats, s)
						}
					}
					set := collect(pats)
					if len(set) > 0 {
						return set, nil
					}
				}
			}
		}
		// Fallback: top-level packages field used in some toolchains (e.g., pnpm)
		if pk, ok := obj["packages"].([]any); ok {
			var pats []string
			for _, it := range pk {
				if s, ok := it.(string); ok {
					pats = append(pats, s)
				}
			}
			set := collect(pats)
			if len(set) > 0 {
				return set, nil
			}
		}
	}
	// Try pnpm-workspace.yaml for packages patterns
	pnpmWS := filepath.Join(rr, "pnpm-workspace.yaml")
	if pb, err := os.ReadFile(pnpmWS); err == nil {
		var y struct {
			Packages []string `yaml:"packages"`
		}
		if yaml.Unmarshal(pb, &y) == nil && len(y.Packages) > 0 {
			set := collect(y.Packages)
			return set, nil
		}
	}
	// Final fallback to packages/*
	set := collect([]string{"packages/*"})
	return set, nil
}
