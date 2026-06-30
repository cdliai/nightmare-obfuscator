use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_nightmare")
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn temp_root(name: &str) -> PathBuf {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("nightmare-{name}-{}-{now}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    root
}

fn copy_dir(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let source = entry.path();
        let dest = to.join(entry.file_name());
        if source.is_dir() {
            copy_dir(&source, &dest);
        } else {
            fs::copy(&source, &dest).unwrap();
        }
    }
}

fn run_ok(command: &mut Command) {
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "command failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn output_fail(command: &mut Command) -> std::process::Output {
    let output = command.output().unwrap();
    assert!(
        !output.status.success(),
        "command unexpectedly succeeded\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn output_ok(command: &mut Command) -> std::process::Output {
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "command failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn write_contract(path: &Path, source: &Path, output: &Path) {
    fs::write(
        path,
        format!(
            r#"schema_version = 1
source = "{}"
output = "{}"
profile = "balanced"
intensity = 7
selected_paths = ["src/lib.rs"]
ignored_patterns = ["snapshots"]

[owner]
name = "CDLI"
contact = "security@example.com"

[project]
name = "fixture"

[checks]
verify_metadata = true
build = "cargo test"
"#,
            source.display(),
            output.display()
        ),
    )
    .unwrap();
}

fn write_contract_with_signing_key(path: &Path, source: &Path, output: &Path, key: &Path) {
    write_contract(path, source, output);
    let mut text = fs::read_to_string(path).unwrap();
    text.push_str(&format!(
        "\n[signing]\nprivate_key_path = \"{}\"\n",
        key.display()
    ));
    fs::write(path, text).unwrap();
}

fn write_signing_key(path: &Path, byte: u8) {
    let encoded = match byte {
        7 => "BwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwc=",
        9 => "CQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQk=",
        _ => panic!("unsupported test key byte"),
    };
    fs::write(path, encoded).unwrap();
}

fn read_toml(path: &Path) -> toml::Value {
    fs::read_to_string(path)
        .unwrap()
        .parse::<toml::Value>()
        .unwrap()
}

fn canonical_string(path: impl AsRef<Path>) -> String {
    path.as_ref()
        .canonicalize()
        .unwrap()
        .to_string_lossy()
        .to_string()
}

#[test]
fn original_rust_fixture_builds() {
    let root = temp_root("original");
    let input = root.join("rust-basic");
    copy_dir(&repo_root().join("fixtures/rust-basic"), &input);

    run_ok(Command::new("cargo").arg("test").current_dir(&input));
}

#[test]
fn default_output_obfuscates_full_rust_project_and_verifies() {
    let root = temp_root("full");
    let input = root.join("rust-basic");
    copy_dir(&repo_root().join("fixtures/rust-basic"), &input);

    run_ok(Command::new(bin()).arg("obfuscate").arg(&input));

    let output = root.join("rust-basic-obfs");
    assert!(output.join("Cargo.toml").exists());
    assert!(output.join(".nightmare/manifest.json").exists());
    assert!(output.join(".nightmare/signature").exists());

    let source = fs::read_to_string(input.join("src/lib.rs")).unwrap();
    let obfuscated = fs::read_to_string(output.join("src/lib.rs")).unwrap();
    assert_ne!(source, obfuscated);
    assert!(obfuscated.contains("local_total should stay inside this string"));
    assert!(!obfuscated.contains("let local_total"));

    run_ok(Command::new("cargo").arg("test").current_dir(&output));
    run_ok(Command::new(bin()).arg("verify").arg(&output));
}

#[test]
fn selected_obfuscation_copies_unselected_files_byte_identical() {
    let root = temp_root("selected");
    let input = root.join("rust-selected");
    let output = root.join("selected-output");
    copy_dir(&repo_root().join("fixtures/rust-selected"), &input);

    run_ok(
        Command::new(bin())
            .arg("obfuscate")
            .arg(&input)
            .arg("--output")
            .arg(&output)
            .arg("--select")
            .arg("src/x.rs"),
    );

    assert_ne!(
        fs::read(input.join("src/x.rs")).unwrap(),
        fs::read(output.join("src/x.rs")).unwrap()
    );
    for rel in ["src/y.rs", "src/z.rs", "src/t.rs", "Cargo.toml"] {
        assert_eq!(
            fs::read(input.join(rel)).unwrap(),
            fs::read(output.join(rel)).unwrap(),
            "{rel} should be copied without changes"
        );
    }

    run_ok(Command::new("cargo").arg("test").current_dir(&output));
    run_ok(Command::new(bin()).arg("verify").arg(&output));
}

#[test]
fn default_ignores_skip_dot_git_without_dropping_dot_github() {
    let root = temp_root("dot-github");
    let input = root.join("rust-basic");
    let output = root.join("dot-github-output");
    copy_dir(&repo_root().join("fixtures/rust-basic"), &input);
    fs::create_dir_all(input.join(".git")).unwrap();
    fs::write(input.join(".git/config"), "private git metadata").unwrap();
    fs::create_dir_all(input.join(".github/workflows")).unwrap();
    fs::write(input.join(".github/CODEOWNERS"), "* @fbkaragoz").unwrap();
    fs::write(input.join(".github/workflows/ci.yml"), "name: ci").unwrap();

    run_ok(
        Command::new(bin())
            .arg("obfuscate")
            .arg(&input)
            .arg("--output")
            .arg(&output),
    );

    assert!(!output.join(".git/config").exists());
    assert!(output.join(".github/CODEOWNERS").exists());
    assert!(output.join(".github/workflows/ci.yml").exists());
}

#[cfg(unix)]
#[test]
fn obfuscation_preserves_executable_permissions_for_scripts() {
    use std::os::unix::fs::PermissionsExt;

    let root = temp_root("permissions");
    let input = root.join("rust-basic");
    let output = root.join("permissions-output");
    copy_dir(&repo_root().join("fixtures/rust-basic"), &input);
    fs::create_dir_all(input.join("scripts")).unwrap();
    let script = input.join("scripts/smoke.sh");
    fs::write(&script, "#!/bin/sh\nexit 0\n").unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();

    run_ok(
        Command::new(bin())
            .arg("obfuscate")
            .arg(&input)
            .arg("--output")
            .arg(&output),
    );

    let mode = fs::metadata(output.join("scripts/smoke.sh"))
        .unwrap()
        .permissions()
        .mode();
    assert_eq!(mode & 0o111, 0o111);
}

#[test]
fn string_encryption_is_opt_in_std_only_and_build_preserving() {
    let root = temp_root("string-encrypt");
    let input = root.join("rust-string-encrypt");
    let output = root.join("string-encrypt-output");
    let config = root.join("nightmare.toml");
    copy_dir(&repo_root().join("fixtures/rust-string-encrypt"), &input);
    write_contract(&config, &input, &output);
    let mut text = fs::read_to_string(&config).unwrap();
    text.push_str("\n[features]\nencrypt_strings = true\nrename_identifiers = false\ndead_code = false\nflatten_control_flow = false\n");
    fs::write(&config, text).unwrap();

    run_ok(Command::new("cargo").arg("test").current_dir(&input));
    run_ok(Command::new(bin()).arg("run").arg(&config));

    let obfuscated = fs::read_to_string(output.join("src/lib.rs")).unwrap();
    assert!(!obfuscated.contains("RUNTIME_SECRET_DISAPPEARS"));
    assert!(obfuscated.contains("CONST_LITERAL_STAYS"));
    assert!(obfuscated.contains("STATIC_LITERAL_STAYS"));
    assert!(obfuscated.contains("MACRO_LITERAL_STAYS"));
    assert!(obfuscated.contains("PATTERN_LITERAL_STAYS"));
    assert!(!obfuscated.contains("base64"));
    assert!(!fs::read_to_string(output.join("Cargo.toml"))
        .unwrap()
        .contains("base64"));

    run_ok(Command::new("cargo").arg("test").current_dir(&output));
    run_ok(Command::new(bin()).arg("verify").arg(&output));
}

#[test]
fn string_encryption_remains_disabled_by_default() {
    let root = temp_root("string-default");
    let input = root.join("rust-string-encrypt");
    let output = root.join("string-default-output");
    copy_dir(&repo_root().join("fixtures/rust-string-encrypt"), &input);

    run_ok(
        Command::new(bin())
            .arg("obfuscate")
            .arg(&input)
            .arg("--output")
            .arg(&output),
    );

    let obfuscated = fs::read_to_string(output.join("src/lib.rs")).unwrap();
    assert!(obfuscated.contains("RUNTIME_SECRET_DISAPPEARS"));
}

#[test]
fn verify_fails_after_tampering() {
    let root = temp_root("tamper");
    let input = root.join("rust-basic");
    let output = root.join("tampered-output");
    copy_dir(&repo_root().join("fixtures/rust-basic"), &input);

    run_ok(
        Command::new(bin())
            .arg("obfuscate")
            .arg(&input)
            .arg("--output")
            .arg(&output),
    );
    fs::write(output.join("README.txt"), "tampered").unwrap();
    let lib = output.join("src/lib.rs");
    let mut content = fs::read_to_string(&lib).unwrap();
    content.push_str("\n// tamper\n");
    fs::write(&lib, content).unwrap();

    let verify = output_fail(Command::new(bin()).arg("verify").arg(&output));
    let stdout = String::from_utf8_lossy(&verify.stdout);
    let stderr = String::from_utf8_lossy(&verify.stderr);
    assert!(
        stdout.contains("checksum mismatch src/lib.rs")
            || stderr.contains("checksum mismatch src/lib.rs"),
        "verify output should include failed file details\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
}

#[test]
fn verify_rejects_wrong_trusted_public_key() {
    let root = temp_root("trusted-key");
    let input = root.join("rust-basic");
    let output = root.join("trusted-key-output");
    let config = root.join("nightmare.toml");
    let owner_key = root.join("owner.key");
    let other_key = root.join("other.key");
    copy_dir(&repo_root().join("fixtures/rust-basic"), &input);
    write_signing_key(&owner_key, 7);
    write_signing_key(&other_key, 9);
    write_contract_with_signing_key(&config, &input, &output, &owner_key);

    run_ok(Command::new(bin()).arg("run").arg(&config));
    let other_public = output_ok(
        Command::new(bin())
            .arg("signing")
            .arg("public-key")
            .arg("--signing-key")
            .arg(&other_key),
    );
    let other_public = String::from_utf8_lossy(&other_public.stdout)
        .trim()
        .to_string();

    let verify = output_fail(
        Command::new(bin())
            .arg("verify")
            .arg(&output)
            .arg("--trusted-public-key")
            .arg(other_public),
    );
    assert!(String::from_utf8_lossy(&verify.stderr).contains("trusted public key mismatch"));
}

#[test]
fn manifest_metadata_can_be_resigned_with_owner_key() {
    let root = temp_root("resign");
    let input = root.join("rust-basic");
    let output = root.join("resign-output");
    let config = root.join("nightmare.toml");
    let owner_key = root.join("owner.key");
    copy_dir(&repo_root().join("fixtures/rust-basic"), &input);
    write_signing_key(&owner_key, 7);
    write_contract_with_signing_key(&config, &input, &output, &owner_key);

    run_ok(Command::new(bin()).arg("run").arg(&config));

    let manifest_path = output.join(".nightmare/manifest.json");
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
    manifest["owner"]["contact"] = serde_json::json!("updated@example.com");
    fs::write(
        &manifest_path,
        serde_json::to_vec_pretty(&manifest).unwrap(),
    )
    .unwrap();

    output_fail(Command::new(bin()).arg("verify").arg(&output));
    run_ok(
        Command::new(bin())
            .arg("signing")
            .arg("sign-manifest")
            .arg(&output)
            .arg("--signing-key")
            .arg(&owner_key),
    );
    run_ok(Command::new(bin()).arg("verify").arg(&output));
}

#[test]
fn init_writes_versioned_contract_and_run_emits_clean_json_stages() {
    let root = temp_root("contract-run");
    let input = root.join("rust-basic");
    let output = root.join("contract-output");
    let config = root.join("nightmare.toml");
    copy_dir(&repo_root().join("fixtures/rust-basic"), &input);

    let init = output_ok(
        Command::new(bin())
            .arg("init")
            .arg("--config")
            .arg(&config)
            .arg("--source")
            .arg(&input)
            .arg("--output")
            .arg(&output)
            .arg("--owner")
            .arg("CDLI")
            .arg("--project")
            .arg("fixture")
            .arg("--select")
            .arg("src/lib.rs")
            .arg("--ignore")
            .arg("snapshots")
            .arg("--yes")
            .arg("--json"),
    );
    let init_json: serde_json::Value = serde_json::from_slice(&init.stdout).unwrap();
    assert_eq!(init_json["schema_version"], 1);
    assert_eq!(
        init_json["config_path"].as_str().unwrap(),
        config.to_string_lossy()
    );
    assert!(config.exists());

    let config_text = fs::read_to_string(&config).unwrap();
    assert!(config_text.contains("schema_version = 1"));
    assert!(config_text.contains("[owner]"));
    assert!(config_text.contains("[checks]"));

    let run = output_ok(Command::new(bin()).arg("run").arg(&config).arg("--json"));
    let result: serde_json::Value = serde_json::from_slice(&run.stdout).unwrap();
    assert_eq!(result["schema_version"], 1);
    assert_eq!(result["status"], "passed");
    assert_eq!(
        canonical_string(PathBuf::from(result["source"].as_str().unwrap())),
        canonical_string(&input)
    );
    assert_eq!(
        canonical_string(PathBuf::from(result["output"].as_str().unwrap())),
        canonical_string(&output)
    );
    assert!(output.join(".nightmare/manifest.json").exists());

    let stages = result["stages"].as_array().unwrap();
    assert_eq!(stages[0]["name"], "obfuscate");
    assert_eq!(stages[0]["status"], "passed");
    assert_eq!(stages[1]["name"], "verify");
    assert_eq!(stages[1]["status"], "passed");
    assert_eq!(stages[2]["name"], "build");
    assert_eq!(stages[2]["status"], "passed");
}

#[test]
fn obfuscate_config_is_canonical_and_cli_flags_override_it() {
    let root = temp_root("config-precedence");
    let input = root.join("rust-basic");
    let config_output = root.join("config-output");
    let cli_output = root.join("cli-output");
    let config = root.join("nightmare.toml");
    copy_dir(&repo_root().join("fixtures/rust-basic"), &input);
    write_contract(&config, &input, &config_output);

    run_ok(
        Command::new(bin())
            .arg("obfuscate")
            .arg(&input)
            .arg("--config")
            .arg(&config)
            .arg("--output")
            .arg(&cli_output)
            .arg("--select")
            .arg("src/main.rs")
            .arg("--intensity")
            .arg("9")
            .arg("--ignore")
            .arg("extra-ignore"),
    );

    assert!(!config_output.exists());
    assert!(cli_output.join(".nightmare/manifest.json").exists());
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(cli_output.join(".nightmare/manifest.json")).unwrap())
            .unwrap();
    assert_eq!(manifest["owner"]["name"], "CDLI");
    assert_eq!(manifest["project"]["name"], "fixture");
    assert_eq!(
        manifest["selected_paths"],
        serde_json::json!(["src/main.rs"])
    );
    assert!(manifest["ignored_patterns"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item == "extra-ignore"));
    assert!(fs::read_to_string(cli_output.join("src/lib.rs"))
        .unwrap()
        .contains("local_total should stay inside this string"));
    assert_ne!(
        fs::read(input.join("src/main.rs")).unwrap(),
        fs::read(cli_output.join("src/main.rs")).unwrap()
    );
}

#[test]
fn gate_github_json_records_immutable_ref_and_stage_boundaries() {
    let root = temp_root("gate");
    let input = root.join("rust-basic");
    let output = root.join("gate-output");
    let config = root.join("nightmare.toml");
    copy_dir(&repo_root().join("fixtures/rust-basic"), &input);
    write_contract(&config, &input, &output);

    let gate = output_ok(
        Command::new(bin())
            .arg("gate")
            .arg("github")
            .arg("--repo")
            .arg("https://github.com/cdliai/nightmare-obfuscator")
            .arg("--ref")
            .arg("0123456789abcdef0123456789abcdef01234567")
            .arg("--config")
            .arg(&config)
            .arg("--json"),
    );
    let result: serde_json::Value = serde_json::from_slice(&gate.stdout).unwrap();
    assert_eq!(result["status"], "planned");
    assert_eq!(
        result["gate"]["repo"],
        "https://github.com/cdliai/nightmare-obfuscator"
    );
    assert_eq!(
        result["gate"]["ref"],
        "0123456789abcdef0123456789abcdef01234567"
    );

    let stages = result["stages"].as_array().unwrap();
    assert_eq!(stages[0]["name"], "fetch");
    assert_eq!(stages[1]["name"], "obfuscate");
    assert_eq!(stages[2]["name"], "verify");
    assert_eq!(stages[3]["name"], "build");
    assert!(stages.iter().all(|stage| stage["status"] == "skipped"));
}

#[test]
fn gate_plan_reflects_verify_and_build_policy_from_contract() {
    let root = temp_root("gate-policy");
    let input = root.join("rust-basic");
    let output = root.join("gate-policy-output");
    let config = root.join("nightmare.toml");
    copy_dir(&repo_root().join("fixtures/rust-basic"), &input);
    write_contract(&config, &input, &output);
    let config_text = fs::read_to_string(&config)
        .unwrap()
        .replace("verify_metadata = true", "verify_metadata = false")
        .replace("build = \"cargo test\"", "build = \"cargo check --locked\"");
    fs::write(&config, config_text).unwrap();

    let gate = output_ok(
        Command::new(bin())
            .arg("gate")
            .arg("github")
            .arg("--repo")
            .arg("https://github.com/cdliai/nightmare-obfuscator")
            .arg("--ref")
            .arg("0123456789abcdef0123456789abcdef01234567")
            .arg("--config")
            .arg(&config)
            .arg("--json"),
    );
    let result: serde_json::Value = serde_json::from_slice(&gate.stdout).unwrap();
    let stages = result["stages"].as_array().unwrap();

    assert_eq!(stages[2]["name"], "verify");
    assert_eq!(stages[2]["status"], "skipped");
    assert!(stages[2]["stderr_summary"]
        .as_str()
        .unwrap()
        .contains("verify_metadata is false"));
    assert_eq!(stages[3]["name"], "build");
    assert_eq!(stages[3]["command"], "cargo check --locked");
}

#[test]
fn disabled_build_check_persists_and_gate_reports_skipped_build() {
    let root = temp_root("disabled-build");
    let input = root.join("rust-basic");
    let output = root.join("disabled-build-output");
    let config = root.join("nightmare.toml");
    copy_dir(&repo_root().join("fixtures/rust-basic"), &input);

    run_ok(
        Command::new(bin())
            .arg("init")
            .arg("--config")
            .arg(&config)
            .arg("--source")
            .arg(&input)
            .arg("--output")
            .arg(&output)
            .arg("--owner")
            .arg("CDLI")
            .arg("--project")
            .arg("fixture")
            .arg("--no-build-check")
            .arg("--yes")
            .arg("--json"),
    );

    let config_toml = read_toml(&config);
    assert_eq!(config_toml["checks"]["build"].as_bool(), Some(false));

    let gate = output_ok(
        Command::new(bin())
            .arg("gate")
            .arg("github")
            .arg("--repo")
            .arg("https://github.com/cdliai/nightmare-obfuscator")
            .arg("--ref")
            .arg("0123456789abcdef0123456789abcdef01234567")
            .arg("--config")
            .arg(&config)
            .arg("--json"),
    );
    let result: serde_json::Value = serde_json::from_slice(&gate.stdout).unwrap();
    let stages = result["stages"].as_array().unwrap();
    assert_eq!(stages[3]["name"], "build");
    assert_eq!(stages[3]["status"], "skipped");
    assert!(stages[3]["stderr_summary"]
        .as_str()
        .unwrap()
        .contains("disabled"));
}

#[test]
fn gate_json_reports_invalid_ref_as_structured_failure() {
    let root = temp_root("gate-invalid-ref");
    let input = root.join("rust-basic");
    let output = root.join("gate-invalid-ref-output");
    let config = root.join("nightmare.toml");
    copy_dir(&repo_root().join("fixtures/rust-basic"), &input);
    write_contract(&config, &input, &output);

    let gate = output_fail(
        Command::new(bin())
            .arg("gate")
            .arg("github")
            .arg("--repo")
            .arg("https://github.com/cdliai/nightmare-obfuscator")
            .arg("--ref")
            .arg("main")
            .arg("--config")
            .arg(&config)
            .arg("--json"),
    );
    let result: serde_json::Value = serde_json::from_slice(&gate.stdout).unwrap();
    assert_eq!(result["status"], "failed");
    assert_eq!(result["gate"]["ref"], "main");
    assert_eq!(result["stages"][0]["name"], "fetch");
    assert_eq!(result["stages"][0]["status"], "failed");
    assert!(result["stages"][0]["stderr_summary"]
        .as_str()
        .unwrap()
        .contains("immutable"));
}

#[test]
fn run_json_reports_failed_build_as_a_separate_stage() {
    let root = temp_root("failed-build");
    let input = root.join("rust-basic");
    let output = root.join("failed-build-output");
    let config = root.join("nightmare.toml");
    copy_dir(&repo_root().join("fixtures/rust-basic"), &input);
    write_contract(&config, &input, &output);
    let mut config_text = fs::read_to_string(&config).unwrap();
    config_text = config_text.replace(
        "build = \"cargo test\"",
        "build = \"cargo test --definitely-not-a-real-cargo-flag\"",
    );
    fs::write(&config, config_text).unwrap();

    let run = output_fail(Command::new(bin()).arg("run").arg(&config).arg("--json"));
    let result: serde_json::Value = serde_json::from_slice(&run.stdout).unwrap();
    assert_eq!(result["status"], "failed");

    let stages = result["stages"].as_array().unwrap();
    assert_eq!(stages[0]["name"], "obfuscate");
    assert_eq!(stages[0]["status"], "passed");
    assert_eq!(stages[1]["name"], "verify");
    assert_eq!(stages[1]["status"], "passed");
    assert_eq!(stages[2]["name"], "build");
    assert_eq!(stages[2]["status"], "failed");
    assert!(stages[2]["stderr_summary"]
        .as_str()
        .unwrap()
        .contains("definitely"));
}

#[test]
fn init_human_mode_uses_retro_banner_and_never_requires_a_tty() {
    let root = temp_root("init-human");
    let config = root.join("nightmare.toml");

    let init = output_ok(
        Command::new(bin())
            .arg("init")
            .arg("--config")
            .arg(&config)
            .arg("--source")
            .arg(".")
            .arg("--output")
            .arg("./nightmare-obfs")
            .arg("--owner")
            .arg("CDLI")
            .arg("--project")
            .arg("nightmare")
            .arg("--yes")
            .arg("--instant"),
    );

    let stdout = String::from_utf8_lossy(&init.stdout);
    assert!(stdout.contains("NIGHTMARE"));
    assert!(stdout.contains("Config written"));
    assert!(config.exists());
}

#[test]
fn init_instant_uses_reduced_banner_and_non_tty_bare_command_prints_help() {
    let root = temp_root("init-instant");
    let config = root.join("nightmare.toml");

    let animated = output_ok(
        Command::new(bin())
            .arg("init")
            .arg("--config")
            .arg(&config)
            .arg("--source")
            .arg(".")
            .arg("--output")
            .arg("./nightmare-obfs")
            .arg("--owner")
            .arg("CDLI")
            .arg("--project")
            .arg("nightmare")
            .arg("--yes"),
    );
    let instant = output_ok(
        Command::new(bin())
            .arg("init")
            .arg("--config")
            .arg(root.join("instant.toml"))
            .arg("--source")
            .arg(".")
            .arg("--output")
            .arg("./nightmare-obfs")
            .arg("--owner")
            .arg("CDLI")
            .arg("--project")
            .arg("nightmare")
            .arg("--yes")
            .arg("--instant"),
    );
    let animated_stdout = String::from_utf8_lossy(&animated.stdout);
    let instant_stdout = String::from_utf8_lossy(&instant.stdout);
    assert!(animated_stdout.lines().count() > instant_stdout.lines().count());
    assert!(instant_stdout.contains("NIGHTMARE ::"));

    let help = output_ok(&mut Command::new(bin()));
    let help_stdout = String::from_utf8_lossy(&help.stdout);
    assert!(help_stdout.contains("Usage:"));
}

#[test]
fn tui_preview_renders_cdli_entry_screen() {
    let preview = output_ok(Command::new(bin()).arg("tui").arg("--preview"));
    let stdout = String::from_utf8_lossy(&preview.stdout);

    assert!(stdout.contains("CDLI.ai"));
    assert!(stdout.contains("cdli.ai"));
    assert!(stdout.contains("Public Login"));
    assert!(stdout.contains("Account Login"));
    assert!(stdout.contains("Nightmare Obfuscator"));
    assert!(stdout.contains("@@@@"));
}

#[test]
fn tui_account_login_rejects_until_backend_is_connected() {
    let login = output_fail(
        Command::new(bin())
            .arg("tui")
            .arg("--account-name")
            .arg("fatih")
            .arg("--password")
            .arg("secret"),
    );
    let stdout = String::from_utf8_lossy(&login.stdout);
    let stderr = String::from_utf8_lossy(&login.stderr);

    assert!(
        stdout.contains("Account login is not connected yet")
            || stderr.contains("Account login is not connected yet"),
        "account login should reject with the planned not-connected message\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
}

#[test]
fn init_edits_existing_contract_without_dropping_unmentioned_values() {
    let root = temp_root("init-edit");
    let input = root.join("rust-basic");
    let output = root.join("existing-output");
    let config = root.join("nightmare.toml");
    copy_dir(&repo_root().join("fixtures/rust-basic"), &input);
    write_contract(&config, &input, &output);

    run_ok(
        Command::new(bin())
            .arg("init")
            .arg("--config")
            .arg(&config)
            .arg("--owner")
            .arg("New Owner")
            .arg("--yes")
            .arg("--json"),
    );

    let config_toml = read_toml(&config);
    assert_eq!(
        config_toml["source"].as_str().unwrap(),
        input.to_string_lossy()
    );
    assert_eq!(
        config_toml["output"].as_str().unwrap(),
        output.to_string_lossy()
    );
    assert_eq!(config_toml["owner"]["name"].as_str().unwrap(), "New Owner");
    assert_eq!(config_toml["project"]["name"].as_str().unwrap(), "fixture");
    assert_eq!(
        config_toml["selected_paths"].as_array().unwrap()[0]
            .as_str()
            .unwrap(),
        "src/lib.rs"
    );
}

#[test]
fn init_can_run_contract_and_show_stage_statuses_without_tui_fragility() {
    let root = temp_root("init-run");
    let input = root.join("rust-basic");
    let output = root.join("init-run-output");
    let config = root.join("nightmare.toml");
    copy_dir(&repo_root().join("fixtures/rust-basic"), &input);

    let init = output_ok(
        Command::new(bin())
            .arg("init")
            .arg("--config")
            .arg(&config)
            .arg("--source")
            .arg(&input)
            .arg("--output")
            .arg(&output)
            .arg("--owner")
            .arg("CDLI")
            .arg("--project")
            .arg("fixture")
            .arg("--select")
            .arg("src/lib.rs")
            .arg("--yes")
            .arg("--instant")
            .arg("--run"),
    );

    let stdout = String::from_utf8_lossy(&init.stdout);
    assert!(stdout.contains("Config written"));
    assert!(stdout.contains("Nightmare Run"));
    assert!(stdout.contains("obfuscate:"));
    assert!(stdout.contains("verify:"));
    assert!(stdout.contains("build:"));
    assert!(output.join(".nightmare/manifest.json").exists());
}

#[test]
fn init_with_nested_config_writes_paths_that_run_resolves_correctly() {
    let root = temp_root("nested-config");
    let input = root.join("rust-basic");
    let output = root.join("nested-output");
    let config = root.join("configs/nightmare.toml");
    copy_dir(&repo_root().join("fixtures/rust-basic"), &input);

    run_ok(
        Command::new(bin())
            .arg("init")
            .arg("--config")
            .arg(&config)
            .arg("--source")
            .arg("rust-basic")
            .arg("--output")
            .arg("nested-output")
            .arg("--owner")
            .arg("CDLI")
            .arg("--project")
            .arg("fixture")
            .arg("--select")
            .arg("src/lib.rs")
            .arg("--yes")
            .current_dir(&root),
    );

    let run = output_ok(Command::new(bin()).arg("run").arg(&config).arg("--json"));
    let result: serde_json::Value = serde_json::from_slice(&run.stdout).unwrap();
    assert_eq!(result["status"], "passed");
    assert_eq!(
        canonical_string(PathBuf::from(result["source"].as_str().unwrap())),
        canonical_string(&input)
    );
    assert_eq!(
        canonical_string(PathBuf::from(result["output"].as_str().unwrap())),
        canonical_string(&output)
    );
    assert!(output.join(".nightmare/manifest.json").exists());
}
