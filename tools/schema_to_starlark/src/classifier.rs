//! Schema-property → Bazel attr classification.
//!
//! `property_to_attr` is the single entry point. It runs a small
//! ordered chain of classifier helpers; the first one that recognises
//! the schema shape wins. Each classifier is independently unit-tested
//! so adding a new pattern means writing one new helper + one new
//! test, not editing one big match.

use serde_json::Value;

/// The set of Bazel attr shapes we know how to emit. The `JsonString`
/// variant is the escape hatch: when a schema property doesn't fit
/// any other shape (nested objects, complex `oneOf`s, arrays of
/// objects), the generated attr takes a JSON-encoded string and the
/// consumer's Rust shard reader parses it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttrSpec {
    String,
    EnumString(Vec<String>),
    Int,
    Bool,
    StringList,
    StringDict,
    JsonString,
}

/// Classify a schema property into an `AttrSpec`. Returns
/// `AttrSpec::JsonString` as the fallback when no specific shape
/// matches — the Rust shard reader handles whichever form was
/// actually written.
pub fn property_to_attr(prop: &Value, ref_name: Option<&str>, root: &Value) -> AttrSpec {
    classify_named_ref(ref_name)
        .or_else(|| classify_one_of(prop, root))
        .or_else(|| classify_enum(prop))
        .or_else(|| classify_type_union(prop))
        .or_else(|| classify_single_type(prop, root))
        .unwrap_or(AttrSpec::JsonString)
}

/// `$ref` to one of compose-spec's well-known shared types
/// (`string_or_list`, `list_or_dict`, `command`, `list_of_strings`).
/// Without this, every `command:` / `environment:` / `ports:` attr
/// degrades to a JSON-encoded string and the typed-attr win is lost.
///
/// Generalises beyond compose-spec by name: other schemas using the
/// same conventional names get the same treatment.
pub fn classify_named_ref(ref_name: Option<&str>) -> Option<AttrSpec> {
    let r = ref_name?;
    let name = r.rsplit('/').next()?;
    match name {
        "list_of_strings" | "string_or_list" | "command" => Some(AttrSpec::StringList),
        // list_or_dict accepts either a list of "K=V" strings or a
        // dict. dict reads better ergonomically for Bazel callers
        // and the Rust shard reader accepts both forms anyway.
        "list_or_dict" => Some(AttrSpec::StringDict),
        _ => None,
    }
}

/// `oneOf` with at least one "list of strings" variant. Compose-spec
/// uses this for service.depends_on / service.networks — short form
/// is the common case, long form covers per-service condition
/// objects we don't model.
pub fn classify_one_of(prop: &Value, root: &Value) -> Option<AttrSpec> {
    let variants = prop.get("oneOf")?.as_array()?;
    if variants
        .iter()
        .map(|v| deref(v, root))
        .any(|v| is_string_list(v, root))
    {
        return Some(AttrSpec::StringList);
    }
    None
}

/// `enum: [...]` of strings → constrained string attr. Sets whose
/// values collide under case-insensitive identifier mangling
/// (compose-spec's selinux `["z", "Z"]`) degrade to a plain string
/// since Bazel's `attr.string(values=...)` rejects duplicates after
/// canonicalisation.
pub fn classify_enum(prop: &Value) -> Option<AttrSpec> {
    let arr = prop.get("enum")?.as_array()?;
    let values: Vec<String> = arr.iter().filter_map(|v| v.as_str().map(String::from)).collect();
    if values.len() != arr.len() {
        return None; // non-string enum entries
    }
    let mut lower = std::collections::HashSet::new();
    let collides = !values.iter().all(|v| lower.insert(v.to_lowercase()));
    if collides {
        Some(AttrSpec::String)
    } else {
        Some(AttrSpec::EnumString(values))
    }
}

/// `type: [...]` (multi-type union — e.g. compose-spec's
/// `external: ["boolean", "string", "object"]` or
/// `retries: ["number", "string"]`). Preference order is bool >
/// string > int: we pick `string` ahead of `int` in number|string
/// unions because compose-spec uses that pattern for human-readable
/// values with unit suffixes (`shm_size: "256m"`); pure numeric
/// values (`retries: 3`) round-trip through the string form too.
pub fn classify_type_union(prop: &Value) -> Option<AttrSpec> {
    let types = prop.get("type")?.as_array()?;
    let names: Vec<&str> = types.iter().filter_map(Value::as_str).collect();
    if names.contains(&"boolean") {
        return Some(AttrSpec::Bool);
    }
    if names.contains(&"string") {
        return Some(AttrSpec::String);
    }
    if names.contains(&"integer") || names.contains(&"number") {
        return Some(AttrSpec::Int);
    }
    Some(AttrSpec::JsonString)
}

