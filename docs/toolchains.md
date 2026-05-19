<!-- Generated with Stardoc: http://skydoc.bazel.build -->

Toolchain rules for rules_jsonschema codegen.

`jsonschema_codegen_toolchain` wraps a single codegen executable
(`schema_to_rust`, `schema_to_starlark`, `schema_to_go`, …) as a
Bazel toolchain. The matching `toolchain_type` lives in
`//jsonschema:BUILD.bazel` — one type per output language so a
consumer can independently swap, say, the Rust generator without
touching the Starlark or Go ones.

Default toolchains are registered in `//rust:BUILD.bazel`,
`//starlark:BUILD.bazel`, `//go:BUILD.bazel`. To swap an
implementation, declare your own `jsonschema_codegen_toolchain` and
`register_toolchains(...)` it ahead of rules_jsonschema's default in
your `MODULE.bazel`.

<a id="jsonschema_codegen_toolchain"></a>

## jsonschema_codegen_toolchain

<pre>
load("@rules_jsonschema//jsonschema:toolchains.bzl", "jsonschema_codegen_toolchain")

jsonschema_codegen_toolchain(<a href="#jsonschema_codegen_toolchain-name">name</a>, <a href="#jsonschema_codegen_toolchain-binary">binary</a>)
</pre>

Declare a schema → code codegen executable as a Bazel toolchain.

**ATTRIBUTES**


| Name  | Description | Type | Mandatory | Default |
| :------------- | :------------- | :------------- | :------------- | :------------- |
| <a id="jsonschema_codegen_toolchain-name"></a>name |  A unique name for this target.   | <a href="https://bazel.build/concepts/labels#target-names">Name</a> | required |  |
| <a id="jsonschema_codegen_toolchain-binary"></a>binary |  The codegen executable for this toolchain. Must accept `--schema PATH --out PATH` plus any language-specific flags.   | <a href="https://bazel.build/concepts/labels">Label</a> | required |  |


