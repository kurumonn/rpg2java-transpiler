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
fn single_file_can_emit_html_report_and_source_map() {
    let base = make_temp_dir("commercial-html-source-map");
    let input = base.join("sample.rpg");
    let output_java = base.join("Commercial.java");
    let report_html = base.join("report.html");
    let source_map = base.join("source-map.json");

    fs::write(
        &input,
        "EVAL TOTAL = 1\nCHAIN CUSTFILE CUSTREC\nIF TOTAL *GT 0\nENDIF\n",
    )
    .expect("failed to write fixture");

    let bin = env!("CARGO_BIN_EXE_rpg2java-transpiler");
    let output = Command::new(bin)
        .arg("convert")
        .arg("--input")
        .arg(&input)
        .arg("--output")
        .arg(&output_java)
        .arg("--class-name")
        .arg("Commercial")
        .arg("--mode")
        .arg("free")
        .arg("--report-html")
        .arg(&report_html)
        .arg("--source-map")
        .arg(&source_map)
        .output()
        .expect("failed to execute binary");

    assert!(
        output.status.success(),
        "expected conversion success\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let html = fs::read_to_string(&report_html).expect("failed to read HTML report");
    assert!(html.contains("RPG2Java Migration Report"), "html={html}");
    assert!(html.contains("Top Unsupported Operations"), "html={html}");
    assert!(html.contains("CHAIN"), "html={html}");

    let map = fs::read_to_string(&source_map).expect("failed to read source map");
    assert!(
        map.contains("\"version\": \"rpg2java-source-map.v1\""),
        "map={map}"
    );
    assert!(map.contains("\"rpg_line\":1"), "map={map}");
    assert!(map.contains("\"java_line\""), "map={map}");
}

#[test]
fn batch_can_auto_emit_html_reports_source_maps_and_gate_todo_rate() {
    let base = make_temp_dir("commercial-batch");
    let input_dir = base.join("input");
    let output_dir = base.join("out");
    fs::create_dir_all(&input_dir).expect("failed to create input dir");
    fs::write(input_dir.join("a.rpg"), "EVAL TOTAL = 1\n").expect("failed to write fixture");
    fs::write(input_dir.join("b.rpg"), "CHAIN CUSTFILE CUSTREC\n")
        .expect("failed to write fixture");

    let bin = env!("CARGO_BIN_EXE_rpg2java-transpiler");
    let output = Command::new(bin)
        .arg("batch")
        .arg("--batch-dir")
        .arg(&input_dir)
        .arg("--output-dir")
        .arg(&output_dir)
        .arg("--html-report")
        .arg("--source-map-report")
        .arg("--fail-on-todo-rate")
        .arg("1.0")
        .output()
        .expect("failed to execute binary");

    assert!(
        output.status.success(),
        "expected batch success\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output_dir.join("A.report.html").exists());
    assert!(output_dir.join("A.source-map.json").exists());
    assert!(output_dir.join("B.report.html").exists());
    assert!(output_dir.join("B.source-map.json").exists());
}

#[test]
fn doctor_command_reports_commercial_runtime_capabilities() {
    let bin = env!("CARGO_BIN_EXE_rpg2java-transpiler");
    let output = Command::new(bin)
        .arg("doctor")
        .output()
        .expect("failed to execute binary");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("rpg2java doctor"), "stdout={stdout}");
    assert!(
        stdout.contains("reports: json, markdown, html, source-map"),
        "stdout={stdout}"
    );
}
