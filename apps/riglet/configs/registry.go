package configs

import (
	"path/filepath"
	"strings"
)

// Convention describes how a config type is detected and processed.
type Convention interface {
	ID() string
	Patterns() []string                            // file globs (e.g., "package.json", "**/tsconfig.json")
	SchemaFor(path string, repoRoot string) string // returns absolute schema path
}

// Registry holds registered conventions (built-in and custom).
type Registry struct {
	items    []Convention
	repoRoot string
}

func NewRegistry() *Registry { return &Registry{} }

func (r *Registry) Register(c Convention) { r.items = append(r.items, c) }

func (r *Registry) All() []Convention { return r.items }

func (r *Registry) SetRepoRoot(root string) { r.repoRoot = filepath.ToSlash(filepath.Clean(root)) }

// Match returns conventions that match the given path.
func (r *Registry) Match(path string) []Convention {
	var out []Convention
	base := filepath.ToSlash(filepath.Clean(path))
	// Scope filtering: only consider files under the configured repo root
	if r.repoRoot != "" {
		rr := r.repoRoot
		// Must be inside repoRoot
		if !(base == rr || strings.HasPrefix(base, rr+"/")) {
			return out
		}
		// Compute path relative to repoRoot
		// No additional scoping restrictions: allow any file to be matched by patterns
	}
	for _, c := range r.items {
		for _, pat := range c.Patterns() {
			if matchPattern(r.repoRoot, base, strings.TrimSpace(pat)) {
				out = append(out, c)
				break
			}
		}
	}
	return out
}

// matchPattern attempts to match a file against a pattern supporting:
// - exact basename (e.g., "package.json")
// - relative glob from repo root (e.g., "packages/*/package.json", "apps/**/package.json")
// - simple suffix match fallback
func matchPattern(repoRoot, absPath, pattern string) bool {
	rr := filepath.ToSlash(filepath.Clean(repoRoot))
	p := filepath.ToSlash(filepath.Clean(absPath))
	rel := p
	if rr != "" && (p == rr || strings.HasPrefix(p, rr+"/")) {
		rel = strings.TrimPrefix(p, rr+"/")
	}
	// exact basename
	if filepath.Base(p) == pattern || filepath.Base(rel) == pattern {
		return true
	}
	// normalize pattern
	pat := filepath.ToSlash(strings.TrimSpace(pattern))
	// support ** by turning it into a naive substring match when combined with following segment
	if strings.Contains(pat, "**/") {
		// e.g., apps/**/package.json -> check suffix and segment containment
		parts := strings.Split(pat, "**/")
		prefix := parts[0]
		suffix := parts[1]
		if prefix != "" && !strings.HasPrefix(rel, prefix) {
			return false
		}
		return strings.HasSuffix(rel, suffix)
	}
	// single * segment: use simple checks
	if strings.Contains(pat, "*") {
		// crude expansion: replace * with wildcard on a single path segment
		// Check parent dir prefix and filename suffix when pattern like dir/*/file
		segs := strings.Split(pat, "/")
		// If no slashes, fallback to suffix
		if len(segs) == 1 {
			return strings.HasSuffix(rel, segs[0])
		}
		// Check prefix up to first * and final suffix
		pre := []string{}
		post := []string{}
		starSeen := false
		for _, s := range segs {
			if strings.Contains(s, "*") {
				starSeen = true
				continue
			}
			if !starSeen {
				pre = append(pre, s)
			} else {
				post = append(post, s)
			}
		}
		preStr := strings.Join(pre, "/")
		postStr := strings.Join(post, "/")
		if preStr != "" && !strings.HasPrefix(rel, preStr) {
			return false
		}
		if postStr != "" && !strings.HasSuffix(rel, postStr) {
			return false
		}
		return true
	}
	// relative exact match
	if rel == pat {
		return true
	}
	// fallback suffix
	return strings.HasSuffix(p, pat) || strings.HasSuffix(rel, pat)
}

// RegisterBuiltins registers built-in conventions.
// Intentionally left empty: Riglet should not ship default conventions.
// Users must provide conventions via riglet.yaml or a convention package.
func RegisterBuiltins(r *Registry) {}
