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
fn perf_summary_includes_failure_categories() {
    let base = make_temp_dir("failure-analysis");
    let input_dir = base.join("in");
    let output_dir = base.join("out");
    let perf_json = base.join("perf.summary.json");
    fs::create_dir_all(&input_dir).expect("failed to create input dir");

    write_file(
        &input_dir.join("ok_sample.rpg"),
        "EVAL TOTAL = PRICE + TAX\nIF TOTAL *GT LIMIT\nENDIF\n",
    );
    fs::write(input_dir.join("bad_utf8.rpg"), vec![0xff, 0xfe, 0x00, 0x81])
        .expect("failed to write invalid utf8 fixture");

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
        !output.status.success(),
        "batch should fail when one file is invalid"
    );
    assert!(perf_json.exists(), "perf report must be generated");

    let body = fs::read_to_string(&perf_json).expect("failed to read perf report");
    assert!(
        body.contains("\"failure_categories\""),
        "missing failure_categories: {body}"
    );
    assert!(
        body.contains("\"key\":\"input_decode\""),
        "missing input_decode category: {body}"
    );
    assert!(
        body.contains("\"count\":1"),
        "missing failure category count: {body}"
    );
}
