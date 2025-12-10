package configs

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"

	yaml "gopkg.in/yaml.v3"
)

type Rule struct {
	ID       string   `yaml:"id"`
	Patterns []string `yaml:"patterns"`
	Schema   string   `yaml:"schema"`
	// Optional fields from riglet.json to support overrides
	Fields struct {
		PatternsOverride []string `json:"patternsOverride" yaml:"patternsOverride"`
	} `yaml:"fields" json:"fields"`
}

type RigletConfig struct {
	Rules       []Rule     `yaml:"rules"`
	Sync        []SyncRule `yaml:"sync"`
	Profile     string     `yaml:"profile"`     // selected profile package/folder name
	NoOverwrite string     `yaml:"noOverwrite"` // optional glob: files that must never be overwritten
}

type customConv struct{ cc Rule }

func (c customConv) ID() string { return c.cc.ID }
func (c customConv) Patterns() []string {
	// Merge base patterns with optional overrides, deduplicated
	m := map[string]struct{}{}
	out := []string{}
	for _, p := range c.cc.Patterns {
		if _, ok := m[p]; !ok {
			m[p] = struct{}{}
			out = append(out, p)
		}
	}
	for _, p := range c.cc.Fields.PatternsOverride {
		if _, ok := m[p]; !ok {
			m[p] = struct{}{}
			out = append(out, p)
		}
	}
	return out
}
func (c customConv) SchemaFor(path, repoRoot string) string {
	// If schema is relative, resolve against repo root
	if filepath.IsAbs(c.cc.Schema) {
		return c.cc.Schema
	}
	return filepath.Join(repoRoot, c.cc.Schema)
}

// LoadRigletConfig loads riglet.yaml if present at repoRoot.
func LoadRigletConfig(repoRoot string) (*RigletConfig, error) {
	cfgPath := filepath.Join(repoRoot, "riglet.yaml")
	b, err := os.ReadFile(cfgPath)
	if err != nil {
		return nil, err
	}
	var cfg RigletConfig
	if err := yaml.Unmarshal(b, &cfg); err != nil {
		return nil, err
	}
	return &cfg, nil
}

// RegisterCustom registers conventions from riglet.config.yaml when available.
func RegisterCustom(r *Registry, cfg *RigletConfig) {
	if cfg == nil {
		return
	}
	for _, cc := range cfg.Rules {
		r.Register(customConv{cc: cc})
	}
}

// SyncRule describes a copy/sync action
type SyncRule struct {
	ID        string `yaml:"id"`
	Source    string `yaml:"source"`    // path or glob, relative to repo root
	Target    string `yaml:"target"`    // relative target path
	When      string `yaml:"when"`      // root|packages|all
	Overwrite bool   `yaml:"overwrite"` // whether to overwrite existing files
}

// ConventionIndex describes riglet metadata inside conventions/<name>/riglet.json
type ConventionIndex struct {
	Profile string     `json:"profile"`
	Rules   []Rule     `json:"rules"`
	Sync    []SyncRule `json:"sync"`
}

// LoadSelectedConvention determines the convention name from riglet.config.yaml or root package.json.
func LoadSelectedProfile(repoRoot string, cfg *RigletConfig) string {
	if cfg != nil && cfg.Profile != "" {
		return cfg.Profile
	}
	// Try root package.json: { riglet: { profile: "nazahex" } }
	b, err := os.ReadFile(filepath.Join(repoRoot, "package.json"))
	if err != nil {
		return ""
	}
	var m map[string]any
	if err := json.Unmarshal(b, &m); err != nil {
		return ""
	}
	if r, ok := m["riglet"].(map[string]any); ok {
		if s, ok := r["profile"].(string); ok {
			return s
		}
	}
	return ""
}

// RegisterConventionPackage loads conventions+sync from either an npm-style package (resolved under repo) or a local path.
// Precedence: package name > path string.
func RegisterConventionPackage(r *Registry, repoRoot, name string, cfg *RigletConfig) (*RigletConfig, error) {
	if name == "" {
		return cfg, nil
	}
	// If name looks like a path (starts with '.' or '/' or contains path separators), treat it as a local path to riglet.json
	var idxPath string
	if strings.HasPrefix(name, ".") || strings.HasPrefix(name, "/") || strings.Contains(name, string(filepath.Separator)) {
		// local path: may be a directory containing riglet.json or the file itself
		base := name
		if !filepath.IsAbs(base) {
			base = filepath.Join(repoRoot, base)
		}
		// if base is a directory, append riglet.json
		fi, err := os.Stat(base)
		if err == nil && fi.IsDir() {
			idxPath = filepath.Join(base, "riglet.json")
		} else {
			idxPath = base
		}
	} else {
		// package-style: convention folder under repo (flat layout). Expect <repoRoot>/<name>/riglet.json
		idxPath = filepath.Join(repoRoot, name, "riglet.json")
		if _, err := os.Stat(idxPath); err != nil {
			// fallback to legacy conventions/<name>/riglet.json if flat layout not found
			legacy := filepath.Join(repoRoot, "conventions", name, "riglet.json")
			if _, err2 := os.Stat(legacy); err2 == nil {
				idxPath = legacy
			}
		}
	}
	b, err := os.ReadFile(idxPath)
	if err != nil {
		return cfg, err
	}
	var idx ConventionIndex
	if err := json.Unmarshal(b, &idx); err != nil {
		return cfg, err
	}
	// Register conventions from index (schema path relative to the convention source root)
	baseDir := filepath.Dir(idxPath)
	for _, c := range idx.Rules {
		// Resolve relative schema to absolute within convention dir
		if !filepath.IsAbs(c.Schema) {
			c.Schema = filepath.Join(baseDir, c.Schema)
		}
		r.Register(customConv{cc: c})
	}
	// Merge sync rules into cfg (so sync command can use them)
	if cfg == nil {
		cfg = &RigletConfig{}
	}
	cfg.Sync = append(cfg.Sync, idx.Sync...)
	return cfg, nil
}
