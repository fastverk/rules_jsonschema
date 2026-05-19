// Smoke test for the typify-via-Bazel pipeline. typify emits one top-
// level type per definition; for our person schema that's `Person` (the
// root object) and `Address` (under $defs).

use person_types::*;

#[test]
fn decodes_valid_input() {
    let raw = r#"{
        "name": "Ada Lovelace",
        "age": 36,
        "favourite_colour": "blue",
        "addresses": [
            {"line1": "1 Computing Lane", "country": "GB"}
        ]
    }"#;
    let p: Person = serde_json::from_str(raw).expect("valid decode");
    assert_eq!(p.name, "Ada Lovelace");
    // typify picked u64 for age because the schema sets minimum: 0.
    assert_eq!(p.age, Some(36u64));
    // Arrays default to empty when absent, so the field is Vec<_>, not Option.
    assert_eq!(p.addresses.len(), 1);
    assert_eq!(p.addresses[0].country, "GB");
}

#[test]
fn rejects_unknown_field() {
    // `additionalProperties: false` on the schema → typify emits
    // `#[serde(deny_unknown_fields)]` → serde_json rejects unknowns.
    // This is the build-time drift gate for typed schemas.
    let raw = r#"{"name":"X","nonexistent":1}"#;
    let err = serde_json::from_str::<Person>(raw).unwrap_err();
    assert!(
        err.to_string().contains("unknown field"),
        "expected unknown-field error, got: {err}"
    );
}

#[test]
fn rejects_invalid_enum_variant() {
    let raw = r#"{"name":"X","favourite_colour":"purple"}"#;
    serde_json::from_str::<Person>(raw)
        .err()
        .expect("purple is not in the schema's enum, decode must fail");
}
