"""Plugin conformance test.

`jsonschema_plugin_contract_test(name, plugin)` runs the contract
test driver against any executable that claims to implement the
rules_jsonschema plugin contract (see
[`plugin_contract.md`](plugin_contract.md)). The driver exercises:

  1. Minimum-viable invocation produces non-empty stdout + exit 0.
  2. Malformed JSON input → non-zero exit, stderr explanation,
     empty stdout (the discipline most likely to be violated by
     plugins emitting partial output before erroring).
  3. Unknown flags are rejected.
  4. Output is deterministic across identical invocations.

Plugin authors use it to gate their toolchain registration:

```python
load("@rules_jsonschema//jsonschema:contract_test.bzl",
     "jsonschema_plugin_contract_test")

jsonschema_plugin_contract_test(
    name = "my_plugin_conforms",
    plugin = "//my:rust_codegen",
)
```
"""

def _impl(ctx):
    runner = ctx.actions.declare_file(ctx.label.name + ".sh")
    plugin_sp = ctx.executable.plugin.short_path
    driver_sp = ctx.executable._driver.short_path

    # Resolve runfiles paths at runtime. short_path is relative to
    # the workspace root within runfiles, except external repos
    # which use `../external/REPO/...`. Same pattern docker_compose's
    # runners use.
    ctx.actions.write(
        output = runner,
        is_executable = True,
        content = """#!/usr/bin/env bash
set -euo pipefail
RUNFILES_DIR="${{RUNFILES_DIR:-$0.runfiles}}"
WS_NAME="{ws}"

resolve() {{
    local sp="$1"
    if [[ "$sp" == ../* ]]; then
        printf '%s' "$RUNFILES_DIR/${{sp#../}}"
    else
        printf '%s' "$RUNFILES_DIR/$WS_NAME/$sp"
    fi
}}

DRIVER="$(resolve "{driver_sp}")"
PLUGIN="$(resolve "{plugin_sp}")"
exec "$DRIVER" "$PLUGIN"
""".format(
            ws = ctx.workspace_name,
            driver_sp = driver_sp,
            plugin_sp = plugin_sp,
        ),
    )

    runfiles = ctx.runfiles(files = [
        ctx.executable._driver,
        ctx.executable.plugin,
    ])
    return [DefaultInfo(executable = runner, runfiles = runfiles)]

jsonschema_plugin_contract_test = rule(
    implementation = _impl,
    test = True,
    attrs = {
        "plugin": attr.label(
            executable = True,
            cfg = "exec",
            mandatory = True,
            doc = "The plugin binary to test. Any executable that " +
                  "claims to implement the rules_jsonschema plugin contract.",
        ),
        "_driver": attr.label(
            default = Label("//tools/contract_test:contract_test_driver"),
            executable = True,
            cfg = "exec",
        ),
    },
    doc = "Run the rules_jsonschema plugin contract scenarios against a plugin binary.",
)
