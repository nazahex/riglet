package engine

import (
	"fmt"

	"cuelang.org/go/cue"
	"cuelang.org/go/cue/cuecontext"
	"cuelang.org/go/cue/load"
	yaml "gopkg.in/yaml.v3"
)

type Schema struct {
	ctx *cue.Context
	v   cue.Value
}

// LoadSchema compiles CUE files into a schema.
func LoadSchema(paths []string) (*Schema, error) {
	ctx := cuecontext.New()
	insts := load.Instances(paths, nil)
	if len(insts) == 0 {
		return nil, fmt.Errorf("no CUE instances loaded")
	}
	v := ctx.BuildInstance(insts[0])
	if err := v.Validate(); err != nil {
		return nil, fmt.Errorf("schema invalid: %w", err)
	}
	return &Schema{ctx: ctx, v: v}, nil
}

// Validate validates a decoded Go value against the schema.
func (s *Schema) Validate(val any) error {
	enc := s.ctx.Encode(val)
	u := s.v.Unify(enc)
	if err := u.Validate(); err != nil {
		return fmt.Errorf("validation error: %w", err)
	}
	return nil
}

// DecodeYAML decodes YAML into a generic Go value.
func DecodeYAML(b []byte) (any, error) {
	var v any
	if err := yaml.Unmarshal(b, &v); err != nil {
		return nil, err
	}
	return v, nil
}
