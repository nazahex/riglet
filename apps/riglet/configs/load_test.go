package configs

import (
	"os"
	"path/filepath"
	"testing"
)

func TestLoadSelectedProfile_FromYaml(t *testing.T) {
	d := t.TempDir()
	// riglet.yaml defines convention
	if err := os.WriteFile(filepath.Join(d, "riglet.yaml"), []byte("profile: \"@org/riglet\"\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	cfg, err := LoadRigletConfig(d)
	if err != nil {
		t.Fatal(err)
	}
	got := LoadSelectedProfile(d, cfg)
	if got != "@org/riglet" {
		t.Fatalf("expected @org/riglet, got %s", got)
	}
}

func TestLoadSelectedProfile_FromPackageJSON(t *testing.T) {
	d := t.TempDir()
	// no riglet.yaml
	if err := os.WriteFile(filepath.Join(d, "package.json"), []byte(`{"riglet":{"profile":"@org/p"}}`), 0o644); err != nil {
		t.Fatal(err)
	}
	got := LoadSelectedProfile(d, nil)
	if got != "@org/p" {
		t.Fatalf("expected @org/p, got %s", got)
	}
}
