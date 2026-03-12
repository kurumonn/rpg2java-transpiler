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
fn fixed_if_indicator_is_reflected_in_java_condition() {
    let base = make_temp_dir("fixed-if-indicator");
    let input = base.join("if_in90.rpg");
    let output_java = base.join("IfIn90.java");
    let output_json = base.join("IfIn90.report.json");

    fs::write(
        &input,
        "000100C                   IF        *IN90\n000200C                   ENDIF\n",
    )
    .expect("write input");

    let bin = env!("CARGO_BIN_EXE_rpg2java-transpiler");
    let output = Command::new(bin)
        .arg("--input")
        .arg(&input)
        .arg("--mode")
        .arg("fixed")
        .arg("--output")
        .arg(&output_java)
        .arg("--report-json")
        .arg(&output_json)
        .arg("--class-name")
        .arg("IfIn90")
        .output()
        .expect("failed to execute binary");

    assert!(
        output.status.success(),
        "conversion should succeed\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let java = fs::read_to_string(&output_java).expect("read java");
    assert!(
        java.contains("if (truthy(IN90)) {"),
        "indicator condition must be reflected in Java output: {java}"
    );

    let json = fs::read_to_string(&output_json).expect("read report");
    assert!(json.contains("\"name\": \"IF\""), "report={json}");
}

#[test]
fn fixed_if_negative_indicator_is_reflected_in_java_condition() {
    let base = make_temp_dir("fixed-if-neg-indicator");
    let input = base.join("if_n90.rpg");
    let output_java = base.join("IfN90.java");

    fs::write(
        &input,
        "000100C                   IF        N90\n000200C                   ENDIF\n",
    )
    .expect("write input");

    let bin = env!("CARGO_BIN_EXE_rpg2java-transpiler");
    let output = Command::new(bin)
        .arg("--input")
        .arg(&input)
        .arg("--mode")
        .arg("fixed")
        .arg("--output")
        .arg(&output_java)
        .arg("--class-name")
        .arg("IfN90")
        .output()
        .expect("failed to execute binary");

    assert!(
        output.status.success(),
        "conversion should succeed\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let java = fs::read_to_string(&output_java).expect("read java");
    assert!(
        java.contains("if (!truthy(IN90)) {"),
        "negative indicator condition must be reflected in Java output: {java}"
    );
}
