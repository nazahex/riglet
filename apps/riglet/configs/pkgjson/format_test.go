package pkgjson

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestFormatFile_SubpackageOrdering_OK(t *testing.T) {
	d := t.TempDir()
	p := filepath.Join(d, "package.json")
	if err := os.WriteFile(p, []byte(`{
        "name":"x",
        "dependencies": {"b":"1","a":"1"},
        "scripts": {"z":"run","a":"start"},
        "repository": {"type":"git","url":"x"},
        "version":"1.0.0"
    }`), 0o644); err != nil {
		t.Fatal(err)
	}
	out, err := FormatFile(p, Options{IsRoot: false})
	if err != nil {
		t.Fatal(err)
	}
	s := string(out)
	// pretty output should contain indentation and sorted maps
	depKey := `"dependencies": {`
	depPos := strings.Index(s, depKey)
	if depPos < 0 || depPos+len(depKey) >= len(s) || s[depPos+len(depKey)] != '\n' {
		t.Fatalf("dependencies not pretty (no newline after '{'): %s", s)
	}
	// scripts pretty formatting may be compact; do not enforce newline
	// ensure order inside dependencies: a before b
	depStart := strings.Index(s, `"dependencies": {`)
	depEnd := strings.Index(s[depStart:], `}`)
	if depStart < 0 || depEnd < 0 {
		t.Fatalf("dependencies section not found: %s", s)
	}
	deps := s[depStart : depStart+depEnd]
	if !(strings.Contains(deps, `"a": "1"`) && strings.Contains(deps, `"b": "1"`) && strings.Index(deps, `"a": "1"`) < strings.Index(deps, `"b": "1"`)) {
		t.Fatalf("dependencies not sorted: %s", deps)
	}
	// scripts order should be preserved (not alphabetically sorted). Expect 'z' before 'a' from input
	scrStart := strings.Index(s, `"scripts": {`)
	scrEnd := strings.Index(s[scrStart:], `}`)
	if scrStart < 0 || scrEnd < 0 {
		t.Fatalf("scripts section not found: %s", s)
	}
	scr := s[scrStart : scrStart+scrEnd]
	if !(strings.Contains(scr, `"z": "run"`) && strings.Contains(scr, `"a": "start"`) && strings.Index(scr, `"z": "run"`) < strings.Index(scr, `"a": "start"`)) {
		t.Fatalf("scripts order not preserved: %s", scr)
	}
	if !(strings.Index(s, `"repository"`) < strings.Index(s, `"scripts"`)) {
		t.Fatalf("top-level order unexpected: %s", s)
	}
}

func TestFormatFile_TopLevelSectionBreaks(t *testing.T) {
	d := t.TempDir()
	p := filepath.Join(d, "package.json")
	// Keys span multiple sections: {name, version, repository} (sec1) then {scripts, dependencies} (sec2)
	if err := os.WriteFile(p, []byte(`{
		"name":"pkg",
		"version":"0.1.0",
		"repository": {"type":"git","url":"x"},
		"scripts": {"build":"x"},
		"dependencies": {"a":"1"}
	}`), 0o644); err != nil {
		t.Fatal(err)
	}
	out, err := FormatFile(p, Options{IsRoot: false})
	if err != nil {
		t.Fatal(err)
	}
	s := string(out)
	// Ensure section break (blank line) between sec1 and sec2 (repository -> scripts)
	repoIdx := strings.Index(s, `"repository"`)
	scrIdx := strings.Index(s, `"scripts"`)
	if repoIdx < 0 || scrIdx < 0 || !(repoIdx < scrIdx) {
		t.Fatalf("unexpected order or missing keys: %s", s)
	}
	// Look for a comma followed by two newlines between repository and scripts
	afterRepo := strings.Index(s[repoIdx:], "},")
	if afterRepo < 0 {
		t.Fatalf("could not find end of repository value: %s", s)
	}
	seg := s[repoIdx+afterRepo : scrIdx]
	if !strings.Contains(seg, ",\n\n") {
		t.Fatalf("expected a blank line between sections, got: %q", seg)
	}
}

func TestFormatFile_CustomAuthorOrder(t *testing.T) {
	d := t.TempDir()
	p := filepath.Join(d, "package.json")
	if err := os.WriteFile(p, []byte(`{
		"name":"x",
		"author": {"url":"https://x", "name":"Kaz", "email":"a@x"}
	}`), 0o644); err != nil {
		t.Fatal(err)
	}
	out, err := FormatFile(p, Options{IsRoot: false})
	if err != nil {
		t.Fatal(err)
	}
	s := string(out)
	aStart := strings.Index(s, `"author": {`)
	if aStart < 0 {
		t.Fatalf("author section not found: %s", s)
	}
	aEnd := strings.Index(s[aStart:], `}`)
	if aEnd < 0 {
		t.Fatalf("author section not closed: %s", s)
	}
	seg := s[aStart : aStart+aEnd]
	// Expect name first, then email, then url
	in := []string{`"name":`, `"email":`, `"url":`}
	last := -1
	for _, k := range in {
		idx := strings.Index(seg, k)
		if idx < 0 {
			t.Fatalf("missing key %s in author: %s", k, seg)
		}
		if idx < last {
			t.Fatalf("wrong key order in author: %s", seg)
		}
		last = idx
	}
}
