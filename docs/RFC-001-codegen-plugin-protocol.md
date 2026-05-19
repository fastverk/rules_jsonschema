# RFC-001 — Codegen Plugin Protocol

> Status: **draft, revised**. Captures the architecture pivot from
> "Rust-binary-per-output-language" to "per-language plugins reading
> the schema directly via a minimal stdin/stdout contract".
>
> Earlier drafts of this RFC proposed a protoc-style architecture
> with a frontend, a parsed AST proto, and a dual `ast` / `raw`
> plugin mode. That design was abandoned because (a) JSON Schema is
> already JSON — every plugin language can parse it directly — and
> (b) most realistic plugins wrap upstream tools (`typify`,
> `atombender/go-jsonschema`, `oapi-codegen`, …) that have their own
> parsing anyway. The AST was a small spec language we'd be
> inventing for marginal benefit. See "Why we abandoned the AST"
> below for the full reasoning.

## Goal

Decouple rules_jsonschema's user-facing rules from a hardcoded codegen
language. After this RFC lands, adding a new output language is:

1. Write a plugin binary **in that language** so it leverages native
   AST tooling — `go/format` for Go, `quote`/`syn` for Rust,
   `ts-morph` for TypeScript.
2. Register a `jsonschema_codegen_toolchain` pointing at it.
3. Add a `jsonschema_<lang>_library` user-facing rule that wraps the
   target language's `*_library` Bazel rule.

The plugin reads the schema bytes from stdin, options from argv,
writes the generated file content to stdout, and signals errors via
stderr + exit code. No protobuf dep, no AST proto, no frontend
binary. Stdlib-only plugins are achievable in any language.

## The contract

A plugin is any executable that conforms to:

```
INPUT
  stdin              the schema file contents (raw bytes)
  argv               --key=value pairs, repeated. Plugin-specific.
                     The rule may also pass standard flags it owns.

OUTPUT
  stdout             the generated file content (raw bytes)
  stderr             diagnostics / error messages

EXIT
  0                  success — stdout is the generated file
  non-zero           failure — stderr explains why
```

That's it. A plugin in Go is:

```go
package main

import (
    "encoding/json"
    "io"
    "os"
)

func main() {
    schemaBytes, _ := io.ReadAll(os.Stdin)
    var schema map[string]any
    if err := json.Unmarshal(schemaBytes, &schema); err != nil {
        fmt.Fprintln(os.Stderr, "parse:", err)
        os.Exit(1)
    }
    // ... generate Go source from schema ...
    os.Stdout.Write([]byte(generated))
}
```

A plugin in Rust is the same thing with `serde_json`. A plugin in
Python wraps `json.load(sys.stdin.buffer)`. There is no contract-
specific dep in any language.

### Standard argv conventions

The rule passes a fixed set of flags every plugin receives, plus
whatever the consumer set in `options`:

| Flag | Set by | Meaning |
|---|---|---|
| `--schema-name=NAME` | rule | Original schema file basename (e.g. `compose-spec.json`). For error messages and stable codegen header comments. |
| `--rule-name=NAME` | rule | The Bazel target's name. Useful for picking output identifiers. |
| `--<consumer-flag>=VAL` | consumer | Free-form per-plugin options from the rule attrs. |

Plugins should treat unknown flags as a hard error so misconfigured
options don't silently degrade output.

## Bazel output declaration

Bazel rules **must declare their outputs at analysis time**, before
any action runs. Three real options were considered:

| Approach | Pros | Cons |
|---|---|---|
| **A. Single file per rule invocation** | Output path known at analysis. Simple. Matches `protoc-gen-go` in practice. | Plugin authors can't naturally split output. |
| **B. `declare_directory` (tree artifact)** | Plugin emits arbitrarily many files. | Downstream `rust_library` / `go_library` rules have to glob the directory or expand it. Awkward, non-standard. |
| **C. Two-pass: pre-flight + emit** | Plugin advertises outputs given a schema, then generates. | Two plugin invocations per build. Doubles action overhead. |

