package configs

import (
	"os"
	"path/filepath"
	"testing"
)

func write(t *testing.T, p string, s string) {
	t.Helper()
	if err := os.MkdirAll(filepath.Dir(p), 0o755); err != nil {
		t.Fatalf("mkdir: %v", err)
	}
	if err := os.WriteFile(p, []byte(s), 0o644); err != nil {
		t.Fatalf("write: %v", err)
	}
}

func TestRegisterConventionPackage_PathVsPackage_Precedence(t *testing.T) {
	d := t.TempDir()
	// local path convention
	write(t, filepath.Join(d, "local/riglet.json"), `{"profile":"local","rules":[],"sync":[]}`)
	// package-style flat layout
	write(t, filepath.Join(d, "@org/riglet.json"), `{"profile":"pkg","rules":[],"sync":[]}`)

	r := NewRegistry()
	cfg := &RigletConfig{}
	// Prefer package when name doesn't look like a path
	if _, err := RegisterConventionPackage(r, d, "@org", cfg); err != nil {
		t.Fatalf("register package: %v", err)
	}
	// Use local path when explicitly provided path
	if _, err := RegisterConventionPackage(r, d, "./local", cfg); err != nil {
		t.Fatalf("register path: %v", err)
	}
}

func TestCustomConventionPatternsOverride(t *testing.T) {
	d := t.TempDir()
	// riglet.json with patterns and patternsOverride
	write(t, filepath.Join(d, "riglet.json"), `{
		"profile":"x",
		"rules":[{
			"id":"pkgjson.sub",
			"schema":"policies/pkgjson/sub.cue",
			"patterns":["packages/*/package.json"],
			"fields": {"patternsOverride": ["apps/**/package.json", "modules/**/package.json"]}
		}],
		"sync":[]
	}`)
	r := NewRegistry()
	cfg := &RigletConfig{}
	_, err := RegisterConventionPackage(r, d, d, cfg)
	if err != nil {
		t.Fatalf("register: %v", err)
	}
	got := r.All()
	if len(got) != 1 {
		t.Fatalf("expected 1 convention, got %d", len(got))
	}
	pats := got[0].Patterns()
	// expect merged 3 patterns
	if len(pats) != 3 {
		t.Fatalf("expected 3 patterns, got %d (%v)", len(pats), pats)
	}
}
