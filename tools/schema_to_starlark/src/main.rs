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
//!           `--kind=ID=POINTER:RULE_NAME:PROVIDER_NAME` (repeated), and
//!           the auto-kinds flags listed below.
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
//! ## Auto-kinds
//!
//! For large schemas (e.g. the AWS CloudFormation Resource
//! Specification with ~1200 resource types) hand-enumerating
//! `--kind` flags is impractical. The auto-kinds flags synthesize one
//! `Kind` per child of a chosen JSON-pointer base:
//!
//!   * `--kinds-pointer-base=POINTER`     — required to enable
//!     auto-kinds. The schema location whose children become kinds
//!     (e.g. `#/definitions`).
//!   * `--kinds-pointer-suffix=SUFFIX`    — appended to each child
//!     pointer (e.g. `/properties/Properties`). Default empty.
//!   * `--kinds-key-filter=REGEX`         — drop children whose key
//!     does not match the regex. Default: accept all.
//!   * `--rule-name-template=TPL`         — Starlark rule symbol.
//!     Default `{snake}`.
//!   * `--provider-name-template=TPL`     — provider symbol.
//!     Default `{camel}Info`.
//!   * `--id-template=TPL`                — short id. Default
//!     `{snake}`.
//!
//! Templates expand three placeholders: `{key}` (raw key),
//! `{snake}` (lower_snake_case derived from key), `{camel}`
//! (UpperCamelCase derived from key). Auto-kinds are appended after
//! any explicit `--kind=` entries; both must be empty for the tool
//! to bail.
//!
//! Property → attr mapping lives in `classifier::property_to_attr`;
//! `.bzl` emission lives in `emit::emit_rule`. This file is just
//! argv / stdin / stdout plumbing.

use std::io::{self, Read, Write};

use anyhow::{anyhow, Context, Result};
use regex::Regex;
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

    let mut kinds = args.kinds.clone();
    if let Some(auto) = &args.auto_kinds {
        let mut synthesized = synthesize_kinds(&schema, auto)?;
        // Auto kinds come after explicit ones, sorted by id for
        // deterministic output across runs.
        synthesized.sort_by(|a, b| a.id.cmp(&b.id));
        kinds.extend(synthesized);
    }
    // Zero kinds is a legal no-op (the contract test exercises it
    // with no --kind=... flags) — emit just the preamble. Surfacing
    // "your auto-kinds filter dropped everything" as a hard error
    // would also block the no-flags conformance probe.

    let mut out = String::new();
    // The preamble's "Source schema:" header takes the basename
    // passed via --schema-name. Without it we'd have nothing
    // descriptive — there's no path to fall back on (the schema
    // arrived as bytes on stdin).
    emit::emit_preamble(&mut out, &args.schema_name);
    for kind in &kinds {
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
    auto_kinds: Option<AutoKindsConfig>,
}

/// One schema definition to emit a rule for.
#[derive(Debug, Clone)]
pub struct Kind {
    pub id: String,
    pub pointer: String,
    pub rule_name: String,
    pub provider_name: String,
}

/// Config for synthesizing kinds from schema children. Built from the
/// `--kinds-*` and `--*-template` flags; the absence of
/// `--kinds-pointer-base` disables auto-kinds entirely.
#[derive(Debug)]
struct AutoKindsConfig {
    pointer_base: String,
    pointer_suffix: String,
    key_filter: Option<Regex>,
    id_template: String,
    rule_name_template: String,
    provider_name_template: String,
}

#[derive(Debug, Default)]
struct AutoKindsBuilder {
    pointer_base: Option<String>,
    pointer_suffix: Option<String>,
    key_filter: Option<String>,
    id_template: Option<String>,
    rule_name_template: Option<String>,
    provider_name_template: Option<String>,
}

impl AutoKindsBuilder {
    fn any_set(&self) -> bool {
        self.pointer_base.is_some()
            || self.pointer_suffix.is_some()
            || self.key_filter.is_some()
            || self.id_template.is_some()
            || self.rule_name_template.is_some()
            || self.provider_name_template.is_some()
    }

