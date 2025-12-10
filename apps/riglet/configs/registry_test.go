package configs

import (
	"path/filepath"
	"testing"
)

func TestRegistryMatchPatterns(t *testing.T) {
	r := NewRegistry()
	r.SetRepoRoot("/repo")
	// register a convention with complex patterns
	r.Register(customConv{cc: Rule{ID: "pkgjson.sub", Patterns: []string{"packages/*/package.json", "apps/**/package.json"}}})

	cases := []struct {
		path string
		ok   bool
	}{
		{"/repo/package.json", false}, // not in this convention's patterns
		{"/repo/packages/a/package.json", true},
		{"/repo/apps/frontend/web/package.json", true},
		{"/repo/modules/x/package.json", false},
	}
	for _, c := range cases {
		m := r.Match(filepath.ToSlash(c.path))
		if c.ok && len(m) == 0 {
			t.Errorf("expected match for %s", c.path)
		}
		if !c.ok && len(m) > 0 {
			t.Errorf("expected no match for %s", c.path)
		}
	}
}
