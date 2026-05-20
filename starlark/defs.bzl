"""Starlark user-facing rule for rules_jsonschema.

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
"""

_TOOLCHAIN = "@rules_jsonschema//jsonschema:starlark_codegen_toolchain_type"

def _starlark_codegen_impl(ctx):
    out = ctx.actions.declare_file(ctx.label.name + ".bzl")
    tc = ctx.toolchains[_TOOLCHAIN].codegen_info
    # Plugin contract: stdin = schema bytes, stdout = generated file,
    # argv = --key=value pairs (see //jsonschema/plugin_contract.md).
    cmd_parts = [
        tc.binary.path,
        "--schema-name={}".format(ctx.file.schema.basename),
        "--rule-name={}".format(ctx.label.name),
    ]
    for encoded in ctx.attr.kinds_encoded:
        cmd_parts.append("--kind={}".format(_shell_quote(encoded)))
    for arg in ctx.attr.extra_args:
        cmd_parts.append(_shell_quote(arg))
    cmd_parts.extend(["<", ctx.file.schema.path, ">", out.path])
    ctx.actions.run_shell(
        outputs = [out],
        inputs = [ctx.file.schema],
        tools = [tc.binary],
        command = " ".join(cmd_parts),
        mnemonic = "JsonschemaStarlarkCodegen",
        progress_message = "schema → starlark %s" % ctx.label,
    )
    return [DefaultInfo(files = depset([out]))]

def _shell_quote(s):
    # Bazel-provided strings (kinds_encoded) come from the macro;
    # they shouldn't contain single quotes. Single-quoting handles
    # the `#`, `:`, and `/` characters in JSON-pointers + provider
    # names that would otherwise be shell-interpreted.
    return "'" + s.replace("'", "'\\''") + "'"

_jsonschema_starlark_codegen_rule = rule(
    implementation = _starlark_codegen_impl,
    attrs = {
        "schema": attr.label(allow_single_file = [".json"], mandatory = True),
        "kinds_encoded": attr.string_list(
            doc = "Each entry: 'id=pointer:rule_name:provider_name'.",
        ),
        "extra_args": attr.string_list(
            doc = "Extra `--key=value` flags appended to the plugin invocation. " +
                  "Use to set plugin-specific options without registering a new " +
                  "toolchain.",
        ),
    },
    toolchains = [_TOOLCHAIN],
)

def jsonschema_starlark_codegen(name, schema, kinds = None, extra_args = None, **kwargs):
    """Generate a `.bzl` of typed rules from a JSON Schema.

    Args:
      name: target name; output file is `<name>.bzl`.
      schema: label of a `.json` schema document.
      kinds: list of `(id, pointer, rule_name, provider_name)` 4-tuples.
        - `id`: short tag used in generated symbol names + the
          rule-name attr (e.g. `service`).
        - `pointer`: JSON-pointer into the schema for the definition
          whose `properties` become attrs (e.g. `#/definitions/service`).
        - `rule_name`: the public Starlark symbol the emitted rule
          binds to.
        - `provider_name`: the public Starlark symbol the rule's
          companion provider binds to.
        Optional — if omitted, `extra_args` typically enables the
        plugin's auto-kinds derivation (e.g.
        `--kinds-pointer-base=...` for the default
        `schema_to_starlark` toolchain). Leaving both empty produces
        a preamble-only `.bzl` (legal but rarely useful).
      extra_args: extra `--key=value` flags appended to the plugin's
        argv. Use to set plugin-specific options without registering
        a new toolchain.
      **kwargs: forwarded to the underlying rule (visibility, etc.).
    """
    encoded = []
    for spec in kinds or []:
        if type(spec) != "tuple" or len(spec) != 4:
            fail("kinds entries must be 4-tuples (id, pointer, rule_name, provider_name); got: {}".format(spec))
        encoded.append("{}={}:{}:{}".format(spec[0], spec[1], spec[2], spec[3]))
    _jsonschema_starlark_codegen_rule(
        name = name,
        schema = schema,
        kinds_encoded = encoded,
        extra_args = extra_args or [],
        **kwargs
    )
