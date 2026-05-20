use std::fs;
use std::path::PathBuf;
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

fn success_command() -> &'static str {
    if cfg!(windows) {
        "cmd /C exit /B 0"
    } else {
        "/bin/true"
    }
}

fn failure_command() -> &'static str {
    if cfg!(windows) {
        "cmd /C exit /B 1"
    } else {
        "/bin/false"
    }
}

#[test]
fn javac_check_requires_output_in_single_mode() {
    let base = make_temp_dir("javac-check-no-output");
    let input = base.join("sample.rpg");
    fs::write(&input, "EVAL A = 1\n").expect("failed to write fixture");

    let bin = env!("CARGO_BIN_EXE_rpg2java-transpiler");
    let output = Command::new(bin)
        .arg("--input")
        .arg(&input)
        .arg("--javac-check")
        .output()
        .expect("failed to execute binary");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("--javac-check requires --output"),
        "stderr={stderr}"
    );
}

#[test]
fn javac_check_accepts_custom_success_command() {
    let base = make_temp_dir("javac-check-success");
    let input = base.join("sample.rpg");
    let output_java = base.join("Sample.java");
    let output_json = base.join("Sample.report.json");
    let output_md = base.join("Sample.report.md");
    fs::write(&input, "EVAL A = 1\n").expect("failed to write fixture");

    let bin = env!("CARGO_BIN_EXE_rpg2java-transpiler");
    let output = Command::new(bin)
        .arg("--input")
        .arg(&input)
        .arg("--output")
        .arg(&output_java)
        .arg("--report-json")
        .arg(&output_json)
        .arg("--report-md")
        .arg(&output_md)
        .arg("--javac-check")
        .arg("--javac-cmd")
        .arg(success_command())
        .output()
        .expect("failed to execute binary");

    assert!(
        output.status.success(),
        "expected success\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json_body = fs::read_to_string(&output_json).expect("failed to read report json");
    assert!(json_body.contains("\"javac_check\""));
    assert!(json_body.contains("\"success\": true"));
    let md_body = fs::read_to_string(&output_md).expect("failed to read report markdown");
    assert!(md_body.contains("Javac Check: success"));
}

#[test]
fn javac_check_reports_failure_with_custom_command() {
    let base = make_temp_dir("javac-check-fail");
    let input = base.join("sample.rpg");
    let output_java = base.join("Sample.java");
    let output_json = base.join("Sample.report.json");
    let output_md = base.join("Sample.report.md");
    fs::write(&input, "EVAL A = 1\n").expect("failed to write fixture");

    let bin = env!("CARGO_BIN_EXE_rpg2java-transpiler");
    let output = Command::new(bin)
        .arg("--input")
        .arg(&input)
        .arg("--output")
        .arg(&output_java)
        .arg("--report-json")
        .arg(&output_json)
        .arg("--report-md")
        .arg(&output_md)
        .arg("--javac-check")
        .arg("--javac-cmd")
        .arg(failure_command())
        .output()
        .expect("failed to execute binary");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("javac check failed"), "stderr={stderr}");
    let json_body = fs::read_to_string(&output_json).expect("failed to read report json");
    assert!(json_body.contains("\"javac_check\""));
    assert!(json_body.contains("\"success\": false"));
    let md_body = fs::read_to_string(&output_md).expect("failed to read report markdown");
    assert!(md_body.contains("Javac Check: failed"));
}
