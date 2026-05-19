<!-- Generated with Stardoc: http://skydoc.bazel.build -->

Helpers used by `schema_to_starlark`-generated rule code.

Kept in a separate file (rather than inlined per generated `.bzl`) so
the codegen output stays small and any helper fix benefits every
consumer at once. Generated `.bzl` files load from this module:

    load("@rules_jsonschema//runtime:helpers.bzl", "strip_empty", "parse_json_or_none")

<a id="parse_json_or_none"></a>

## parse_json_or_none

<pre>
load("@rules_jsonschema//runtime:helpers.bzl", "parse_json_or_none")

parse_json_or_none(<a href="#parse_json_or_none-s">s</a>)
</pre>

Return `None` for empty input, otherwise `json.decode(s)`.

Used for typed schema attrs whose value is a structured object
or array. Generated rule callers pass `json.encode({...})` (or
leave the attr empty); the generated impl invokes this to expand
the encoded payload back into a Starlark dict/list that gets
merged into the shard.

**PARAMETERS**


| Name  | Description | Default Value |
| :------------- | :------------- | :------------- |
| <a id="parse_json_or_none-s"></a>s |  <p align="center"> - </p>   |  none |


<a id="strip_empty"></a>

## strip_empty

<pre>
load("@rules_jsonschema//runtime:helpers.bzl", "strip_empty")

strip_empty(<a href="#strip_empty-d">d</a>)
</pre>

Drop dict entries whose values are absent / zero / empty.

Matches the JSON `omitempty` convention so generated shards stay
terse — Bazel `attr.*` zero values (0, False, "", [], {}) shouldn't
serialise as explicit overrides. Distinguishing "user set to 0"
from "user didn't set" isn't possible at the Starlark layer, so
we conflate them: every typed schema field that wants to mean
something non-default ships a non-zero/-empty value.

**PARAMETERS**


| Name  | Description | Default Value |
| :------------- | :------------- | :------------- |
| <a id="strip_empty-d"></a>d |  <p align="center"> - </p>   |  none |


