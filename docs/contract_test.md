<!-- Generated with Stardoc: http://skydoc.bazel.build -->

Plugin conformance test.

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

<a id="jsonschema_plugin_contract_test"></a>

## jsonschema_plugin_contract_test

<pre>
load("@rules_jsonschema//jsonschema:contract_test.bzl", "jsonschema_plugin_contract_test")

jsonschema_plugin_contract_test(<a href="#jsonschema_plugin_contract_test-name">name</a>, <a href="#jsonschema_plugin_contract_test-plugin">plugin</a>)
</pre>

Run the rules_jsonschema plugin contract scenarios against a plugin binary.

**ATTRIBUTES**


| Name  | Description | Type | Mandatory | Default |
| :------------- | :------------- | :------------- | :------------- | :------------- |
| <a id="jsonschema_plugin_contract_test-name"></a>name |  A unique name for this target.   | <a href="https://bazel.build/concepts/labels#target-names">Name</a> | required |  |
| <a id="jsonschema_plugin_contract_test-plugin"></a>plugin |  The plugin binary to test. Any executable that claims to implement the rules_jsonschema plugin contract.   | <a href="https://bazel.build/concepts/labels">Label</a> | required |  |