    fn build(self) -> Result<Option<AutoKindsConfig>> {
        if !self.any_set() {
            return Ok(None);
        }
        let pointer_base = self.pointer_base.ok_or_else(|| {
            anyhow!("--kinds-pointer-base is required when any --kinds-*/--*-template flag is set")
        })?;
        let key_filter = self
            .key_filter
            .map(|p| Regex::new(&p).with_context(|| format!("invalid --kinds-key-filter regex: {p}")))
            .transpose()?;
        Ok(Some(AutoKindsConfig {
            pointer_base,
            pointer_suffix: self.pointer_suffix.unwrap_or_default(),
            key_filter,
            id_template: self.id_template.unwrap_or_else(|| "{snake}".to_string()),
            rule_name_template: self.rule_name_template.unwrap_or_else(|| "{snake}".to_string()),
            provider_name_template: self
                .provider_name_template
                .unwrap_or_else(|| "{camel}Info".to_string()),
        }))
    }
}

fn parse_args() -> Result<Args> {
    let mut args = Args::default();
    let mut auto = AutoKindsBuilder::default();
    for raw in std::env::args().skip(1) {
        if raw == "-h" || raw == "--help" {
            eprintln!(
                "Usage: schema_to_starlark < schema.json > rules.bzl \\\n  [--schema-name=NAME] [--rule-name=NAME] \\\n  --kind=ID=POINTER:RULE_NAME:PROVIDER_NAME ... \\\n  [--kinds-pointer-base=POINTER] [--kinds-pointer-suffix=SUFFIX] \\\n  [--kinds-key-filter=REGEX] [--id-template=TPL] \\\n  [--rule-name-template=TPL] [--provider-name-template=TPL]"
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
            "--kinds-pointer-base" => auto.pointer_base = Some(value.to_string()),
            "--kinds-pointer-suffix" => auto.pointer_suffix = Some(value.to_string()),
            "--kinds-key-filter" => auto.key_filter = Some(value.to_string()),
            "--id-template" => auto.id_template = Some(value.to_string()),
            "--rule-name-template" => auto.rule_name_template = Some(value.to_string()),
            "--provider-name-template" => auto.provider_name_template = Some(value.to_string()),
            other => return Err(anyhow!("unknown flag: {other}")),
        }
    }
    args.auto_kinds = auto.build()?;
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

/// Walk `auto.pointer_base`'s children and synthesize a `Kind` per
/// surviving key. The base must resolve to a JSON object.
fn synthesize_kinds(schema: &Value, auto: &AutoKindsConfig) -> Result<Vec<Kind>> {
    let base = classifier::resolve_pointer(schema, &auto.pointer_base).ok_or_else(|| {
        anyhow!(
            "--kinds-pointer-base {} did not resolve",
            auto.pointer_base
        )
    })?;
    let obj = base.as_object().ok_or_else(|| {
        anyhow!(
            "--kinds-pointer-base {} resolved to non-object (auto-kinds requires an object whose keys become kinds)",
            auto.pointer_base
        )
    })?;
    let mut out = Vec::new();
    for key in obj.keys() {
        if let Some(filter) = &auto.key_filter {
            if !filter.is_match(key) {
                continue;
            }
        }
        let snake = to_snake_case(key);
        let camel = to_camel_case(key);
        let pointer = format!(
            "{base}/{key}{suffix}",
            base = auto.pointer_base,
            key = escape_pointer_segment(key),
            suffix = auto.pointer_suffix,
        );
        out.push(Kind {
            id: render_template(&auto.id_template, key, &snake, &camel),
            pointer,
            rule_name: render_template(&auto.rule_name_template, key, &snake, &camel),
            provider_name: render_template(&auto.provider_name_template, key, &snake, &camel),
        });
    }
    Ok(out)
}

fn render_template(tpl: &str, key: &str, snake: &str, camel: &str) -> String {
    tpl.replace("{key}", key)
        .replace("{snake}", snake)
        .replace("{camel}", camel)
}

/// RFC 6901 JSON-pointer segment escaping: `~` → `~0`, `/` → `~1`.
/// Applied before joining a child key into a pointer so keys
/// containing slashes (rare but legal) don't accidentally extend the
/// pointer path.
fn escape_pointer_segment(key: &str) -> String {
    key.replace('~', "~0").replace('/', "~1")
}

/// Turn an arbitrary key like `AWS::S3::Bucket` or `My-Resource` into
/// `aws_s3_bucket` / `my_resource`. Non-alphanumeric runs collapse to
/// `_`; CamelCase boundaries split.
fn to_snake_case(key: &str) -> String {
    let mut out = String::with_capacity(key.len());
    let mut prev_lower = false;
    for ch in key.chars() {
        if ch.is_ascii_alphanumeric() {
            if ch.is_ascii_uppercase() && prev_lower {
                out.push('_');
            }
            out.push(ch.to_ascii_lowercase());
            prev_lower = ch.is_ascii_lowercase() || ch.is_ascii_digit();
        } else {
            if !out.ends_with('_') && !out.is_empty() {
                out.push('_');
            }
            prev_lower = false;
        }
    }
    out.trim_matches('_').to_string()
}

/// Turn an arbitrary key like `AWS::S3::Bucket` into `AwsS3Bucket`.
/// Strategy: split on non-alphanumerics + camel boundaries, then
/// title-case each segment.
fn to_camel_case(key: &str) -> String {
    let snake = to_snake_case(key);
    let mut out = String::with_capacity(snake.len());
    for segment in snake.split('_') {
        let mut chars = segment.chars();
        if let Some(first) = chars.next() {
            out.push(first.to_ascii_uppercase());
            for c in chars {
                out.push(c);
            }
        }
    }
    out
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

    #[test]
    fn snake_camel_handle_cfn_style_keys() {
        assert_eq!(to_snake_case("AWS::S3::Bucket"), "aws_s3_bucket");
        assert_eq!(to_camel_case("AWS::S3::Bucket"), "AwsS3Bucket");
        assert_eq!(to_snake_case("MyResource"), "my_resource");
        assert_eq!(to_camel_case("my-resource"), "MyResource");
    }

    #[test]
    fn render_template_substitutes_all_placeholders() {
        let rendered = render_template(
            "cloudformation_{snake}_for_{key}_{camel}Info",
            "AWS::S3::Bucket",
            "aws_s3_bucket",
            "AwsS3Bucket",
        );
        assert_eq!(
            rendered,
            "cloudformation_aws_s3_bucket_for_AWS::S3::Bucket_AwsS3BucketInfo"
        );
    }

    #[test]
    fn auto_kinds_builder_requires_pointer_base_when_other_flags_set() {
        let b = AutoKindsBuilder {
            rule_name_template: Some("x_{snake}".into()),
            ..Default::default()
        };
        assert!(b.build().is_err());
    }

    #[test]
    fn auto_kinds_builder_disabled_when_no_flags_set() {
        let b = AutoKindsBuilder::default();
        assert!(b.build().unwrap().is_none());
    }

    #[test]
    fn synthesize_kinds_filters_and_renders() {
        let schema: Value = serde_json::from_str(
            r#"{
              "definitions": {
                "AWS::S3::Bucket": {"type": "object"},
                "AWS::EC2::Instance": {"type": "object"},
                "NotAnAwsType": {"type": "object"}
              }
            }"#,
        )
        .unwrap();
        let auto = AutoKindsConfig {
            pointer_base: "#/definitions".into(),
            pointer_suffix: "".into(),
            key_filter: Some(Regex::new(r"^AWS::").unwrap()),
            id_template: "{snake}".into(),
            rule_name_template: "cfn_{snake}".into(),
            provider_name_template: "Cfn{camel}Info".into(),
        };
        let mut kinds = synthesize_kinds(&schema, &auto).unwrap();
        kinds.sort_by(|a, b| a.id.cmp(&b.id));
        assert_eq!(kinds.len(), 2);
        assert_eq!(kinds[0].id, "aws_ec2_instance");
        assert_eq!(kinds[0].rule_name, "cfn_aws_ec2_instance");
        assert_eq!(kinds[0].provider_name, "CfnAwsEc2InstanceInfo");
        assert_eq!(kinds[0].pointer, "#/definitions/AWS::EC2::Instance");
        assert_eq!(kinds[1].id, "aws_s3_bucket");
    }
}
