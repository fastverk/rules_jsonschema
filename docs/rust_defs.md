<!-- Generated with Stardoc: http://skydoc.bazel.build -->

Rust user-facing rules for rules_jsonschema.

`jsonschema_rust_library` is the Rust-specific shape of the
schema → code pipeline:

  1. Resolves the `rust_codegen_toolchain_type` toolchain.
  2. Runs the toolchain's binary on the schema, producing a `.rs`.
  3. Wraps the `.rs` in a `rust_library` with serde / serde_json /
     regress threaded as direct deps.

The default toolchain (registered by rules_jsonschema's MODULE.bazel)
points at the in-repo typify-based `schema_to_rust` binary. Swap by
declaring your own `jsonschema_codegen_toolchain` + registering it
ahead of the default.

<a id="jsonschema_rust_library"></a>

## jsonschema_rust_library

<pre>
load("@rules_jsonschema//rust:defs.bzl", "jsonschema_rust_library")

jsonschema_rust_library(<a href="#jsonschema_rust_library-name">name</a>, <a href="#jsonschema_rust_library-schema">schema</a>, <a href="#jsonschema_rust_library-extra_args">extra_args</a>, <a href="#jsonschema_rust_library-serde">serde</a>, <a href="#jsonschema_rust_library-serde_json">serde_json</a>, <a href="#jsonschema_rust_library-regress">regress</a>, <a href="#jsonschema_rust_library-visibility">visibility</a>,
                        <a href="#jsonschema_rust_library-rust_library_kwargs">**rust_library_kwargs</a>)
</pre>

Generate a rust_library of typed schema bindings.

The emitted library exports one Rust struct/enum per top-level
JSON-Schema definition, with `#[derive(Serialize, Deserialize)]`
plus `#[serde(deny_unknown_fields)]` wherever the source schema
sets `additionalProperties: false`.


**PARAMETERS**


| Name  | Description | Default Value |
| :------------- | :------------- | :------------- |
| <a id="jsonschema_rust_library-name"></a>name |  rust_library target name. Consumers add this to `deps`.   |  none |
| <a id="jsonschema_rust_library-schema"></a>schema |  label of a `.json` schema file.   |  none |
| <a id="jsonschema_rust_library-extra_args"></a>extra_args |  extra `--key=value` flags appended to the plugin's argv. Use to set plugin-specific options without registering a new toolchain. The default plugin (schema_to_rust) accepts no extra flags today; consumers of custom toolchains will.   |  `None` |
| <a id="jsonschema_rust_library-serde"></a>serde |  label of the `serde` crate to use as a direct dep. Defaults to rules_jsonschema's own `@crates//:serde`. **Consumers whose binary also depends on serde must point this at their own crate repo**, otherwise the generated types' trait impls live in a different compile unit than the consumer's and Rust treats them as distinct types (`error[E0277]: the trait bound Service: serde::Serialize is not satisfied`).   |  `None` |
| <a id="jsonschema_rust_library-serde_json"></a>serde_json |  same story for `serde_json`.   |  `None` |
| <a id="jsonschema_rust_library-regress"></a>regress |  same story for `regress` (typify uses it for `pattern`-validated string newtypes).   |  `None` |
| <a id="jsonschema_rust_library-visibility"></a>visibility |  forwarded to rust_library.   |  `None` |
| <a id="jsonschema_rust_library-rust_library_kwargs"></a>rust_library_kwargs |  forwarded to rust_library (e.g. extra `deps`).   |  none |


