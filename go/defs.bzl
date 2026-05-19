"""Go user-facing rules for rules_jsonschema.

`jsonschema_go_library` is the Go-specific shape of the schema → code
pipeline:

  1. Resolves the `go_codegen_toolchain_type` toolchain.
  2. Runs the toolchain's binary on the schema (stdin/argv/stdout
     per `//jsonschema/plugin_contract.md`), producing a `.go` file.
  3. Wraps the `.go` in a `go_library` from `@rules_go`.

The default toolchain (registered by rules_jsonschema's MODULE.bazel)
points at the in-repo `schema_to_go` Go binary. Coverage is minimal —
primitives, structs, slices, maps, optional pointers, refs. For
fuller JSON-Schema-to-Go support, register your own
`jsonschema_codegen_toolchain` pointing at a different binary (e.g.
[atombender/go-jsonschema](https://github.com/atombender/go-jsonschema)).
"""

load("@rules_go//go:def.bzl", "go_library")

_TOOLCHAIN = "@rules_jsonschema//jsonschema:go_codegen_toolchain_type"

def _go_codegen_action_impl(ctx):
    out = ctx.actions.declare_file(ctx.label.name + ".go")
    tc = ctx.toolchains[_TOOLCHAIN].codegen_info
    # Plugin contract: stdin = schema bytes, stdout = generated file,
    # argv = --key=value pairs.
    cmd_parts = [
        tc.binary.path,
        "--schema-name={}".format(ctx.file.schema.basename),
        "--rule-name={}".format(ctx.label.name),
    ]
    if ctx.attr.package:
        cmd_parts.append("--package={}".format(ctx.attr.package))
    for arg in ctx.attr.extra_args:
        cmd_parts.append(_shell_quote(arg))
    cmd_parts.extend(["<", ctx.file.schema.path, ">", out.path])
    ctx.actions.run_shell(
        outputs = [out],
        inputs = [ctx.file.schema],
        tools = [tc.binary],
        command = " ".join(cmd_parts),
        mnemonic = "JsonschemaTypegenGo",
        progress_message = "schema → go %s" % ctx.label,
    )
    return [DefaultInfo(files = depset([out]))]

_jsonschema_go_codegen = rule(
    implementation = _go_codegen_action_impl,
    attrs = {
        "schema": attr.label(
            allow_single_file = [".json"],
            mandatory = True,
            doc = "JSON Schema document.",
        ),
        "package": attr.string(
            doc = "Go package name. Defaults to the rule name (sanitised) " +
                  "if not set, or `generated` as a last resort.",
        ),
        "extra_args": attr.string_list(
            doc = "Extra `--key=value` flags appended to the plugin invocation. " +
                  "Use to set plugin-specific options without registering a new " +
                  "toolchain.",
        ),
    },
    toolchains = [_TOOLCHAIN],
)

def _shell_quote(s):
    return "'" + s.replace("'", "'\\''") + "'"

def jsonschema_go_library(name, schema, importpath, package = None, extra_args = None, visibility = None, **go_library_kwargs):
    """Generate a go_library of typed schema bindings.

    The emitted package exports one Go type per schema `$defs` /
    `definitions` entry plus a top-level type from the schema's
    `title` (if set). Required properties become value-typed fields;
    optional properties become pointer-typed with `,omitempty` tags.

    Args:
      name: go_library target name. Consumers add to `deps`.
      schema: label of a `.json` schema file.
      importpath: Go import path for the generated package.
      package: Go package name. Defaults to a sanitised rule name.
      extra_args: extra `--key=value` flags appended to the plugin's
        argv. Use to set plugin-specific options without registering
        a new toolchain.
      visibility: forwarded to go_library.
      **go_library_kwargs: forwarded to go_library.
    """
    gen_name = name + "_go_gen"
    _jsonschema_go_codegen(
        name = gen_name,
        schema = schema,
        package = package,
        extra_args = extra_args or [],
    )
    go_library(
        name = name,
        srcs = [":" + gen_name],
        importpath = importpath,
        visibility = visibility,
        **go_library_kwargs
    )
