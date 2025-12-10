package pkgjson

import (
	"bytes"
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"sort"
	"strings"
)

// Options for formatting package.json
type Options struct {
	IsRoot      bool
	TopSections [][]string
	SubOrders   map[string][]string
}

// Known top-level key sections for deterministic output
// Line breaks are inserted between these sections when adjacent sections are present.
var orderSections = [][]string{
	{
		"name", "version", "description", "license", "private", "homepage", "repository", "bugs", "author", "keywords",
	},
	{
		"scripts", "workspaces", "dependencies", "devDependencies", "peerDependencies", "optionalDependencies",
	},
	{
		"packageManager", "engines", "os", "cpu",
	},
	{
		"bin", "main", "module", "types", "exports", "files", "sideEffects",
	},
	{
		"publishConfig",
	},
}

// Flattened order (kept for compatibility and tests)
var order = func() []string {
	var flat []string
	for _, sec := range orderSections {
		flat = append(flat, sec...)
	}
	return flat
}()

// Custom subfield orders for specific top-level object fields.
// Keys listed here are emitted in this exact order; remaining keys
// are appended in lexicographic order for stability.
var customSubOrders = map[string][]string{
	// Per request: author should be name, email, url
	"author": {"name", "email", "url"},
	// Best practice for repository
	"repository": {"type", "url", "directory"},
	// Best practice for publishConfig
	"publishConfig": {"access", "provenance"},
}

// applySpecificSubOrders applies preferred sub-orders from provided map
func applySpecificSubOrders(top map[string]any, orders map[string][]string) {
	for field, ord := range orders {
		if v, ok := top[field]; ok {
			if mm, ok := v.(map[string]any); ok {
				top[field] = orderObjectWithPreferred(mm, ord)
			}
		}
	}
}

// OrderKeys returns the canonical top-level key order used by Riglet.
func OrderKeys() []string { return append([]string(nil), order...) }

// FormatFile reads a package.json, normalizes ordering and nested maps, and returns formatted bytes.
func FormatFile(path string, opts Options) ([]byte, error) {
	b, err := os.ReadFile(path)
	if err != nil {
		return nil, err
	}
	var m map[string]any
	if err := json.Unmarshal(b, &m); err != nil {
		return nil, fmt.Errorf("invalid JSON: %w", err)
	}
	// Preserve original order for scripts by reading keys from raw JSON
	if v, ok := m["scripts"]; ok {
		if mm, ok := v.(map[string]any); ok {
			if ordered := orderedFieldKeys(b, "scripts"); len(ordered) > 0 {
				kvs := make([]kv, 0, len(mm))
				for _, k := range ordered {
					if val, exists := mm[k]; exists {
						kvs = append(kvs, kv{key: k, val: normalizeValue(val)})
					}
				}
				// append any remaining keys not found (edge cases)
				for k, val := range mm {
					found := false
					for _, okk := range ordered {
						if okk == k {
							found = true
							break
						}
					}
					if !found {
						kvs = append(kvs, kv{key: k, val: normalizeValue(val)})
					}
				}
				m["scripts"] = kvs
			}
		}
	}
	// sort nested maps that should be alpha by key
	sortNested(m, "dependencies")
	sortNested(m, "devDependencies")
	sortNested(m, "peerDependencies")
	sortNested(m, "optionalDependencies")
	// engines alphabetical (node, bun, etc.)
	sortNested(m, "engines")

	// apply custom subfield orders for known object fields
	if len(opts.SubOrders) > 0 {
		applySpecificSubOrders(m, opts.SubOrders)
	} else {
		applyCustomSubOrders(m)
	}

	var ordered []kv
	if len(opts.TopSections) > 0 {
		ordered = orderTopLevelWithSections(m, opts.TopSections)
	} else {
		ordered = orderTopLevel(m, order)
	}
	// encode deterministically with pretty formatting
	out := &bytes.Buffer{}
	indent := detectIndent(path)
	encodeJSONPretty(out, ordered, 0, indent)
	out.WriteByte('\n')
	return out.Bytes(), nil
}

// sortNested sorts map[string]any field keys lexicographically if present
func sortNested(m map[string]any, field string) {
	if v, ok := m[field]; ok {
		if mm, ok := v.(map[string]any); ok {
			m[field] = sortMap(mm)
		}
	}
}

