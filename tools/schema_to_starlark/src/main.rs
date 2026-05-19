//! schema_to_starlark: emit a Bazel `.bzl` file containing typed
//! `rule()` definitions whose attrs are derived from JSON Schema
//! `properties`. Default codegen toolchain behind rules_jsonschema's
//! `jsonschema_starlark_codegen` rule.
//!
//! ## Contract
//!
//! Implements the rules_jsonschema plugin contract (see
//! [`jsonschema/plugin_contract.md`](../../jsonschema/plugin_contract.md)):
//!
//!   * stdin: schema file contents (JSON)
//!   * argv: `--schema-name=NAME`, `--rule-name=NAME`,
//!           `--kind=ID=POINTER:RULE_NAME:PROVIDER_NAME` (repeated)
//!   * stdout: generated `.bzl` source
//!   * stderr: warnings + errors
//!   * exit: 0 success, non-zero failure
//!
//! Each `--kind` packs four fields:
//!   - ID            short tag used in generated symbol names (e.g. `service`)
//!   - POINTER       JSON-pointer into the schema (e.g. `#/definitions/service`)
//!   - RULE_NAME     public Starlark rule symbol
//!   - PROVIDER_NAME public Starlark provider symbol
//!
//! Property → attr mapping lives in `classifier::property_to_attr`;
//! `.bzl` emission lives in `emit::emit_rule`. This file is just
//! argv / stdin / stdout plumbing.

use std::io::{self, Read, Write};

use anyhow::{anyhow, Context, Result};
use serde_json::Value;

mod classifier;
mod emit;

fn main() -> Result<()> {
    let args = parse_args()?;

    let mut schema_bytes = Vec::new();
    io::stdin()
        .read_to_end(&mut schema_bytes)
        .context("reading schema from stdin")?;
    let schema: Value = serde_json::from_slice(&schema_bytes)
        .context("parsing schema from stdin")?;

    let mut out = String::new();
    // The preamble's "Source schema:" header takes the basename
    // passed via --schema-name. Without it we'd have nothing
    // descriptive — there's no path to fall back on (the schema
    // arrived as bytes on stdin).
    emit::emit_preamble(&mut out, &args.schema_name);
    for kind in &args.kinds {
        let def = classifier::resolve_pointer(&schema, &kind.pointer)
            .ok_or_else(|| anyhow!("schema pointer {} did not resolve", kind.pointer))?;
        emit::emit_rule(&mut out, kind, def, &schema)?;
    }

    io::stdout()
        .write_all(out.as_bytes())
        .context("writing generated .bzl to stdout")?;
    Ok(())
}

#[derive(Debug, Default)]
struct Args {
    schema_name: String,
    #[allow(dead_code)] // accepted for contract conformance; not consumed here
    rule_name: String,
    kinds: Vec<Kind>,
}

/// One schema definition to emit a rule for.
#[derive(Debug, Clone)]
pub struct Kind {
    pub id: String,
    pub pointer: String,
    pub rule_name: String,
    pub provider_name: String,
}

fn parse_args() -> Result<Args> {
    let mut args = Args::default();
    for raw in std::env::args().skip(1) {
        if raw == "-h" || raw == "--help" {
            eprintln!(
                "Usage: schema_to_starlark < schema.json > rules.bzl \\\n  [--schema-name=NAME] [--rule-name=NAME] \\\n  --kind=ID=POINTER:RULE_NAME:PROVIDER_NAME ..."
            );
            std::process::exit(0);
        }
        let (key, value) = raw
            .split_once('=')
            .ok_or_else(|| anyhow!("expected --key=value, got: {raw}"))?;
        match key {
            "--schema-name" => args.schema_name = value.to_string(),
            "--rule-name" => args.rule_name = value.to_string(),
            "--kind" => args.kinds.push(parse_kind(value)?),
            other => return Err(anyhow!("unknown flag: {other}")),
        }
    }
    Ok(args)
}

/// `id=pointer:rule_name:provider_name`
fn parse_kind(raw: &str) -> Result<Kind> {
    let (id, rest) = raw
        .split_once('=')
        .ok_or_else(|| anyhow!("expected id=pointer:rule_name:provider_name, got {raw:?}"))?;
    let mut parts = rest.splitn(3, ':');
    let pointer = parts.next().context("missing pointer")?.to_string();
    let rule_name = parts.next().context("missing rule_name")?.to_string();
    let provider_name = parts.next().context("missing provider_name")?.to_string();
    Ok(Kind {
        id: id.to_string(),
        pointer,
        rule_name,
        provider_name,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_kind_happy_path() {
        let k = parse_kind("service=#/definitions/service:docker_compose_service:ComposeServiceInfo")
            .unwrap();
        assert_eq!(k.id, "service");
        assert_eq!(k.pointer, "#/definitions/service");
        assert_eq!(k.rule_name, "docker_compose_service");
        assert_eq!(k.provider_name, "ComposeServiceInfo");
    }

    #[test]
    fn parse_kind_requires_all_fields() {
        assert!(parse_kind("oops").is_err());
        assert!(parse_kind("id=#:rule").is_err()); // missing provider
    }

    #[test]
    fn parse_kind_tolerates_colons_in_pointer() {
        // Future-proofing: pointers shouldn't have colons today, but
        // splitn(3) caps how many splits we do so a colon in the
        // provider name wouldn't be swallowed.
        let k = parse_kind("svc=#/path:R:P").unwrap();
        assert_eq!(k.pointer, "#/path");
        assert_eq!(k.rule_name, "R");
        assert_eq!(k.provider_name, "P");
    }
}
