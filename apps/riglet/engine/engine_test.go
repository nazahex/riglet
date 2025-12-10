package engine

import (
	"os"
	"path/filepath"
	"testing"
)

type simpleIgnore struct{}

func (simpleIgnore) ShouldSkip(rel string, isDir bool) bool {
	return rel == "skipme.txt" || (isDir && rel == "skipdir")
}

func TestWalkFilesWithIgnore(t *testing.T) {
	d := t.TempDir()
	// create dirs
	os.MkdirAll(filepath.Join(d, "node_modules/dep"), 0o755)
	os.MkdirAll(filepath.Join(d, "skipdir"), 0o755)
	// create files
	os.WriteFile(filepath.Join(d, "a.txt"), []byte("a"), 0o644)
	os.WriteFile(filepath.Join(d, "skipme.txt"), []byte("x"), 0o644)
	os.WriteFile(filepath.Join(d, "node_modules/x.txt"), []byte("m"), 0o644)

	seen := []string{}
	err := WalkFilesWithIgnore(d, simpleIgnore{}, func(p string) error {
		seen = append(seen, filepath.Base(p))
		return nil
	})
	if err != nil {
		t.Fatalf("walk: %v", err)
	}
	// should not include node_modules file or skipme.txt
	for _, b := range seen {
		if b == "skipme.txt" || b == "x.txt" {
			t.Fatalf("ignored file was seen: %s", b)
		}
	}
	// ensure a.txt present
	found := false
	for _, b := range seen {
		if b == "a.txt" {
			found = true
		}
	}
	if !found {
		t.Fatalf("a.txt not seen")
	}
}