// orderTopLevel constructs an ordered map representation following the preferred order,
// with any unknown keys appended at the end in lexicographic order.
func orderTopLevel(m map[string]any, _ []string) []kv {
	seen := map[string]bool{}
	var res []kv
	// Build by sections and insert separators between non-empty sections
	hasAhead := func(start int) bool {
		for i := start; i < len(orderSections); i++ {
			for _, k := range orderSections[i] {
				if _, ok := m[k]; ok {
					return true
				}
			}
		}
		return false
	}
	for i, sec := range orderSections {
		added := 0
		for _, k := range sec {
			if v, ok := m[k]; ok {
				res = append(res, kv{key: k, val: normalizeValue(v)})
				seen[k] = true
				added++
			}
		}
		if added > 0 && hasAhead(i+1) {
			res = append(res, kv{sep: true})
		}
	}
	// append the rest sorted
	var rest []string
	for k := range m {
		if !seen[k] {
			rest = append(rest, k)
		}
	}
	sort.Strings(rest)
	for _, k := range rest {
		res = append(res, kv{key: k, val: normalizeValue(m[k])})
	}
	return res
}

// orderTopLevelWithSections mirrors orderTopLevel but uses provided sections.
func orderTopLevelWithSections(m map[string]any, sections [][]string) []kv {
	seen := map[string]bool{}
	var res []kv
	hasAhead := func(start int) bool {
		for i := start; i < len(sections); i++ {
			for _, k := range sections[i] {
				if _, ok := m[k]; ok {
					return true
				}
			}
		}
		return false
	}
	for i, sec := range sections {
		added := 0
		for _, k := range sec {
			if v, ok := m[k]; ok {
				res = append(res, kv{key: k, val: normalizeValue(v)})
				seen[k] = true
				added++
			}
		}
		if added > 0 && hasAhead(i+1) {
			res = append(res, kv{sep: true})
		}
	}
	// append the rest sorted
	var rest []string
	for k := range m {
		if !seen[k] {
			rest = append(rest, k)
		}
	}
	sort.Strings(rest)
	for _, k := range rest {
		res = append(res, kv{key: k, val: normalizeValue(m[k])})
	}
	return res
}

// sortMap returns an ordered representation of a map with keys sorted
func sortMap(m map[string]any) []kv {
	keys := make([]string, 0, len(m))
	for k := range m {
		keys = append(keys, k)
	}
	sort.Strings(keys)
	res := make([]kv, 0, len(keys))
	for _, k := range keys {
		res = append(res, kv{key: k, val: normalizeValue(m[k])})
	}
	return res
}

// orderObjectWithPreferred returns []kv where keys in `preferred` appear first
// in the given order, followed by any remaining keys sorted lexicographically.
func orderObjectWithPreferred(m map[string]any, preferred []string) []kv {
	seen := map[string]bool{}
	res := make([]kv, 0, len(m))
	for _, k := range preferred {
		if v, ok := m[k]; ok {
			res = append(res, kv{key: k, val: normalizeValue(v)})
			seen[k] = true
		}
	}
	rest := make([]string, 0, len(m))
	for k := range m {
		if !seen[k] {
			rest = append(rest, k)
		}
	}
	sort.Strings(rest)
	for _, k := range rest {
		res = append(res, kv{key: k, val: normalizeValue(m[k])})
	}
	return res
}

// applyCustomSubOrders applies default preferred sub-orders to known fields
func applyCustomSubOrders(top map[string]any) {
	for field, ord := range customSubOrders {
		if v, ok := top[field]; ok {
			if mm, ok := v.(map[string]any); ok {
				top[field] = orderObjectWithPreferred(mm, ord)
			}
		}
	}
}

// normalizeValue ensures nested maps are encoded deterministically
func normalizeValue(v any) any {
	switch vv := v.(type) {
	case map[string]any:
		// Do not sort generic maps to preserve user-defined order (e.g., scripts).
		return vv
	case []any:
		// normalize nested elements recursively
		arr := make([]any, len(vv))
		for i := range vv {
			arr[i] = normalizeValue(vv[i])
		}
		return arr
	default:
		return v
	}
}

type kv struct {
	key string
	val any
	sep bool // if true, emit a blank line between object members (top-level only)
}

