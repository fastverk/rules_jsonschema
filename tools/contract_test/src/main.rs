//! contract_test_driver: exercises the rules_jsonschema plugin
//! contract against an arbitrary plugin binary. Used by the
//! `jsonschema_plugin_contract_test` rule so plugin authors can
//! verify their plugin behaves correctly without depending on any
//! specific output language.
//!
//! The scenarios test load-bearing contract properties:
//!
//!   1. Minimum-viable invocation succeeds with non-empty stdout.
//!   2. Malformed JSON input → non-zero exit, stderr explanation,
//!      **empty stdout** (the discipline most likely to be broken).
//!   3. Unknown flags are rejected (per contract).
//!   4. Output is deterministic across identical invocations.
//!
//! Usage: `contract_test_driver <plugin-binary>`

use std::io::Write;
use std::process::{Command, ExitStatus, Stdio};

fn main() {
    let plugin = std::env::args().nth(1).expect("usage: contract_test_driver <plugin-path>");

    let scenarios: &[(&str, fn(&str) -> Result<(), String>)] = &[
        ("valid_minimal", scenario_valid_minimal),
        ("malformed_input", scenario_malformed_input),
        ("unknown_flag", scenario_unknown_flag),
        ("determinism", scenario_determinism),
    ];

    let mut failures = Vec::new();
    for (name, run) in scenarios {
        match run(&plugin) {
            Ok(()) => println!("PASS: {name}"),
            Err(e) => {
                println!("FAIL: {name}: {e}");
                failures.push(*name);
            }
        }
    }

    if !failures.is_empty() {
        eprintln!(
            "\ncontract_test: {} scenario(s) failed: {}",
            failures.len(),
            failures.join(", "),
        );
        std::process::exit(1);
    }
    println!("\nall {} scenarios passed", scenarios.len());
}

struct InvokeResult {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    status: ExitStatus,
}

fn invoke(plugin: &str, args: &[&str], stdin_bytes: &[u8]) -> std::io::Result<InvokeResult> {
    let mut child = Command::new(plugin)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(stdin_bytes)?;
        // Drop stdin to send EOF — without this, plugins reading to
        // EOF hang forever.
    }
    let out = child.wait_with_output()?;
    Ok(InvokeResult {
        stdout: out.stdout,
        stderr: out.stderr,
        status: out.status,
    })
}

// --- scenarios -------------------------------------------------------

fn scenario_valid_minimal(plugin: &str) -> Result<(), String> {
    let r = invoke(
        plugin,
        &["--schema-name=test.json", "--rule-name=test"],
        br#"{"type":"object","title":"Test","properties":{}}"#,
    )
    .map_err(|e| format!("spawn failed: {e}"))?;

    if !r.status.success() {
        return Err(format!(
            "expected exit 0, got {:?}; stderr was: {}",
            r.status,
            String::from_utf8_lossy(&r.stderr),
        ));
    }
    if r.stdout.is_empty() {
        return Err("plugin produced empty stdout on a valid input".into());
    }
    Ok(())
}

fn scenario_malformed_input(plugin: &str) -> Result<(), String> {
    let r = invoke(
        plugin,
        &["--schema-name=test.json", "--rule-name=test"],
        b"this is not valid json {{ ::: ",
    )
    .map_err(|e| format!("spawn failed: {e}"))?;

    if r.status.success() {
        return Err("plugin exited 0 on garbage input; contract says non-zero".into());
    }
    if !r.stdout.is_empty() {
        return Err(format!(
            "stdout must be empty on error; got {} bytes:\n{}",
            r.stdout.len(),
            String::from_utf8_lossy(&r.stdout),
        ));
    }
    if r.stderr.is_empty() {
        return Err("plugin failed silently; stderr should explain why".into());
    }
    Ok(())
}

fn scenario_unknown_flag(plugin: &str) -> Result<(), String> {
    let r = invoke(
        plugin,
        &[
            "--schema-name=test.json",
            "--rule-name=test",
            "--this-flag-does-not-exist=value",
        ],
        br#"{"type":"object","properties":{}}"#,
    )
    .map_err(|e| format!("spawn failed: {e}"))?;

    if r.status.success() {
        return Err(
            "plugin accepted an unknown flag; contract says reject unknown flags".into(),
        );
    }
    Ok(())
}

fn scenario_determinism(plugin: &str) -> Result<(), String> {
    let args = ["--schema-name=det.json", "--rule-name=det"];
    let body = br#"{
        "type":"object",
        "title":"Det",
        "properties":{
            "z":{"type":"string"},
            "a":{"type":"integer"},
            "m":{"type":"boolean"}
        }
    }"#;
    let r1 = invoke(plugin, &args, body).map_err(|e| format!("spawn #1: {e}"))?;
    let r2 = invoke(plugin, &args, body).map_err(|e| format!("spawn #2: {e}"))?;

    if !r1.status.success() {
        return Err(format!(
            "setup invocation failed: {}",
            String::from_utf8_lossy(&r1.stderr),
        ));
    }
    if r1.stdout != r2.stdout {
        return Err(format!(
            "plugin output differs between identical invocations\n--- run 1 ---\n{}\n--- run 2 ---\n{}",
            String::from_utf8_lossy(&r1.stdout),
            String::from_utf8_lossy(&r2.stdout),
        ));
    }
    Ok(())
}
