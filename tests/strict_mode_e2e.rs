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
fn strict_mode_fails_on_unsupported_operation_single() {
    let base = make_temp_dir("strict-single");
    let input = base.join("unsupported.rpg");
    let output_java = base.join("out.java");
    fs::write(&input, "SETLL KEY FILE\n").expect("failed to write input");

    let bin = env!("CARGO_BIN_EXE_rpg2java-transpiler");
    let output = Command::new(bin)
        .arg("--input")
        .arg(&input)
        .arg("--output")
        .arg(&output_java)
        .arg("--strict")
        .output()
        .expect("failed to execute binary");

    assert!(
        !output.status.success(),
        "strict mode should fail for unsupported op"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("strict mode violation"), "stderr={stderr}");
    assert!(
        stderr.contains("unsupported operations found"),
        "stderr={stderr}"
    );
}

#[test]
fn strict_mode_batch_marks_unsupported_file_as_failed() {
    let base = make_temp_dir("strict-batch");
    let input_dir = base.join("in");
    let output_dir = base.join("out");
    fs::create_dir_all(&input_dir).expect("failed to create input dir");
    fs::write(input_dir.join("ok.rpg"), "EVAL A = 1\n").expect("failed to write ok");
    fs::write(input_dir.join("ng.rpg"), "CHAIN K FILE\n").expect("failed to write ng");

    let bin = env!("CARGO_BIN_EXE_rpg2java-transpiler");
    let output = Command::new(bin)
        .arg("--batch-dir")
        .arg(&input_dir)
        .arg("--output-dir")
        .arg(&output_dir)
        .arg("--strict")
        .output()
        .expect("failed to execute binary");

    assert!(
        !output.status.success(),
        "batch strict mode should fail when any file has unsupported op"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("strict mode violation"), "stderr={stderr}");
    assert!(stderr.contains("success=1"), "stderr={stderr}");
    assert!(stderr.contains("failed=1"), "stderr={stderr}");
}