**Decision: A.** Plugin produces exactly one file (on stdout) per
rule invocation. Multi-output needs (types vs validators, client vs
server) split into separate rule targets:

```python
jsonschema_go_types(name = "person_types", schema = "person.json")
jsonschema_go_validators(name = "person_validators", schema = "person.json")
```

Each target is independently cacheable; the build graph is clearer.
Tree artifacts (B) remain available as an escape hatch for the rare
genuinely-multi-file plugin.

## Bazel rule shape

Each per-language user-facing rule has the same structure:

```python
def _jsonschema_rust_codegen_impl(ctx):
    out = ctx.actions.declare_file(ctx.label.name + ".rs")
    tc = ctx.toolchains[_RUST_TOOLCHAIN].codegen_info

    args = [
        "--schema-name=" + ctx.file.schema.basename,
        "--rule-name=" + ctx.label.name,
    ]
    # Plugin-specific options passed through from rule attrs.
    for k, v in ctx.attr.options.items():
        args.append("--{}={}".format(k, v))

    ctx.actions.run_shell(
        inputs = [ctx.file.schema],
        outputs = [out],
        tools = [tc.binary],
        command = '{plugin} {args} < {schema} > {out}'.format(
            plugin = tc.binary.path,
            args = " ".join([shell.quote(a) for a in args]),
            schema = ctx.file.schema.path,
            out = out.path,
        ),
    )
    return [DefaultInfo(files = depset([out]))]
```

User-facing macro composes that codegen with the target language's
library rule:

```python
def jsonschema_rust_library(name, schema, **kwargs):
    gen_name = name + "_rs_gen"
    _jsonschema_rust_codegen(name = gen_name, schema = schema)
    rust_library(
        name = name,
        srcs = [":" + gen_name],
        edition = "2021",
        deps = [...],
        **kwargs
    )
```

Same shape per language.

## Why we abandoned the AST

The first draft of this RFC proposed a protoc-style architecture: a
frontend parses the schema into a canonical AST proto, plugins
consume that AST instead of raw bytes. After looking at it harder I
think this was the wrong call. Reasons:

1. **The protoc analogy doesn't transfer.** protoc has an AST
   because `.proto` files have a grammar nobody else has implemented.
   Plugin authors would otherwise re-implement parsing. JSON Schema
   is already JSON — every plugin language has a JSON parser in
   stdlib or one-line dep. The "no plugin reparses" argument is
   ~free to ignore for us.

2. **Most plugins wrap upstream tools.** `typify`,
   `atombender/go-jsonschema`, `oapi-codegen`, `openapi-generator`
   all take raw schema bytes and have their own parsing. Our AST
   would be throwaway work for them. The dual `mode = "ast" | "raw"`
   we briefly proposed was *evidence* the AST wasn't the natural fit.

3. **Cross-plugin consistency was illusory.** Different upstream
   tools interpret edge cases differently (recursive refs, allOf
   ordering, oneOf discriminator behavior). Putting an AST in front
   doesn't unify them — each wrapping plugin still defers to its
   underlying library.

4. **Maintenance cost is real.** Defining `Schema` / `Type` /
   `UnionType` / `IntersectionType` is a small spec language we
   invent and ship. Every JSON Schema feature we don't model
   becomes an `extra_json` escape hatch. We'd end up maintaining a
   parallel type system that nothing consumes natively.

5. **Plugin author ergonomics matter.** "Read stdin, write stdout"
   is the lowest possible barrier to entry. A Bash script could be
   a plugin. Adding "deserialise a protobuf request" pushes plugin
   authors into language-specific toolchain setup before they write
   the first line of codegen logic.

The toolchain pattern (toolchain types per output language, register
your own plugin to override) survives the simplification unchanged.

## Why we also abandoned the proto envelope

