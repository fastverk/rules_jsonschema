<!-- Generated with Stardoc: http://skydoc.bazel.build -->

Providers exposed by rules_jsonschema.

`JsonschemaCodegenToolchainInfo` is the contract every codegen
toolchain provides: a single `binary` File that implements the
schema → output-language conversion. Per-language user-facing rules
resolve a toolchain by type
(`@rules_jsonschema//jsonschema:<lang>_codegen_toolchain_type`),
fetch this provider, and run the binary.

Splitting it out from `defs.bzl` lets language modules (`//rust:`,
`//starlark:`, `//go:`, …) load just the provider without dragging in
language-specific BUILD machinery.

<a id="JsonschemaCodegenToolchainInfo"></a>

## JsonschemaCodegenToolchainInfo

<pre>
load("@rules_jsonschema//jsonschema:providers.bzl", "JsonschemaCodegenToolchainInfo")

JsonschemaCodegenToolchainInfo(<a href="#JsonschemaCodegenToolchainInfo-binary">binary</a>)
</pre>

A schema → code codegen tool.

**FIELDS**

| Name  | Description |
| :------------- | :------------- |
| <a id="JsonschemaCodegenToolchainInfo-binary"></a>binary |  File: the codegen executable. Invoked with `--schema PATH --out PATH` and any language-specific flags the calling rule passes through.    |


