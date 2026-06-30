use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn oss_smoke_matrix_lists_expected_coverage() {
    let output = Command::new(repo_root().join("scripts/oss_smoke_matrix.sh"))
        .arg("--list")
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "matrix list failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        let parts = line.split('\t').collect::<Vec<_>>();
        assert_eq!(
            parts[2].len(),
            40,
            "target ref must be a full commit SHA: {line}"
        );
        assert!(parts[2].chars().all(|ch| ch.is_ascii_hexdigit()));
    }
    for expected in [
        "library",
        "binary",
        "workspace",
        "features",
        "tests",
        "doctests",
        "macros",
        "derives",
        "serde",
        "public-api",
    ] {
        assert!(stdout.contains(expected), "missing coverage: {expected}");
    }
}

#[test]
fn oss_smoke_workflow_is_manual_and_cache_friendly() {
    let workflow = fs::read_to_string(repo_root().join(".github/workflows/oss-smoke.yml")).unwrap();

    assert!(workflow.contains("workflow_dispatch"));
    assert!(workflow.contains("schedule"));
    assert!(workflow.contains("actions/cache"));
    assert!(workflow.contains("scripts/oss_smoke_matrix.sh"));
    assert!(workflow.contains("--target"));
}

#[test]
fn oss_smoke_runner_uses_isolated_run_contract() {
    let script = fs::read_to_string(repo_root().join("scripts/oss_smoke_matrix.sh")).unwrap();

    assert!(script.contains("RUNNER_TEMP"));
    assert!(script.contains("TMPDIR"));
    assert!(!script.contains("$ROOT/target/oss-smoke"));
    assert!(script.contains("nightmare.toml"));
    assert!(script.contains("run \"$config\" --json"));
    assert!(script.contains("assert_run_result"));
    assert!(script.contains("cargo test --all-targets && cargo test --doc"));
}

#[test]
fn oss_smoke_limitations_are_documented() {
    let docs = fs::read_to_string(repo_root().join("docs/oss-smoke-matrix.md")).unwrap();

    assert!(docs.contains("Expected Limitations"));
    assert!(docs.contains("network"));
    assert!(docs.contains("pinned"));
    assert!(docs.contains("public APIs"));
    assert!(docs.contains("NIGHTMARE_SMOKE_WORKDIR"));
    assert!(docs.contains("nightmare run --json"));
}