Even without an AST, we considered keeping a thin proto wrapper:
`CodeGenRequest{raw_schema, options, version}` in, `CodeGenResponse{file, error, features}` out. Forward-compat without the AST baggage.

The argument against:

- The structured-options part is the only piece of the proto that
  isn't trivially expressible as stdin/argv/stderr/exit-code. argv
  handles structured options fine.
- For ~5 plugins over the foreseeable future, "add a field without
  breaking old plugins" isn't load-bearing; we can coordinate.
- Plugin author barrier matters more than abstract evolvability. A
  one-file Python plugin (15 lines) beats a Rust plugin with
  protobuf codegen deps for any reasonable measure.
- We can always add a proto envelope later if we hit a real wall.
  Migrating plugins is straightforward — only the stdin-parsing
  changes, the codegen logic doesn't.

## Open questions

1. **Stable JSON Schema spec-version handling.** Plugins should
   probably refuse to operate on schemas whose `$schema` doesn't
   match what they expect. Convention: plugins error with
   `--schema-name=… : unsupported $schema: <value>` rather than
   producing wrong output. Each plugin owns its own version
   detection.

2. **Cross-plugin shared parsing.** If we ever need it (we don't
   yet), a future RFC could add an optional sidecar artifact: the
   rule runs a one-time `jsonschema_parse` action that emits a
   normalised JSON form, and plugins opt into reading that instead
   of the original schema. Backward compatible — old plugins
   still consume raw.

3. **Diagnostic format.** stderr is freeform today. If we ever want
   structured diagnostics (file:line:col annotations), we'd define
   a stderr-line format like `WARNING:path:line:col:msg`. Not v1.

4. **Toolchain attr surface.** Currently the toolchain rule just
   carries `binary`. Future fields might include: `supported_drafts`
   (list of `$schema` values), `default_options` (dict), `version`
   (for diagnostic banners). All additive.

## Decisions to lock in before Phase 1

1. **Plugin contract**: stdin = schema bytes, argv = options,
   stdout = generated file content, stderr + exit code for errors.
   No proto, no AST.
2. **Bazel outputs**: single file per rule invocation. Multi-output
   needs split into separate targets. Tree-artifact escape hatch
   for genuine many-file plugins.
3. **Plugin discovery**: toolchain types per output language
   (already in place).
4. **Repo naming**: stay `rules_jsonschema`.

## Phases

### Phase 1: nail down the contract in code

- `//jsonschema:plugin_contract.md` (or similar) — a concise
  written spec of stdin/argv/stdout/stderr the contract docs
  reference.
- Refit the existing Rust + Starlark codegen binaries to the new
  contract. `schema_to_rust` already mostly does this (it reads a
  path from `--schema`); switch to stdin and the standard argv
  flags.
- Update `//rust:defs.bzl` and `//starlark:defs.bzl` to invoke
  plugins via the contract.
- Existing rules_docker_compose tests should pass byte-identical.

### Phase 2: Go plugin (in Go)

- `tools/plugin_go/main.go` reads schema bytes from stdin, parses
  via `encoding/json`, emits Go types using `go/format`. Uses
  rules_go.
- `//go:defs.bzl` with `jsonschema_go_library`.
- Smoke example: person.json → Go types → round-trip decode test.

This validates the cross-language contract works as cleanly as the
RFC claims. If implementing the Go plugin is harder than the
"15 lines" pitch, the contract needs tightening.

### Phase 3: contract testing

A small integration-test rule that runs an arbitrary plugin against
a curated set of "interesting" schemas (compose-spec subset, edge
cases, malformed input) and asserts on stdout/stderr/exit behavior.
Lets plugin authors verify conformance before registering as a
toolchain.

### Phase 4: rules_docker_compose migration

Should be a no-op end-user-visibly — the codegen binaries still
exist, just invoked through the new contract. Tests pass
byte-identical.
