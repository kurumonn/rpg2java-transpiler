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
fn report_includes_symbol_assignment_and_reference_lines() {
    let base = make_temp_dir("symbol-positions");
    let input = base.join("symbols.rpg");
    let output_java = base.join("Symbols.java");
    let output_json = base.join("Symbols.report.json");
    let output_md = base.join("Symbols.report.md");

    fs::write(
        &input,
        "EVAL A = 1\n\
EVAL B = A\n\
IF B *GT 0\n\
EVAL A = B\n\
ENDIF\n\
WRITE A\n",
    )
    .expect("write input");

    let bin = env!("CARGO_BIN_EXE_rpg2java-transpiler");
    let output = Command::new(bin)
        .arg("--input")
        .arg(&input)
        .arg("--mode")
        .arg("free")
        .arg("--output")
        .arg(&output_java)
        .arg("--report-json")
        .arg(&output_json)
        .arg("--report-md")
        .arg(&output_md)
        .arg("--class-name")
        .arg("Symbols")
        .output()
        .expect("failed to execute binary");

    assert!(
        output.status.success(),
        "conversion should succeed\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let json = fs::read_to_string(&output_json).expect("read json report");
    assert!(json.contains("\"symbols\""), "json={json}");
    assert!(
        json.contains(
            "\"name\":\"A\",\"type\":\"number\",\"assigned_lines\":[1,4],\"referenced_lines\":[2,6]"
        ),
        "json={json}"
    );
    assert!(
        json.contains(
            "\"name\":\"B\",\"type\":\"number\",\"assigned_lines\":[2],\"referenced_lines\":[3,4]"
        ),
        "json={json}"
    );

    let md = fs::read_to_string(&output_md).expect("read markdown report");
    assert!(md.contains("## Symbols"), "md={md}");
    assert!(md.contains("| A | number | 1,4 | 2,6 |"), "md={md}");
    assert!(md.contains("| B | number | 2 | 3,4 |"), "md={md}");
}