func encodeJSONString(buf *bytes.Buffer, s string) {
	buf.WriteByte('"')
	// custom escape that preserves <, >, & (avoids \u003c etc)
	for i := 0; i < len(s); i++ {
		ch := s[i]
		switch ch {
		case '\\':
			buf.WriteString("\\\\")
		case '"':
			buf.WriteString("\\\"")
		case '\n':
			buf.WriteString("\\n")
		case '\r':
			buf.WriteString("\\r")
		case '\t':
			buf.WriteString("\\t")
		case '\b':
			buf.WriteString("\\b")
		case '\f':
			buf.WriteString("\\f")
		default:
			// control chars < 0x20 must be escaped
			if ch < 0x20 {
				// \u00XX
				hex := "0123456789abcdef"
				buf.WriteString("\\u00")
				buf.WriteByte(hex[ch>>4])
				buf.WriteByte(hex[ch&0xF])
			} else {
				buf.WriteByte(ch)
			}
		}
	}
	buf.WriteByte('"')
}

// IsRootByPath determines if a package.json path is the repository root one.
func IsRootByPath(path, repoRoot string) bool {
	p := filepath.ToSlash(filepath.Clean(path))
	rr := filepath.ToSlash(filepath.Clean(repoRoot))
	return p == rr+"/package.json"
}

// encodeJSONPretty writes JSON with deterministic ordering and indentation.
func encodeJSONPretty(buf *bytes.Buffer, v any, level int, indent string) {
	switch vv := v.(type) {
	case []kv:
		// Pretty-print object represented as ordered key/value pairs (with optional separators)
		// Count real entries (non-separators)
		realTotal := 0
		for _, e := range vv {
			if !e.sep {
				realTotal++
			}
		}
		buf.WriteByte('{')
		if realTotal > 0 {
			buf.WriteByte('\n')
			realRemaining := realTotal
			printed := 0
			addExtra := false
			for _, e := range vv {
				if e.sep {
					if printed > 0 && realRemaining > 0 {
						addExtra = true
					}
					continue
				}
				if addExtra {
					buf.WriteByte('\n')
					addExtra = false
				}
				writeIndent(buf, level+1, indent)
				encodeJSONString(buf, e.key)
				buf.WriteString(": ")
				encodeJSONPretty(buf, e.val, level+1, indent)
				realRemaining--
				if realRemaining > 0 {
					buf.WriteByte(',')
				}
				buf.WriteByte('\n')
				printed++
			}
			writeIndent(buf, level, indent)
		}
		buf.WriteByte('}')
	case []any:
		buf.WriteByte('[')
		if len(vv) > 0 {
			buf.WriteByte('\n')
			for i, it := range vv {
				writeIndent(buf, level+1, indent)
				encodeJSONPretty(buf, it, level+1, indent)
				if i < len(vv)-1 {
					buf.WriteByte(',')
				}
				buf.WriteByte('\n')
			}
			writeIndent(buf, level, indent)
		}
		buf.WriteByte(']')
	case map[string]any:
		// Encode generic maps deterministically with pretty indentation
		kvs := sortMap(vv)
		encodeJSONPretty(buf, kvs, level, indent)
	case string:
		encodeJSONString(buf, vv)
	case float64, float32, int, int64, int32, int16, int8, uint, uint64, uint32, uint16, uint8:
		fmt.Fprintf(buf, "%v", vv)
	case bool:
		if vv {
			buf.WriteString("true")
		} else {
			buf.WriteString("false")
		}
	case nil:
		buf.WriteString("null")
	default:
		// fallback for primitive or uncommon types
		b, _ := json.Marshal(vv)
		buf.Write(b)
	}
}

func writeIndent(buf *bytes.Buffer, level int, indent string) {
	for i := 0; i < level; i++ {
		buf.WriteString(indent)
	}
}

