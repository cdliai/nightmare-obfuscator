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

fn run_fail(command: &mut Command) {
    let output = command.output().unwrap();
    assert!(
        !output.status.success(),
        "command unexpectedly succeeded\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
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

    run_fail(Command::new(bin()).arg("verify").arg(&output));
}
