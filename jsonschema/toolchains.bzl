"""Toolchain rules for rules_jsonschema codegen.

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
"""

load(":providers.bzl", "JsonschemaCodegenToolchainInfo")

def _jsonschema_codegen_toolchain_impl(ctx):
    return [platform_common.ToolchainInfo(
        codegen_info = JsonschemaCodegenToolchainInfo(
            binary = ctx.executable.binary,
        ),
    )]

jsonschema_codegen_toolchain = rule(
    implementation = _jsonschema_codegen_toolchain_impl,
    attrs = {
        "binary": attr.label(
            executable = True,
            cfg = "exec",
            mandatory = True,
            doc = "The codegen executable for this toolchain. Must accept " +
                  "`--schema PATH --out PATH` plus any language-specific flags.",
        ),
    },
    doc = "Declare a schema → code codegen executable as a Bazel toolchain.",
)
