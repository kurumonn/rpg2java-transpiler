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
fn fixed_mode_reports_misaligned_opcode_column() {
    let base = make_temp_dir("fixed-misaligned-opcode");
    let input = base.join("misaligned.rpg");
    let output_java = base.join("Misaligned.java");
    let output_json = base.join("Misaligned.report.json");

    fs::write(&input, "000100C EVAL      PRICE + TAX    TOTAL\n").expect("write input");

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
        .arg("Misaligned")
        .output()
        .expect("failed to execute binary");

    assert!(
        output.status.success(),
        "conversion should complete with diagnostics\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let report = fs::read_to_string(&output_json).expect("read report");
    assert!(
        report.contains("outside fixed opcode field"),
        "expected fixed-column diagnostic in report: {report}"
    );
    assert!(
        report.contains("\"severity\":\"warning\"")
            && report.contains("\"code\":\"PARSER_FIXED_OPCODE_MISALIGNED\"")
            && report.contains("\"line\":1"),
        "expected standardized severity/code/line diagnostic fields: {report}"
    );
}

#[test]
fn fixed_mode_reports_short_c_spec_line() {
    let base = make_temp_dir("fixed-short-line");
    let input = base.join("short.rpg");
    let output_java = base.join("Short.java");
    let output_json = base.join("Short.report.json");
    let output_md = base.join("Short.report.md");

    fs::write(&input, "000100C\n").expect("write input");

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
        .arg("--report-md")
        .arg(&output_md)
        .arg("--class-name")
        .arg("Short")
        .output()
        .expect("failed to execute binary");

    assert!(
        output.status.success(),
        "conversion should complete with diagnostics\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let report = fs::read_to_string(&output_json).expect("read report");
    assert!(
        report.contains("\"code\":\"PARSER_FIXED_SHORT_LINE\"")
            && report.contains("\"line\":1")
            && report.contains("\"column\":26")
            && report.contains("too short for opcode field"),
        "expected short-line structured diagnostic in report: {report}"
    );

    let md = fs::read_to_string(&output_md).expect("read markdown report");
    assert!(
        md.contains("[warning][PARSER_FIXED_SHORT_LINE] line 1 col 26"),
        "expected standardized diagnostic format in markdown: {md}"
    );
}
