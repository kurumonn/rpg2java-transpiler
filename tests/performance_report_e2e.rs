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
fn perf_summary_contains_thresholds_and_signals() {
    let base = make_temp_dir("perf-thresholds");
    let input_dir = base.join("in");
    let output_dir = base.join("out");
    let perf_json = base.join("perf.summary.json");
    fs::create_dir_all(&input_dir).expect("failed to create input dir");

    write_file(
        &input_dir.join("sample_a.rpg"),
        "EVAL A = 1\nIF A *GT 0\nCALLP HELLO\nENDIF\n",
    );
    write_file(
        &input_dir.join("sample_b.rpg"),
        "EVAL B = 2\nIF B *GT 0\nWRITE REC_B\nENDIF\n",
    );

    let bin = env!("CARGO_BIN_EXE_rpg2java-transpiler");
    let output = Command::new(bin)
        .arg("--batch-dir")
        .arg(&input_dir)
        .arg("--output-dir")
        .arg(&output_dir)
        .arg("--perf-report-json")
        .arg(&perf_json)
        .arg("--jobs")
        .arg("2")
        .output()
        .expect("failed to execute binary");

    assert!(
        output.status.success(),
        "batch should succeed\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(perf_json.exists(), "perf summary json should exist");

    let body = fs::read_to_string(&perf_json).expect("failed to read perf summary");
    assert!(body.contains("\"summary\""), "body={body}");
    assert!(body.contains("\"avg_ms_per_kib\""), "body={body}");
    assert!(body.contains("\"thresholds\""), "body={body}");
    assert!(body.contains("\"signals\""), "body={body}");
    assert!(body.contains("\"p95_ms_warn\""), "body={body}");
    assert!(body.contains("\"low_parallelism\""), "body={body}");
}
