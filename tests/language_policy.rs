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

fn run_ok(command: &mut Command) -> std::process::Output {
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "command failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn run_fail(command: &mut Command) -> std::process::Output {
    let output = command.output().unwrap();
    assert!(
        !output.status.success(),
        "command unexpectedly succeeded\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

#[test]
fn language_docs_keep_v1_rust_only() {
    let readme = fs::read_to_string(repo_root().join("README.md")).unwrap();
    let docs = fs::read_to_string(repo_root().join("docs/language-support.md")).unwrap();
    let core = fs::read_to_string(repo_root().join("crates/core/src/lib.rs")).unwrap();
    let parser = fs::read_to_string(repo_root().join("crates/parser/src/lib.rs")).unwrap();

    assert!(readme.contains("Rust source support only"));
    assert!(docs.contains("Python, JavaScript, TypeScript, Go, C, C++, and Java are roadmap-only"));
    assert!(docs.contains("syntax-aware parser"));
    assert!(docs.contains("public APIs"));
    assert!(!core.contains("Supported programming languages"));
    assert!(!parser.contains("Multi-language parser"));
}

#[test]
fn run_contract_rejects_unknown_language_fields() {
    let root = temp_root("unknown-language-field");
    let input = root.join("rust-basic");
    let output = root.join("out");
    let config = root.join("nightmare.toml");
    copy_dir(&repo_root().join("fixtures/rust-basic"), &input);
    fs::write(
        &config,
        format!(
            r#"schema_version = 1
source = "{}"
output = "{}"
languages = ["python"]

[owner]
name = "CDLI"

[project]
name = "fixture"
"#,
            input.display(),
            output.display()
        ),
    )
    .unwrap();

    let failed = run_fail(Command::new(bin()).arg("run").arg(&config).arg("--json"));
    assert!(String::from_utf8_lossy(&failed.stderr).contains("unknown field"));
}

#[test]
fn non_rust_files_are_opaque_assets_in_v1_manifest() {
    let root = temp_root("mixed-language");
    let input = root.join("mixed");
    let output = root.join("mixed-output");
    copy_dir(&repo_root().join("fixtures/rust-basic"), &input);
    fs::write(input.join("tool.py"), "print('python stays opaque')\n").unwrap();
    fs::write(input.join("web.js"), "console.log('js stays opaque')\n").unwrap();
    fs::write(
        input.join("typed.ts"),
        "export const x = 'ts stays opaque'\n",
    )
    .unwrap();
    fs::write(input.join("go.go"), "package main\n").unwrap();
    fs::write(input.join("c.c"), "int main(void) { return 0; }\n").unwrap();
    fs::write(input.join("cpp.cpp"), "int main() { return 0; }\n").unwrap();
    fs::write(input.join("Java.java"), "class Java {}\n").unwrap();

    run_ok(
        Command::new(bin())
            .arg("obfuscate")
            .arg(&input)
            .arg("--output")
            .arg(&output),
    );

    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(output.join(".nightmare/manifest.json")).unwrap())
            .unwrap();
    for rel in [
        "tool.py",
        "web.js",
        "typed.ts",
        "go.go",
        "c.c",
        "cpp.cpp",
        "Java.java",
    ] {
        let entry = manifest["files"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["path"] == rel)
            .unwrap();
        assert_eq!(entry["language"], "Unknown");
        assert_eq!(entry["obfuscated"], false);
        assert_eq!(
            fs::read(input.join(rel)).unwrap(),
            fs::read(output.join(rel)).unwrap()
        );
    }
}

#[test]
fn direct_non_rust_selection_fails_with_rust_only_message() {
    let root = temp_root("select-non-rust");
    let input = root.join("mixed");
    let output = root.join("mixed-output");
    copy_dir(&repo_root().join("fixtures/rust-basic"), &input);
    fs::write(input.join("tool.py"), "print('python')\n").unwrap();

    let failed = run_fail(
        Command::new(bin())
            .arg("obfuscate")
            .arg(&input)
            .arg("--output")
            .arg(&output)
            .arg("--select")
            .arg("tool.py"),
    );
    assert!(String::from_utf8_lossy(&failed.stderr).contains("Rust-only"));
}
