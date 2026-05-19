<!-- Generated with Stardoc: http://skydoc.bazel.build -->

Go user-facing rules for rules_jsonschema.

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

<a id="jsonschema_go_library"></a>

## jsonschema_go_library

<pre>
load("@rules_jsonschema//go:defs.bzl", "jsonschema_go_library")

jsonschema_go_library(<a href="#jsonschema_go_library-name">name</a>, <a href="#jsonschema_go_library-schema">schema</a>, <a href="#jsonschema_go_library-importpath">importpath</a>, <a href="#jsonschema_go_library-package">package</a>, <a href="#jsonschema_go_library-extra_args">extra_args</a>, <a href="#jsonschema_go_library-visibility">visibility</a>,
                      <a href="#jsonschema_go_library-go_library_kwargs">**go_library_kwargs</a>)
</pre>

Generate a go_library of typed schema bindings.

The emitted package exports one Go type per schema `$defs` /
`definitions` entry plus a top-level type from the schema's
`title` (if set). Required properties become value-typed fields;
optional properties become pointer-typed with `,omitempty` tags.


**PARAMETERS**


| Name  | Description | Default Value |
| :------------- | :------------- | :------------- |
| <a id="jsonschema_go_library-name"></a>name |  go_library target name. Consumers add to `deps`.   |  none |
| <a id="jsonschema_go_library-schema"></a>schema |  label of a `.json` schema file.   |  none |
| <a id="jsonschema_go_library-importpath"></a>importpath |  Go import path for the generated package.   |  none |
| <a id="jsonschema_go_library-package"></a>package |  Go package name. Defaults to a sanitised rule name.   |  `None` |
| <a id="jsonschema_go_library-extra_args"></a>extra_args |  extra `--key=value` flags appended to the plugin's argv. Use to set plugin-specific options without registering a new toolchain.   |  `None` |
| <a id="jsonschema_go_library-visibility"></a>visibility |  forwarded to go_library.   |  `None` |
| <a id="jsonschema_go_library-go_library_kwargs"></a>go_library_kwargs |  forwarded to go_library.   |  none |