// orderedFieldKeys scans raw JSON and returns the key order for a top-level object field.
func orderedFieldKeys(raw []byte, field string) []string {
	dec := json.NewDecoder(bytes.NewReader(raw))
	// Top-level must be an object
	// Read start token
	tok, err := dec.Token()
	if err != nil {
		return nil
	}
	if delim, ok := tok.(json.Delim); !ok || delim != '{' {
		return nil
	}
	for dec.More() {
		// Read key
		ktok, err := dec.Token()
		if err != nil {
			return nil
		}
		key, ok := ktok.(string)
		if !ok {
			return nil
		}
		// If this is the target field
		if key == field {
			// Next should be an object
			tok, err := dec.Token()
			if err != nil {
				return nil
			}
			if delim, ok := tok.(json.Delim); !ok || delim != '{' {
				return nil
			}
			var keys []string
			for dec.More() {
				kt, err := dec.Token()
				if err != nil {
					return keys
				}
				kstr, ok := kt.(string)
				if !ok {
					return keys
				}
				keys = append(keys, kstr)
				// skip value
				if err := skipValue(dec); err != nil {
					return keys
				}
			}
			// consume closing '}' of scripts
			_, _ = dec.Token()
			return keys
		}
		// skip value of other field
		if err := skipValue(dec); err != nil {
			return nil
		}
	}
	return nil
}

// skipValue consumes the next JSON value from decoder
func skipValue(dec *json.Decoder) error {
	tok, err := dec.Token()
	if err != nil {
		return err
	}
	switch t := tok.(type) {
	case json.Delim:
		// arrays or objects: consume nested contents
		switch t {
		case '{':
			for dec.More() {
				// key
				if _, err := dec.Token(); err != nil {
					return err
				}
				if err := skipValue(dec); err != nil {
					return err
				}
			}
			// closing
			_, _ = dec.Token()
		case '[':
			for dec.More() {
				if err := skipValue(dec); err != nil {
					return err
				}
			}
			_, _ = dec.Token()
		}
	default:
		// primitives already consumed
	}
	return nil
}

// detectIndent decides indentation based on formatter configs in the repo tree.
// Priority: Biome > Prettier > EditorConfig > fallback 4 spaces.
func detectIndent(startPath string) string {
	dir := filepath.Dir(startPath)
	for {
		// Biome config typical names
		if exists(filepath.Join(dir, "biome.json")) || exists(filepath.Join(dir, "biome.jsonc")) || exists(filepath.Join(dir, ".biome.json")) || exists(filepath.Join(dir, ".biome.jsonc")) {
			return "  "
		}
		// Prettier config typical names
		if exists(filepath.Join(dir, ".prettierrc")) || exists(filepath.Join(dir, ".prettierrc.json")) || exists(filepath.Join(dir, ".prettierrc.yaml")) || exists(filepath.Join(dir, "prettier.config.js")) || exists(filepath.Join(dir, "prettier.config.cjs")) || exists(filepath.Join(dir, "prettier.config.mjs")) || exists(filepath.Join(dir, "prettier.config.ts")) {
			return "  "
		}
		// EditorConfig: try to parse indent_size for *.json (very light read)
		if exists(filepath.Join(dir, ".editorconfig")) {
			sz := editorConfigIndentSize(filepath.Join(dir, ".editorconfig"))
			if sz > 0 {
				return strings.Repeat(" ", sz)
			}
		}
		parent := filepath.Dir(dir)
		if parent == dir {
			break
		}
		dir = parent
	}
	return "    " // 4 spaces fallback
}

func exists(p string) bool {
	if fi, err := os.Stat(p); err == nil && !fi.IsDir() {
		return true
	}
	return false
}

// Very small EditorConfig parser: looks for section matching *.json and reads indent_size.
func editorConfigIndentSize(path string) int {
	b, err := os.ReadFile(path)
	if err != nil {
		return 0
	}
	lines := strings.Split(string(b), "\n")
	match := false
	size := 0
	for _, ln := range lines {
		l := strings.TrimSpace(ln)
		if len(l) == 0 || strings.HasPrefix(l, ";") || strings.HasPrefix(l, "#") {
			continue
		}
		if strings.HasPrefix(l, "[") && strings.HasSuffix(l, "]") {
			sect := strings.TrimSuffix(strings.TrimPrefix(l, "["), "]")
			// naive glob check
			if sect == "*.json" || sect == "*" || strings.Contains(sect, "json") {
				match = true
			} else {
				match = false
			}
			continue
		}
		if match && strings.HasPrefix(l, "indent_size") {
			parts := strings.SplitN(l, "=", 2)
			if len(parts) == 2 {
				v := strings.TrimSpace(parts[1])
				// numeric only
				n := 0
				for i := 0; i < len(v); i++ {
					if v[i] < '0' || v[i] > '9' {
						n = 0
						break
					}
					n = n*10 + int(v[i]-'0')
				}
				if n > 0 {
					size = n
					break
				}
			}
		}
	}
	return size
}