/// `type: "..."` (single primitive or container). Returns None on
/// nested-object / `oneOf`-of-objects / array-of-object shapes that
/// don't compress into a typed Bazel attr — those flow to the
/// JsonString fallback in `property_to_attr`.
pub fn classify_single_type(prop: &Value, root: &Value) -> Option<AttrSpec> {
    match prop.get("type")?.as_str()? {
        "string" => Some(AttrSpec::String),
        "integer" | "number" => Some(AttrSpec::Int),
        "boolean" => Some(AttrSpec::Bool),
        "array" => classify_array(prop, root),
        "object" => classify_object(prop, root),
        _ => None,
    }
}

/// Arrays of strings → `string_list`. Arrays of `oneOf [string, ...]`
/// (compose-spec's ports/volumes/env_file short-form) also collapse to
/// `string_list` on the assumption that callers wanting the long-form
/// object syntax can pass a JSON wrapper or use a future flattening
/// rule. Everything else falls back to JsonString.
pub fn classify_array(prop: &Value, root: &Value) -> Option<AttrSpec> {
    let items = prop.get("items").map(|i| deref(i, root))?;
    if items.get("type").and_then(Value::as_str) == Some("string") {
        return Some(AttrSpec::StringList);
    }
    if let Some(variants) = items.get("oneOf").and_then(Value::as_array) {
        if variants
            .iter()
            .map(|v| deref(v, root))
            .any(|v| v.get("type").and_then(Value::as_str) == Some("string"))
        {
            return Some(AttrSpec::StringList);
        }
    }
    None
}

/// Objects whose value type is string-typed → `string_dict`. Compose-
/// spec uses both `additionalProperties: {type: "string"}` AND
/// `patternProperties: {"^.+$": {type: "string"}}` for this; we
/// accept both. Also accept value unions that *contain* "string"
/// (e.g. `["string", "number"]`).
pub fn classify_object(prop: &Value, root: &Value) -> Option<AttrSpec> {
    let value_node = prop
        .get("additionalProperties")
        .or_else(|| {
            prop.get("patternProperties").and_then(|pp| {
                pp.as_object().and_then(|m| m.values().next())
            })
        })
        .map(|v| deref(v, root))?;
    let kind = value_node.get("type")?;
    let is_stringy = match kind {
        Value::String(s) => s == "string",
        Value::Array(types) => types
            .iter()
            .filter_map(Value::as_str)
            .any(|t| t == "string"),
        _ => false,
    };
    is_stringy.then_some(AttrSpec::StringDict)
}

// --- schema helpers --------------------------------------------------

/// Resolve a JSON-pointer-ish string against the schema root. The
/// leading `#` and `/` are tolerated; segments are unescaped per
/// RFC 6901 (`~1` → `/`, `~0` → `~`). Returns None if any segment
/// doesn't resolve.
pub fn resolve_pointer<'a>(root: &'a Value, ptr: &str) -> Option<&'a Value> {
    let ptr = ptr.strip_prefix('#').unwrap_or(ptr);
    let ptr = ptr.strip_prefix('/').unwrap_or(ptr);
    if ptr.is_empty() {
        return Some(root);
    }
    let mut node = root;
    for seg in ptr.split('/') {
        let seg = seg.replace("~1", "/").replace("~0", "~");
        node = node.get(&seg)?;
    }
    Some(node)
}

/// Follow a single `$ref` if present. We don't recursively flatten;
/// the only need is to look at the immediate type of a property
/// described via `$ref: "#/$defs/foo"`.
pub fn deref<'a>(node: &'a Value, root: &'a Value) -> &'a Value {
    if let Some(r) = node.get("$ref").and_then(Value::as_str) {
        if let Some(target) = resolve_pointer(root, r) {
            return target;
        }
    }
    node
}

