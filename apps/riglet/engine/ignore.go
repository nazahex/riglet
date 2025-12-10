package engine

import (
	"bufio"
	"os"
	"path/filepath"
	"strings"
)

// SimpleIgnore implements IgnoreMatcher using default patterns and optional user patterns.
// Paths are matched against repo-root-relative Unix-style paths.
type SimpleIgnore struct {
	repoRoot string
	patterns []string
}

// Default ignore patterns; directories are matched recursively.
var defaultPatterns = []string{
	"node_modules/**",
	".git/**",
	"dist/**",
	".turbo/**",
	".artifacts/**",
}

// LoadRigletIgnore builds an ignore matcher from .rigletignore and defaults.
func LoadRigletIgnore(repoRoot string) *SimpleIgnore {
	repoRoot = filepath.ToSlash(filepath.Clean(repoRoot))
	pats := make([]string, 0, len(defaultPatterns))
	pats = append(pats, defaultPatterns...)
	// Read .rigletignore if present
	fp := filepath.Join(repoRoot, ".rigletignore")
	f, err := os.Open(fp)
	if err == nil {
		s := bufio.NewScanner(f)
		for s.Scan() {
			line := strings.TrimSpace(s.Text())
			if line == "" || strings.HasPrefix(line, "#") {
				continue
			}
			// normalize to slash format
			line = filepath.ToSlash(line)
			pats = append(pats, line)
		}
		_ = f.Close()
	}
	return &SimpleIgnore{repoRoot: repoRoot, patterns: pats}
}

// ShouldSkip checks whether rel path should be skipped.
// Supports simple prefix and contains checks for "dir/**" style globs.
func (si *SimpleIgnore) ShouldSkip(rel string, isDir bool) bool {
	rel = filepath.ToSlash(filepath.Clean(rel))
	for _, p := range si.patterns {
		p = strings.TrimSpace(p)
		if p == "" {
			continue
		}
		// If pattern ends with "/**", treat as directory prefix
		if strings.HasSuffix(p, "/**") {
			prefix := strings.TrimSuffix(p, "/**")
			if rel == prefix || strings.HasPrefix(rel, prefix+"/") {
				return true
			}
			continue
		}
		// Exact file or prefix match for directories
		if rel == p || strings.HasPrefix(rel, p+"/") {
			return true
		}
	}
	return false
}
