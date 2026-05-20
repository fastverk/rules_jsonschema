<!-- Generated with Stardoc: http://skydoc.bazel.build -->

Starlark user-facing rule for rules_jsonschema.

`jsonschema_starlark_codegen` emits typed Bazel `rule()` definitions
from a JSON Schema:

  1. Resolves the `starlark_codegen_toolchain_type` toolchain.
  2. Runs the toolchain's binary on the schema, producing a `.bzl`.

The default toolchain (registered by rules_jsonschema's MODULE.bazel)
points at the in-repo `schema_to_starlark` binary. Swap by declaring
your own `jsonschema_codegen_toolchain` and registering it ahead of
the default.

The output is meant to be committed in the consumer repo; pair with a
`diff_test` to catch drift (re-runs codegen on every CI build and
asserts the committed `.bzl` matches what the toolchain emits).

<a id="jsonschema_starlark_codegen"></a>

## jsonschema_starlark_codegen

<pre>
load("@rules_jsonschema//starlark:defs.bzl", "jsonschema_starlark_codegen")

jsonschema_starlark_codegen(<a href="#jsonschema_starlark_codegen-name">name</a>, <a href="#jsonschema_starlark_codegen-schema">schema</a>, <a href="#jsonschema_starlark_codegen-kinds">kinds</a>, <a href="#jsonschema_starlark_codegen-extra_args">extra_args</a>, <a href="#jsonschema_starlark_codegen-kwargs">**kwargs</a>)
</pre>

Generate a `.bzl` of typed rules from a JSON Schema.

**PARAMETERS**


| Name  | Description | Default Value |
| :------------- | :------------- | :------------- |
| <a id="jsonschema_starlark_codegen-name"></a>name |  target name; output file is `<name>.bzl`.   |  none |
| <a id="jsonschema_starlark_codegen-schema"></a>schema |  label of a `.json` schema document.   |  none |
| <a id="jsonschema_starlark_codegen-kinds"></a>kinds |  list of `(id, pointer, rule_name, provider_name)` 4-tuples. - `id`: short tag used in generated symbol names + the   rule-name attr (e.g. `service`). - `pointer`: JSON-pointer into the schema for the definition   whose `properties` become attrs (e.g. `#/definitions/service`). - `rule_name`: the public Starlark symbol the emitted rule   binds to. - `provider_name`: the public Starlark symbol the rule's   companion provider binds to. Optional — if omitted, `extra_args` typically enables the plugin's auto-kinds derivation (e.g. `--kinds-pointer-base=...` for the default `schema_to_starlark` toolchain). Leaving both empty produces a preamble-only `.bzl` (legal but rarely useful).   |  `None` |
| <a id="jsonschema_starlark_codegen-extra_args"></a>extra_args |  extra `--key=value` flags appended to the plugin's argv. Use to set plugin-specific options without registering a new toolchain.   |  `None` |
| <a id="jsonschema_starlark_codegen-kwargs"></a>kwargs |  forwarded to the underlying rule (visibility, etc.).   |  none |