/// Whether `node` describes a list of strings — either:
///   - `type: array, items: {type: string}`
///   - `type: array, items: {$ref: "#/.../list_of_strings"}`
///   - `type: array, items: {oneOf: [{type: string}, ...]}` (we treat
///     this as string-list because the short-form is the string
///     variant; long-form callers can pass JSON via the future
///     wrapper rule once flattening is implemented)
pub fn is_string_list(node: &Value, root: &Value) -> bool {
    if node.get("type").and_then(Value::as_str) != Some("array") {
        return false;
    }
    let items = match node.get("items").map(|i| deref(i, root)) {
        Some(i) => i,
        None => return false,
    };
    if items.get("type").and_then(Value::as_str) == Some("string") {
        return true;
    }
    if let Some(variants) = items.get("oneOf").and_then(Value::as_array) {
        return variants.iter().any(|v| {
            let r = deref(v, root);
            r.get("type").and_then(Value::as_str) == Some("string")
        });
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // --- classify_named_ref ------------------------------------------

    #[test]
    fn named_ref_to_string_list() {
        assert_eq!(
            classify_named_ref(Some("#/definitions/string_or_list")),
            Some(AttrSpec::StringList)
        );
        assert_eq!(
            classify_named_ref(Some("#/definitions/list_of_strings")),
            Some(AttrSpec::StringList)
        );
        assert_eq!(
            classify_named_ref(Some("#/definitions/command")),
            Some(AttrSpec::StringList)
        );
    }

    #[test]
    fn named_ref_to_string_dict() {
        assert_eq!(
            classify_named_ref(Some("#/definitions/list_or_dict")),
            Some(AttrSpec::StringDict)
        );
    }

    #[test]
    fn named_ref_unknown_returns_none() {
        assert!(classify_named_ref(Some("#/definitions/something_custom")).is_none());
        assert!(classify_named_ref(None).is_none());
    }

    // --- classify_one_of ---------------------------------------------

    #[test]
    fn one_of_with_string_list_variant_picks_string_list() {
        let prop = json!({
            "oneOf": [
                {"type": "array", "items": {"type": "string"}},
                {"type": "object", "patternProperties": {"^[a-z]+$": {"type": "object"}}}
            ]
        });
        assert_eq!(
            classify_one_of(&prop, &Value::Null),
            Some(AttrSpec::StringList)
        );
    }

    #[test]
    fn one_of_without_string_list_returns_none() {
        let prop = json!({
            "oneOf": [{"type": "string"}, {"type": "object"}]
        });
        assert!(classify_one_of(&prop, &Value::Null).is_none());
    }

    // --- classify_enum -----------------------------------------------

    #[test]
    fn enum_of_strings_to_enum_string() {
        let prop = json!({"enum": ["always", "on-failure", "no"]});
        match classify_enum(&prop) {
            Some(AttrSpec::EnumString(v)) => assert_eq!(v, vec!["always", "on-failure", "no"]),
            other => panic!("got {other:?}"),
        }
    }

    #[test]
    fn case_colliding_enum_degrades_to_string() {
        // compose-spec's selinux `["z", "Z"]` — Bazel attr.string
        // values must be distinct under canonicalisation.
        let prop = json!({"enum": ["z", "Z"]});
        assert_eq!(classify_enum(&prop), Some(AttrSpec::String));
    }

    #[test]
    fn enum_with_non_string_entry_returns_none() {
        let prop = json!({"enum": ["a", 42]});
        assert!(classify_enum(&prop).is_none());
    }

    // --- classify_type_union -----------------------------------------

    #[test]
    fn union_with_boolean_wins() {
        let prop = json!({"type": ["boolean", "string", "object"]});
        assert_eq!(classify_type_union(&prop), Some(AttrSpec::Bool));
    }

    #[test]
    fn string_beats_int_in_union() {
        // shm_size: ["number", "string"] — caller writes "256m".
        let prop = json!({"type": ["number", "string"]});
        assert_eq!(classify_type_union(&prop), Some(AttrSpec::String));
    }

    #[test]
    fn int_only_union_picks_int() {
        let prop = json!({"type": ["integer", "number"]});
        assert_eq!(classify_type_union(&prop), Some(AttrSpec::Int));
    }

    // --- classify_single_type ----------------------------------------

    #[test]
    fn single_primitive_types() {
        let cases = [
            (json!({"type": "string"}), AttrSpec::String),
            (json!({"type": "integer"}), AttrSpec::Int),
            (json!({"type": "number"}), AttrSpec::Int),
            (json!({"type": "boolean"}), AttrSpec::Bool),
        ];
        for (prop, want) in cases {
            assert_eq!(classify_single_type(&prop, &Value::Null), Some(want));
        }
    }

    #[test]
    fn array_of_string_is_string_list() {
        let prop = json!({"type": "array", "items": {"type": "string"}});
        assert_eq!(
            classify_single_type(&prop, &Value::Null),
            Some(AttrSpec::StringList)
        );
    }

    #[test]
    fn array_of_one_of_with_string_variant_is_string_list() {
        // service.ports / service.volumes shape: list of [string, object].
        let prop = json!({
            "type": "array",
            "items": {"oneOf": [{"type": "string"}, {"type": "object"}]}
        });
        assert_eq!(
            classify_single_type(&prop, &Value::Null),
            Some(AttrSpec::StringList)
        );
    }

    #[test]
    fn array_of_objects_returns_none() {
        let prop = json!({"type": "array", "items": {"type": "object"}});
        assert!(classify_single_type(&prop, &Value::Null).is_none());
    }

    #[test]
    fn object_with_string_additional_properties_is_string_dict() {
        let prop = json!({
            "type": "object",
            "additionalProperties": {"type": "string"}
        });
        assert_eq!(
            classify_single_type(&prop, &Value::Null),
            Some(AttrSpec::StringDict)
        );
    }

    #[test]
    fn object_with_string_pattern_properties_is_string_dict() {
        let prop = json!({
            "type": "object",
            "patternProperties": {
                "^.+$": {"type": ["string", "number"]}
            }
        });
        assert_eq!(
            classify_single_type(&prop, &Value::Null),
            Some(AttrSpec::StringDict)
        );
    }

    #[test]
    fn object_with_no_string_values_returns_none() {
        let prop = json!({
            "type": "object",
            "additionalProperties": {"type": "integer"}
        });
        assert!(classify_single_type(&prop, &Value::Null).is_none());
    }

    // --- property_to_attr (full pipeline) ----------------------------

    #[test]
    fn property_to_attr_falls_back_to_json_string() {
        let prop = json!({"type": "array", "items": {"type": "object"}});
        assert_eq!(
            property_to_attr(&prop, None, &Value::Null),
            AttrSpec::JsonString
        );
    }

    #[test]
    fn property_to_attr_resolves_through_ref_name() {
        // A property authored as `{$ref: ".../list_or_dict",
        // description: "..."}` — the property itself has no `type`
        // but we recognise the ref name and emit string_dict.
        assert_eq!(
            property_to_attr(
                &Value::Object(serde_json::Map::new()),
                Some("#/definitions/list_or_dict"),
                &Value::Null,
            ),
            AttrSpec::StringDict
        );
    }

    // --- pointer / deref / is_string_list ----------------------------

    #[test]
    fn resolve_pointer_walks_nested_keys() {
        let root = json!({"definitions": {"foo": {"bar": 42}}});
        assert_eq!(
            resolve_pointer(&root, "#/definitions/foo/bar"),
            Some(&Value::from(42))
        );
    }

    #[test]
    fn resolve_pointer_root_returns_root() {
        let root = json!({"x": 1});
        assert_eq!(resolve_pointer(&root, "#"), Some(&root));
    }

    #[test]
    fn deref_follows_ref() {
        let root = json!({"definitions": {"x": {"type": "string"}}});
        let node = json!({"$ref": "#/definitions/x"});
        assert_eq!(deref(&node, &root).get("type").and_then(Value::as_str), Some("string"));
    }

    #[test]
    fn is_string_list_via_inline() {
        let n = json!({"type": "array", "items": {"type": "string"}});
        assert!(is_string_list(&n, &Value::Null));
    }

    #[test]
    fn is_string_list_via_ref() {
        let root = json!({"definitions": {"sl": {"type": "array", "items": {"type": "string"}}}});
        let n = json!({"$ref": "#/definitions/sl"});
        // is_string_list takes the *already-deref'd* node, but
        // callers pass the resolved one. Mimic that.
        assert!(is_string_list(deref(&n, &root), &root));
    }
}
