# rules_jsonschema

Bazel rules that turn a JSON Schema document into typed code, with
output languages pluggable via Bazel toolchains. The schema is the
single source of truth: an unknown field is a build-time decode
error, and the set of generated types regenerates on every build.

## Architecture

Plugins implement a minimal stdin/argv/stdout contract; per-language
Bazel rules wrap them. The host repo only registers toolchains.

```
//jsonschema:                 language-neutral core
  - toolchain_type definitions per output language
  - JsonschemaCodegenToolchainInfo provider
  - jsonschema_codegen_toolchain rule (register a plugin)
  - jsonschema_plugin_contract_test rule (verify a plugin conforms)
  - plugin_contract.md (authoritative spec)

//rust:                       Rust output
  - jsonschema_rust_library rule
  - default toolchain → //tools/schema_to_rust (Rust, typify-backed)

//go:                         Go output
  - jsonschema_go_library rule
  - default toolchain → //tools/schema_to_go (Go, uses go/format)

//starlark:                   Bazel rule (.bzl) output
  - jsonschema_starlark_codegen rule
  - default toolchain → //tools/schema_to_starlark (Rust)

//util/write_source_files.bzl committed-codegen helper rule
//runtime/helpers.bzl         shared Starlark helpers loaded by emitted .bzl
```

Adding a new output language is:

1. Write a plugin binary in *that language* (it gets to leverage
   native AST tooling — `go/format`, `quote`/`syn`, `ts-morph`).
2. Register a `jsonschema_codegen_toolchain` pointing at it.
3. Add a `jsonschema_<lang>_library` user-facing rule that wraps the
   target language's `*_library` Bazel rule.

## The plugin contract

A plugin is any executable that conforms to:

```
INPUT
  stdin              schema bytes (JSON)
  argv               --key=value pairs

OUTPUT
  stdout             generated file content (single file per invocation)
  stderr             diagnostics

EXIT
  0                  success
  non-zero           failure
```

Standard argv flags every plugin receives: `--schema-name=NAME`
(schema basename) and `--rule-name=NAME` (Bazel target name).
Rule-specific flags (e.g. `--kind=...`, `--package=...`) are passed
through by the calling rule.

A 15-line Python plugin is a real plugin:

```python
import json, sys
schema = json.load(sys.stdin.buffer)
# ... generate ...
sys.stdout.write(generated)
```

See [`jsonschema/plugin_contract.md`](jsonschema/plugin_contract.md)
for the authoritative spec.

## What ships

- **`jsonschema_rust_library`** (in [rust/defs.bzl](rust/defs.bzl)) —
  Rust struct/enum bindings via the default typify-backed plugin.
  Emits `#[derive(Serialize, Deserialize, Clone, Debug)]` plus
  `#[serde(deny_unknown_fields)]` where the schema sets
  `additionalProperties: false`.
- **`jsonschema_go_library`** (in [go/defs.bzl](go/defs.bzl)) — Go
  struct bindings. Optional properties become `*T` with `,omitempty`;
  required become value types. Default plugin uses `go/format` for
  canonical layout.
- **`jsonschema_starlark_codegen`** (in [starlark/defs.bzl](starlark/defs.bzl))
  — typed Bazel `rule()` + `provider()` definitions, one per
  requested schema definition. Output is committed via
  `write_source_files` + gated with `diff_test`.
- **`jsonschema_codegen_toolchain`** (in [jsonschema/toolchains.bzl](jsonschema/toolchains.bzl))
  — wrap your own plugin binary as a Bazel toolchain to override
  defaults.
- **`jsonschema_plugin_contract_test`** (in [jsonschema/contract_test.bzl](jsonschema/contract_test.bzl))
  — runs contract scenarios (valid input, malformed input, unknown
  flag, determinism) against any plugin binary. Plugin authors gate
  toolchain registration with it.
- **`write_source_files`** (in [util/write_source_files.bzl](util/write_source_files.bzl))
  — the canonical "copy generated outputs back into source" rule.
  Replaces hand-rolled `sh_binary + update.sh` patterns.
- **`@rules_jsonschema//runtime:helpers.bzl`** — `strip_empty` and
  `parse_json_or_none`, called by `jsonschema_starlark_codegen`'s
  emitted rule impls.

