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

#[test]
fn java_target_is_reflected_in_output_and_report() {
    let base = make_temp_dir("java-target");
    let input = base.join("sample.rpg");
    let output_java = base.join("Sample.java");
    let output_json = base.join("Sample.report.json");
    let output_md = base.join("Sample.report.md");

    fs::write(
        &input,
        "EVAL class = 1\nCALLP switch\nIF class *GT 0\nWRITE rec\nENDIF\n",
    )
    .expect("failed to write fixture");

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
        .arg("--class-name")
        .arg("class")
        .arg("--java-target")
        .arg("java25-stable")
        .output()
        .expect("failed to execute binary");

    assert!(
        output.status.success(),
        "expected conversion success\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let java_body = fs::read_to_string(&output_java).expect("failed to read java output");
    assert!(java_body.contains("target=java25-stable"));
    assert!(java_body.contains("public class class_v"));
    assert!(java_body.contains("double class_v = 0;"));
    assert!(java_body.contains("switch_v();"));

    let json_body = fs::read_to_string(&output_json).expect("failed to read report json");
    assert!(json_body.contains("\"java_target\": \"java25-stable\""));

    let md_body = fs::read_to_string(&output_md).expect("failed to read report markdown");
    assert!(md_body.contains("Java Target: java25-stable"));
}

#[test]
fn invalid_java_target_returns_error() {
    let base = make_temp_dir("java-target-invalid");
    let input = base.join("sample.rpg");
    fs::write(&input, "EVAL A = 1\n").expect("failed to write fixture");

    let bin = env!("CARGO_BIN_EXE_rpg2java-transpiler");
    let output = Command::new(bin)
        .arg("--input")
        .arg(&input)
        .arg("--java-target")
        .arg("java99")
        .output()
        .expect("failed to execute binary");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("invalid --java-target"), "stderr={stderr}");
}
