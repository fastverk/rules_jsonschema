// Smoke test for the Go side of the plugin contract: schema_to_go
// emits one Go struct per `$defs` entry plus a top-level type from
// the schema's `title`. This exercises the decode path against the
// same fixture used in roundtrip_test.rs.
package person_go_types

import (
	"encoding/json"
	"testing"
)

func TestDecodesValidInput(t *testing.T) {
	raw := []byte(`{
		"name": "Ada Lovelace",
		"age": 36,
		"favourite_colour": "blue",
		"addresses": [
			{"line1": "1 Computing Lane", "country": "GB"}
		]
	}`)
	var p Person
	if err := json.Unmarshal(raw, &p); err != nil {
		t.Fatalf("decode failed: %v", err)
	}
	if p.Name != "Ada Lovelace" {
		t.Errorf("Name = %q want Ada Lovelace", p.Name)
	}
	if p.Age == nil || *p.Age != 36 {
		t.Errorf("Age = %v want 36", p.Age)
	}
	if len(p.Addresses) != 1 {
		t.Fatalf("Addresses len = %d want 1", len(p.Addresses))
	}
	if p.Addresses[0].Country != "GB" {
		t.Errorf("Addresses[0].Country = %q want GB", p.Addresses[0].Country)
	}
}

func TestOptionalFieldsAbsent(t *testing.T) {
	raw := []byte(`{"name": "Solo"}`)
	var p Person
	if err := json.Unmarshal(raw, &p); err != nil {
		t.Fatalf("decode failed: %v", err)
	}
	if p.Age != nil {
		t.Errorf("Age should be nil when absent, got %v", p.Age)
	}
	if p.FavouriteColour != nil {
		t.Errorf("FavouriteColour should be nil when absent, got %v", p.FavouriteColour)
	}
}
