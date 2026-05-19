# Changelog

All notable changes to rules_jsonschema. The format is loosely
[Keep a Changelog](https://keepachangelog.com/) — version headers
mirror the published bazel-registry entries.

## 0.2.0 — docs + CI infrastructure

- Stardoc-generated reference docs for all 8 public-API .bzl
  files in `docs/`. `bazel run //docs:update` regenerates;
  `bazel test //docs:all` gates the committed copies via
  `diff_test`.
- GitHub Actions CI: `bazel test //...` on ubuntu + macos, plus a
  buildifier lint job.
- `CHANGELOG.md` (this file).
- `.gitignore`: `.claude/` and `MODULE.bazel.lock`.

No API changes.

## 0.1.0 — initial release

- Language-neutral codegen plugin contract documented in
  `jsonschema/plugin_contract.md` — stdin = JSON Schema bytes,
  argv = `--key=value`, stdout = generated source, stderr + exit
  code for errors. Single-file output per rule invocation.
- Toolchain types per (language, use-case):
  `rust_typegen_toolchain_type`, `starlark_codegen_toolchain_type`,
  `go_typegen_toolchain_type`.
- `JsonschemaCodegenToolchainInfo` provider + `jsonschema_codegen_toolchain`
  rule to register plugins.
- Default in-repo plugins:
  - `tools/schema_to_rust` — wraps `typify` to emit serde-typed Rust.
  - `tools/schema_to_starlark` — emits Bazel `attr.*` declarations
    from object schemas (35 unit tests).
  - `tools/schema_to_go` — Go plugin using `go/format` (8 unit tests).
- User-facing rules: `jsonschema_rust_library`,
  `jsonschema_starlark_codegen`, `jsonschema_go_library`.
- `openapi_plugin_contract_test` conformance driver.
- `util/write_source_files` Starlark helper for the
  generate-then-commit workflow (replaces hand-rolled `.sh`
  scripts).
- Runtime helpers in `runtime/helpers.bzl` shared with downstream
  rules_* repos.
