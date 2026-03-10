use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn make_temp_dir(name: &str) -> PathBuf {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock error")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "rpg2java-{name}-{}-{ts}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).expect("failed to create temp dir");
    dir
}

fn write_file(path: &Path, content: &str) {
    fs::write(path, content).expect("failed to write test file");
}

#[test]
fn batch_continues_when_one_file_is_invalid_utf8() {
    let base = make_temp_dir("batch-continue");
    let input_dir = base.join("in");
    let output_dir = base.join("out");
    fs::create_dir_all(&input_dir).expect("failed to create input dir");

    let ok_file = input_dir.join("ok_sample.rpg");
    write_file(
        &ok_file,
        "* sample\nEVAL TOTAL = PRICE + TAX\nIF TOTAL *GT LIMIT\nWRITE ORDER_REC\nENDIF\n",
    );

    let ng_file = input_dir.join("broken_sample.rpg");
    fs::write(&ng_file, vec![0xff, 0xfe, 0x00, 0x81]).expect("failed to write invalid file");

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
        !output.status.success(),
        "batch should fail when at least one file fails"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("batch complete:"),
        "must print final batch summary"
    );
    assert!(
        stderr.contains("failed=1"),
        "expected one failed file, stderr={stderr}"
    );
    assert!(
        stderr.contains("success=1"),
        "expected one successful file, stderr={stderr}"
    );

    let java_out = output_dir.join("OkSample.java");
    assert!(
        java_out.exists(),
        "valid file should still be converted even if another file fails"
    );
}

#[test]
fn batch_success_when_all_files_are_valid() {
    let base = make_temp_dir("batch-success");
    let input_dir = base.join("in");
    let output_dir = base.join("out");
    fs::create_dir_all(&input_dir).expect("failed to create input dir");

    write_file(
        &input_dir.join("sample_a.rpg"),
        "EVAL A = 1\nIF A *GT 0\nCALLP HELLO\nENDIF\n",
    );
    write_file(
        &input_dir.join("sample_b.rpg"),
        "EVAL B = 2\nWRITE REC_B\n",
    );

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

    assert!(output.status.success(), "expected success when all files are valid");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("success=2"), "stderr={stderr}");
    assert!(stderr.contains("failed=0"), "stderr={stderr}");
    assert!(output_dir.join("SampleA.java").exists());
    assert!(output_dir.join("SampleB.java").exists());
}
