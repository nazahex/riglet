package engine

import (
	"io/fs"
	"path/filepath"
	"strings"
)

// IgnoreMatcher decides whether a path should be skipped.
type IgnoreMatcher interface {
	ShouldSkip(rel string, isDir bool) bool
}

// WalkFiles walks files under root and calls fn for each file path.
func WalkFiles(root string, fn func(string) error) error {
	return WalkFilesWithIgnore(root, nil, fn)
}

// WalkFilesWithIgnore walks files and uses the ignore matcher to skip.
func WalkFilesWithIgnore(root string, ign IgnoreMatcher, fn func(string) error) error {
	rr := filepath.ToSlash(filepath.Clean(root))
	return filepath.WalkDir(root, func(path string, d fs.DirEntry, err error) error {
		if err != nil {
			return err
		}
		base := filepath.ToSlash(filepath.Clean(path))
		// Only consider paths under rr
		if !(base == rr || strings.HasPrefix(base, rr+"/")) {
			return nil
		}
		rel := strings.TrimPrefix(base, rr+"/")
		// Always skip common heavy dirs
		if d.IsDir() {
			if d.Name() == "node_modules" || d.Name() == ".git" || d.Name() == "dist" || d.Name() == ".turbo" || d.Name() == ".artifacts" {
				return fs.SkipDir
			}
			if ign != nil && ign.ShouldSkip(rel, true) {
				return fs.SkipDir
			}
			return nil
		}
		if ign != nil && ign.ShouldSkip(rel, false) {
			return nil
		}
		return fn(path)
	})
}
