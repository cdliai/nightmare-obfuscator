use std::fs;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn release_workflow_builds_tagged_artifacts() {
    let workflow = fs::read_to_string(repo_root().join(".github/workflows/release.yml")).unwrap();

    assert!(workflow.contains("on:"));
    assert!(workflow.contains("tags:"));
    assert!(workflow.contains("v*.*.*"));
    assert!(workflow.contains("Verify tag is on main"));
    assert!(workflow.contains("cargo test --locked --workspace"));
    assert!(workflow.contains("cargo build --locked --release"));
    assert!(workflow.contains("softprops/action-gh-release"));
    assert!(workflow.contains("ubuntu-latest"));
    assert!(workflow.contains("macos-latest"));
}

#[test]
fn branch_protection_policy_documents_main_and_dev_rules() {
    let policy = fs::read_to_string(repo_root().join("docs/branch-protection.md")).unwrap();

    assert!(policy.contains("main"));
    assert!(policy.contains("pull request review"));
    assert!(policy.contains("CI / rust"));
    assert!(policy.contains("dev"));
    assert!(policy.contains("requires CI"));
    assert!(policy.contains("does not require review"));
}

#[test]
fn codeowners_uses_valid_repository_owner() {
    let codeowners = fs::read_to_string(repo_root().join(".github/CODEOWNERS")).unwrap();

    assert!(codeowners.contains("@fbkaragoz"));
    assert!(!codeowners.contains("@CDLI-ai/maintainers"));
}
