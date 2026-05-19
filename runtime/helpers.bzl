"""Helpers used by `schema_to_starlark`-generated rule code.

Kept in a separate file (rather than inlined per generated `.bzl`) so
the codegen output stays small and any helper fix benefits every
consumer at once. Generated `.bzl` files load from this module:

    load("@rules_jsonschema//runtime:helpers.bzl", "strip_empty", "parse_json_or_none")
"""

def strip_empty(d):
    """Drop dict entries whose values are absent / zero / empty.

    Matches the JSON `omitempty` convention so generated shards stay
    terse — Bazel `attr.*` zero values (0, False, "", [], {}) shouldn't
    serialise as explicit overrides. Distinguishing "user set to 0"
    from "user didn't set" isn't possible at the Starlark layer, so
    we conflate them: every typed schema field that wants to mean
    something non-default ships a non-zero/-empty value.
    """
    out = {}
    for k, v in d.items():
        if v == None:
            continue
        t = type(v)
        if t == "list" and len(v) == 0:
            continue
        if t == "dict" and len(v) == 0:
            continue
        if t == "string" and v == "":
            continue
        if t == "int" and v == 0:
            continue
        if t == "bool" and v == False:
            continue
        out[k] = v
    return out

def parse_json_or_none(s):
    """Return `None` for empty input, otherwise `json.decode(s)`.

    Used for typed schema attrs whose value is a structured object
    or array. Generated rule callers pass `json.encode({...})` (or
    leave the attr empty); the generated impl invokes this to expand
    the encoded payload back into a Starlark dict/list that gets
    merged into the shard.
    """
    if not s:
        return None
    return json.decode(s)
