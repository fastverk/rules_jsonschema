"""Providers exposed by rules_jsonschema.

`JsonschemaCodegenToolchainInfo` is the contract every codegen
toolchain provides: a single `binary` File that implements the
schema → output-language conversion. Per-language user-facing rules
resolve a toolchain by type
(`@rules_jsonschema//jsonschema:<lang>_codegen_toolchain_type`),
fetch this provider, and run the binary.

Splitting it out from `defs.bzl` lets language modules (`//rust:`,
`//starlark:`, `//go:`, …) load just the provider without dragging in
language-specific BUILD machinery.
"""

JsonschemaCodegenToolchainInfo = provider(
    doc = "A schema → code codegen tool.",
    fields = {
        "binary": "File: the codegen executable. Invoked with " +
                  "`--schema PATH --out PATH` and any language-specific " +
                  "flags the calling rule passes through.",
    },
)