[`rules_docker_compose`](https://github.com/fastverk/rules_docker_compose)
is the production consumer.

## Install

`.bazelrc`:

```
common --registry=https://raw.githubusercontent.com/fastverk/bazel-registry/main/
common --registry=https://bcr.bazel.build/
```

`MODULE.bazel`:

```python
bazel_dep(name = "rules_jsonschema", version = "0.1.0")
```

`rules_rust`, `rules_go`, a Rust toolchain (1.88+), `crates_universe`,
and a Go SDK are pulled in transitively. Default Rust + Go + Starlark
toolchains are registered automatically.

## Per-language rules

### `jsonschema_rust_library`

```python
load("@rules_jsonschema//rust:defs.bzl", "jsonschema_rust_library")

jsonschema_rust_library(
    name = "person_types",
    schema = "person.schema.json",
    # When the consumer's binary depends on serde from its own
    # crates_universe, thread the labels through so trait identities
    # match. See "Two crates_universe instances" below.
    serde      = "@my_crates//:serde",
    serde_json = "@my_crates//:serde_json",
    regress    = "@my_crates//:regress",
    # Optional per-plugin flags — empty for the default plugin.
    extra_args = ["--my-custom-flag=value"],
)
```

### `jsonschema_go_library`

```python
load("@rules_jsonschema//go:defs.bzl", "jsonschema_go_library")

jsonschema_go_library(
    name = "person_types",
    schema = "person.schema.json",
    importpath = "github.com/myorg/person_types",
    package = "person_types",
)
```

### `jsonschema_starlark_codegen`

```python
load("@rules_jsonschema//starlark:defs.bzl", "jsonschema_starlark_codegen")
load("@rules_jsonschema//util:write_source_files.bzl", "write_source_files")
load("@bazel_skylib//rules:diff_test.bzl", "diff_test")

jsonschema_starlark_codegen(
    name = "compose_rules_gen",
    schema = "compose-spec.json",
    kinds = [
        # (id, schema-pointer, rule_name, provider_name)
        ("service", "#/definitions/service",
         "docker_compose_service", "ComposeServiceInfo"),
        ("volume", "#/definitions/volume",
         "docker_compose_volume",  "ComposeVolumeInfo"),
    ],
)

diff_test(
    name = "compose_rules_up_to_date",
    file1 = "compose_rules.bzl",
    file2 = ":compose_rules_gen",
)

write_source_files(
    name = "update_compose_rules",
    files = {"compose_rules.bzl": ":compose_rules_gen"},
)
```

The committed `compose_rules.bzl` is loaded normally. The `diff_test`
gates freshness; `bazel run :update_compose_rules` refreshes.

## Property → attr mapping (Starlark codegen)

| Schema | Bazel attr |
|---|---|
| `type: "string"` | `attr.string` |
| `enum` of strings | `attr.string(values=...)` |
| `type: "integer"` / `"number"` | `attr.int` |
| `type: "boolean"` | `attr.bool` |
| Array of strings (incl. `oneOf [string, object]` short-form) | `attr.string_list` |
| Object with string-valued props (incl. `patternProperties`) | `attr.string_dict` |
| `$ref` to `string_or_list` / `list_of_strings` / `command` | `attr.string_list` |
| `$ref` to `list_or_dict` | `attr.string_dict` |
| `type: [...]` multi-type unions | preference: bool > string > int |
| Everything else | `attr.string()` taking JSON-encoded text |

The full mapping logic lives in [`tools/schema_to_starlark/src/classifier.rs`](tools/schema_to_starlark/src/classifier.rs);
each classifier (`classify_named_ref`, `classify_one_of`, `classify_enum`,
`classify_type_union`, `classify_single_type`) is independently
unit-tested.

## Swapping a plugin

Each language's default toolchain points at rules_jsonschema's
in-repo binary. To use a different plugin, register your own
toolchain ahead of ours:

```python
# Your MODULE.bazel
register_toolchains(
    "//your/path:my_custom_rust_codegen_toolchain",
)
```

```python
# Your BUILD.bazel
load("@rules_jsonschema//jsonschema:toolchains.bzl", "jsonschema_codegen_toolchain")

jsonschema_codegen_toolchain(
    name = "my_custom_rust_codegen",
    binary = "//path/to:your_binary",  # any executable conforming to plugin_contract.md
)

toolchain(
    name = "my_custom_rust_codegen_toolchain",
    toolchain = ":my_custom_rust_codegen",
    toolchain_type = "@rules_jsonschema//jsonschema:rust_codegen_toolchain_type",
)
```

Gate your plugin with the conformance test:

```python
load("@rules_jsonschema//jsonschema:contract_test.bzl",
     "jsonschema_plugin_contract_test")

jsonschema_plugin_contract_test(
    name = "my_plugin_conforms",
    plugin = "//path/to:your_binary",
)
```

## Two `crates_universe` instances

When both `rules_jsonschema` (host) and a consumer module use their
own `crates_universe` extensions, the same crate (`serde`, `regress`)
gets compiled twice. Rust treats the two compilations as **distinct
types**: the generated `Service: Serialize` trait impl lives in the
host's `serde`, while the consumer's code references its own
`serde`. The bound fails with `error[E0277]`.

`jsonschema_rust_library`'s optional `serde` / `serde_json` /
`regress` attrs thread the consumer's crates through so the
generated library compiles against the same serde the consumer
links. Defaults point at `rules_jsonschema`'s own `@crates`, which
works for within-repo callers but not for downstream consumers —
explicit threading is required there.

## Compatibility

- **Bazel**: 7.4+, bzlmod required (tested on 9.1).
- **Rust**: 1.88+ (transitive deps need stabilised `let-chains`).
- **Go**: 1.23+ (default SDK pinned in `MODULE.bazel`).
- **JSON Schema**: Draft 2020-12, with the typify-supported subset
  (refs, oneOf, allOf, enum, additionalProperties, patternProperties).

## Testing

```sh
bazel test //...
```

| Target | Coverage |
|---|---|
| `//tools/schema_to_starlark:schema_to_starlark_test` | 35 Rust unit tests on classifier + emission helpers |
| `//tools/schema_to_go:schema_to_go_test` | 8 Go unit tests on type mapping + name munging |
| `//tools/schema_to_rust:schema_to_rust_conforms` | plugin contract conformance |
| `//tools/schema_to_starlark:schema_to_starlark_conforms` | plugin contract conformance |
| `//tools/schema_to_go:schema_to_go_conforms` | plugin contract conformance |
| `//examples/smoke:person_types_test` | Rust types decode/encode round-trip |
| `//examples/smoke:person_go_types_test` | Go types decode/encode round-trip |
| `//examples/smoke:person_rules_up_to_date` | Starlark codegen output stays in sync |

## Design

The architecture pivot from "Rust-binary-per-output-language" to the
current contract-based plugin model is captured in
[`docs/RFC-001-codegen-plugin-protocol.md`](docs/RFC-001-codegen-plugin-protocol.md)
(with the commit history showing the design iterations the RFC
went through).

## License

MIT.
