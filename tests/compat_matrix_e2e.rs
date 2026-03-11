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

fn fixture_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("rpg_versions")
}

#[test]
fn version_matrix_converts_all_fixtures() {
    let input_dir = fixture_path();
    let base = make_temp_dir("version-matrix");
    let output_dir = base.join("out");
    let snapshot_dir = base.join("snapshots");

    let bin = env!("CARGO_BIN_EXE_rpg2java-transpiler");
    let output = Command::new(bin)
        .arg("--batch-dir")
        .arg(&input_dir)
        .arg("--output-dir")
        .arg(&output_dir)
        .arg("--snapshot-dir")
        .arg(&snapshot_dir)
        .arg("--update-snapshots")
        .arg("--jobs")
        .arg("2")
        .output()
        .expect("failed to execute binary");

    assert!(
        output.status.success(),
        "expected all fixture files to convert successfully\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let expected = [
        "Rpg3FixedCalc",
        "Rpg4FixedFileio",
        "IleRpgFreeProc",
        "As400HybridStyle",
    ];
    for name in expected {
        assert!(
            output_dir.join(format!("{name}.java")).exists(),
            "missing converted java for {name}"
        );
        assert!(
            output_dir.join(format!("{name}.report.json")).exists(),
            "missing JSON report for {name}"
        );
        assert!(
            output_dir.join(format!("{name}.report.md")).exists(),
            "missing Markdown report for {name}"
        );
        assert!(
            snapshot_dir.join(format!("{name}.java")).exists(),
            "missing snapshot for {name}"
        );
    }
}

#[test]
fn version_matrix_reports_include_operation_summary() {
    let input_dir = fixture_path();
    let base = make_temp_dir("version-report");
    let output_dir = base.join("out");

    let bin = env!("CARGO_BIN_EXE_rpg2java-transpiler");
    let output = Command::new(bin)
        .arg("--batch-dir")
        .arg(&input_dir)
        .arg("--output-dir")
        .arg(&output_dir)
        .arg("--jobs")
        .arg("2")
        .output()
        .expect("failed to execute binary");

    assert!(
        output.status.success(),
        "conversion should succeed\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let checks = [
        ("Rpg3FixedCalc.report.json", "\"name\": \"EVAL\""),
        ("Rpg4FixedFileio.report.json", "\"name\": \"READE\""),
        ("IleRpgFreeProc.report.json", "\"name\": \"CALLP\""),
        ("As400HybridStyle.report.json", "\"name\": \"CHAIN\""),
    ];
    for (file, marker) in checks {
        let body = fs::read_to_string(output_dir.join(file))
            .unwrap_or_else(|_| panic!("failed to read report {file}"));
        assert!(
            body.contains("\"operation_summary\""),
            "report missing operation summary: {file}"
        );
        assert!(
            body.contains(marker),
            "report does not include expected operation marker '{marker}' in {file}"
        );
    }
}
