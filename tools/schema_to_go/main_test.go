package main

import "testing"

func TestGoFieldName(t *testing.T) {
	cases := []struct{ in, want string }{
		{"name", "Name"},
		{"first_name", "FirstName"},
		{"favourite-colour", "FavouriteColour"},
		{"net.core.somaxconn", "NetCoreSomaxconn"},
		{"Already", "Already"},
		{"1st", "_1st"},
		{"", "_"},
	}
	for _, c := range cases {
		if got := goFieldName(c.in); got != c.want {
			t.Errorf("goFieldName(%q) = %q, want %q", c.in, got, c.want)
		}
	}
}

func TestSanitizePkg(t *testing.T) {
	cases := []struct{ in, want string }{
		{"person_types", "person_types"},
		{"Person-Types", "person_types"},
		{"123pkg", "_123pkg"},
		{"", "generated"},
	}
	for _, c := range cases {
		if got := sanitizePkg(c.in); got != c.want {
			t.Errorf("sanitizePkg(%q) = %q, want %q", c.in, got, c.want)
		}
	}
}

func TestBaseGoTypePrimitives(t *testing.T) {
	root := map[string]any{}
	cases := []struct {
		prop map[string]any
		want string
	}{
		{map[string]any{"type": "string"}, "string"},
		{map[string]any{"type": "integer"}, "int"},
		{map[string]any{"type": "number"}, "float64"},
		{map[string]any{"type": "boolean"}, "bool"},
	}
	for _, c := range cases {
		if got := baseGoType(c.prop, root); got != c.want {
			t.Errorf("baseGoType(%v) = %q, want %q", c.prop, got, c.want)
		}
	}
}

func TestBaseGoTypeArrayOfStrings(t *testing.T) {
	root := map[string]any{}
	prop := map[string]any{
		"type":  "array",
		"items": map[string]any{"type": "string"},
	}
	if got := baseGoType(prop, root); got != "[]string" {
		t.Errorf("got %q want []string", got)
	}
}

func TestBaseGoTypeMapWithStringValues(t *testing.T) {
	root := map[string]any{}
	prop := map[string]any{
		"type":                 "object",
		"additionalProperties": map[string]any{"type": "string"},
	}
	if got := baseGoType(prop, root); got != "map[string]string" {
		t.Errorf("got %q want map[string]string", got)
	}
}

func TestBaseGoTypeResolvesRef(t *testing.T) {
	root := map[string]any{}
	prop := map[string]any{"$ref": "#/$defs/address"}
	if got := baseGoType(prop, root); got != "Address" {
		t.Errorf("got %q want Address", got)
	}
}

func TestGoTypeForOptionalWrapsInPointer(t *testing.T) {
	root := map[string]any{}
	prop := map[string]any{"type": "string"}
	if got := goTypeFor(prop, root, false); got != "*string" {
		t.Errorf("optional string: got %q want *string", got)
	}
	if got := goTypeFor(prop, root, true); got != "string" {
		t.Errorf("required string: got %q want string", got)
	}
}

func TestGoTypeForDoesNotWrapSlicesOrMaps(t *testing.T) {
	root := map[string]any{}
	arr := map[string]any{"type": "array", "items": map[string]any{"type": "string"}}
	m := map[string]any{"type": "object", "additionalProperties": map[string]any{"type": "string"}}
	if got := goTypeFor(arr, root, false); got != "[]string" {
		t.Errorf("arr: got %q want []string", got)
	}
	if got := goTypeFor(m, root, false); got != "map[string]string" {
		t.Errorf("map: got %q want map[string]string", got)
	}
}

func TestUnionPrefersBoolThenStringThenInt(t *testing.T) {
	root := map[string]any{}
	cases := []struct {
		types []any
		want  string
	}{
		{[]any{"boolean", "string"}, "bool"},
		{[]any{"number", "string"}, "string"},
		{[]any{"integer", "number"}, "int"},
	}
	for _, c := range cases {
		prop := map[string]any{"type": c.types}
		if got := baseGoType(prop, root); got != c.want {
			t.Errorf("baseGoType(%v) = %q, want %q", c.types, got, c.want)
		}
	}
}
