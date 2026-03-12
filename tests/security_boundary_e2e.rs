use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn make_temp_dir(name: &str) -> PathBuf {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock error")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("rpg2java-{name}-{}-{ts}", std::process::id()));
    fs::create_dir_all(&dir).expect("failed to create temp dir");
    dir
}

fn write_file(path: &Path, content: &str) {
    fs::write(path, content).expect("failed to write file");
}

#[test]
fn single_mode_invalid_utf8_returns_error_without_panic() {
    let base = make_temp_dir("security-invalid-utf8");
    let input = base.join("bad_utf8.rpg");
    fs::write(&input, vec![0xff, 0xfe, 0x81]).expect("failed to write invalid utf8 fixture");

    let bin = env!("CARGO_BIN_EXE_rpg2java-transpiler");
    let output = Command::new(bin)
        .arg("--input")
        .arg(&input)
        .arg("--class-name")
        .arg("MainProgram")
        .output()
        .expect("failed to execute binary");

    assert!(!output.status.success(), "must fail for invalid utf8");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("failed to read input file"), "stderr={stderr}");
    assert!(
        !stderr.to_ascii_lowercase().contains("panic"),
        "stderr={stderr}"
    );
}

#[test]
fn dangerous_class_name_is_sanitized_in_java_output() {
    let base = make_temp_dir("security-class-name");
    let input = base.join("sample.rpg");
    let output_java = base.join("Sanitized.java");
    write_file(&input, "EVAL A = 1\n");

    let bin = env!("CARGO_BIN_EXE_rpg2java-transpiler");
    let output = Command::new(bin)
        .arg("--input")
        .arg(&input)
        .arg("--output")
        .arg(&output_java)
        .arg("--class-name")
        .arg("../../Bad;Name$(rm -rf /)")
        .arg("--mode")
        .arg("free")
        .output()
        .expect("failed to execute binary");

    assert!(
        output.status.success(),
        "conversion should succeed with sanitized class name\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let body = fs::read_to_string(&output_java).expect("failed to read java output");
    let class_line = body
        .lines()
        .find(|l| l.starts_with("public class "))
        .unwrap_or("");
    assert!(!class_line.is_empty(), "missing class declaration: {body}");
    assert!(!class_line.contains(".."), "class line={class_line}");
    assert!(!class_line.contains('/'), "class line={class_line}");
    assert!(!class_line.contains(';'), "class line={class_line}");
    assert!(
        !class_line.contains("$("),
        "shell-like token must be sanitized: {class_line}"
    );
}

#[test]
fn very_long_input_line_is_handled_without_crash() {
    let base = make_temp_dir("security-long-line");
    let input = base.join("long_line.rpg");
    let output_java = base.join("LongLine.java");

    let mut expr = String::from("1");
    for _ in 0..4000 {
        expr.push_str(" + 1");
    }
    write_file(&input, &format!("EVAL TOTAL = {expr}\n"));

    let bin = env!("CARGO_BIN_EXE_rpg2java-transpiler");
    let output = Command::new(bin)
        .arg("--input")
        .arg(&input)
        .arg("--output")
        .arg(&output_java)
        .arg("--class-name")
        .arg("LongLine")
        .arg("--mode")
        .arg("free")
        .output()
        .expect("failed to execute binary");

    assert!(
        output.status.success(),
        "conversion should handle long input line\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output_java.exists(), "java output should exist");
}
