<!-- Generated with Stardoc: http://skydoc.bazel.build -->

`write_source_files`: copy generated outputs back into source.

The canonical Bazel pattern for committed-codegen workflows. A typical
setup pairs a codegen rule (whose output sits under `bazel-bin/...`)
with a `write_source_files` target that copies the output to a path
under source control:

    jsonschema_starlark_codegen(
        name = "compose_rules_gen",
        schema = "...",
        kinds = [...],
    )

    write_source_files(
        name = "update_compose_rules",
        files = {
            "compose_rules.bzl": ":compose_rules_gen",
        },
    )

  * `bazel build //compose:update_compose_rules` — no-op.
  * `bazel run //compose:update_compose_rules` — copies each
    generated file to its source-tree destination, respecting
    `BUILD_WORKSPACE_DIRECTORY` so multi-repo workspaces still work.

Pair with a `diff_test` to gate freshness:

    diff_test(
        name = "compose_rules_up_to_date",
        file1 = "compose_rules.bzl",
        file2 = ":compose_rules_gen",
    )

This rule replaces ad-hoc `sh_binary + update.sh` pairs throughout
rules_jsonschema's consumers. Functionally equivalent to
`@aspect_bazel_lib//lib:write_source_files.bzl`, but in-repo so we
don't take on `aspect_bazel_lib` as a dep for a single rule.

<a id="write_source_files"></a>

## write_source_files

<pre>
load("@rules_jsonschema//util:write_source_files.bzl", "write_source_files")

write_source_files(<a href="#write_source_files-name">name</a>, <a href="#write_source_files-files">files</a>)
</pre>

`bazel run`-able target that copies generated files back into source control.

**ATTRIBUTES**


| Name  | Description | Type | Mandatory | Default |
| :------------- | :------------- | :------------- | :------------- | :------------- |
| <a id="write_source_files-name"></a>name |  A unique name for this target.   | <a href="https://bazel.build/concepts/labels#target-names">Name</a> | required |  |
| <a id="write_source_files-files"></a>files |  Map of package-relative destination path → label whose single output file should be copied there. Each source label must produce exactly one output file.   | Dictionary: String -> Label | required |  |


