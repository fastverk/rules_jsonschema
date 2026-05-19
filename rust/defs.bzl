"""Rust user-facing rules for rules_jsonschema.

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
"""

load("@rules_rust//rust:defs.bzl", "rust_library")

_TOOLCHAIN = "@rules_jsonschema//jsonschema:rust_codegen_toolchain_type"

def _rust_codegen_action_impl(ctx):
    out = ctx.actions.declare_file(ctx.label.name + ".rs")
    tc = ctx.toolchains[_TOOLCHAIN].codegen_info
    # Plugin contract: stdin = schema bytes, stdout = generated file,
    # argv = --key=value pairs (see //jsonschema/plugin_contract.md).
    cmd_parts = [
        tc.binary.path,
        "--schema-name={}".format(ctx.file.schema.basename),
        "--rule-name={}".format(ctx.label.name),
    ]
    for arg in ctx.attr.extra_args:
        cmd_parts.append(_shell_quote(arg))
    cmd_parts.extend(["<", ctx.file.schema.path, ">", out.path])
    ctx.actions.run_shell(
        outputs = [out],
        inputs = [ctx.file.schema],
        tools = [tc.binary],
        command = " ".join(cmd_parts),
        mnemonic = "JsonschemaTypegenRs",
        progress_message = "schema → rust %s" % ctx.label,
    )
    return [DefaultInfo(files = depset([out]))]

_jsonschema_rust_codegen = rule(
    implementation = _rust_codegen_action_impl,
    attrs = {
        "schema": attr.label(
            allow_single_file = [".json"],
            mandatory = True,
            doc = "JSON Schema document.",
        ),
        "extra_args": attr.string_list(
            doc = "Extra `--key=value` flags appended to the plugin invocation. " +
                  "Use this to set plugin-specific options without registering " +
                  "a new toolchain.",
        ),
    },
    toolchains = [_TOOLCHAIN],
)

def _shell_quote(s):
    # Bazel-provided strings shouldn't contain single quotes; the
    # standard escape handles them anyway.
    return "'" + s.replace("'", "'\\''") + "'"

def jsonschema_rust_library(
        name,
        schema,
        extra_args = None,
        serde = None,
        serde_json = None,
        regress = None,
        visibility = None,
        **rust_library_kwargs):
    """Generate a rust_library of typed schema bindings.

    The emitted library exports one Rust struct/enum per top-level
    JSON-Schema definition, with `#[derive(Serialize, Deserialize)]`
    plus `#[serde(deny_unknown_fields)]` wherever the source schema
    sets `additionalProperties: false`.

    Args:
      name: rust_library target name. Consumers add this to `deps`.
      schema: label of a `.json` schema file.
      extra_args: extra `--key=value` flags appended to the plugin's
        argv. Use to set plugin-specific options without registering
        a new toolchain. The default plugin (schema_to_rust) accepts
        no extra flags today; consumers of custom toolchains will.
      serde: label of the `serde` crate to use as a direct dep.
        Defaults to rules_jsonschema's own `@crates//:serde`.
        **Consumers whose binary also depends on serde must point this
        at their own crate repo**, otherwise the generated types' trait
        impls live in a different compile unit than the consumer's and
        Rust treats them as distinct types
        (`error[E0277]: the trait bound Service: serde::Serialize is
        not satisfied`).
      serde_json: same story for `serde_json`.
      regress: same story for `regress` (typify uses it for
        `pattern`-validated string newtypes).
      visibility: forwarded to rust_library.
      **rust_library_kwargs: forwarded to rust_library (e.g. extra `deps`).
    """
    gen_name = name + "_rs_gen"
    _jsonschema_rust_codegen(
        name = gen_name,
        schema = schema,
        extra_args = extra_args or [],
    )
    rt_deps = [
        serde or Label("@crates//:serde"),
        serde_json or Label("@crates//:serde_json"),
        regress or Label("@crates//:regress"),
    ]
    extra_deps = rust_library_kwargs.pop("deps", [])
    rust_library(
        name = name,
        srcs = [":" + gen_name],
        edition = "2021",
        deps = rt_deps + extra_deps,
        visibility = visibility,
        **rust_library_kwargs
    )
