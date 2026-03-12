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
fn real_corpus_pipeline_script_matches_documented_layout_and_snapshot_behavior() {
    let base = make_temp_dir("corpus-ops-script");
    let input_dir = base.join("in");
    let output_dir = base.join("out");
    fs::create_dir_all(&input_dir).expect("failed to create input dir");

    write_file(&input_dir.join("sample_a.rpg"), "EVAL A = 1\nWRITE REC_A\n");
    write_file(&input_dir.join("sample_b.rpg"), "EVAL B = 2\nIF B *GT 0\nENDIF\n");

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let script = manifest_dir.join("scripts").join("real_corpus_pipeline.sh");

    let first = Command::new("bash")
        .arg(&script)
        .arg(&input_dir)
        .arg(&output_dir)
        .arg("2")
        .env("RPG_JAVA_TARGET", "java21")
        .current_dir(&manifest_dir)
        .output()
        .expect("failed to execute script");
    assert!(
        first.status.success(),
        "script first run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&first.stdout),
        String::from_utf8_lossy(&first.stderr)
    );

    assert!(output_dir.join("batch").join("SampleA.java").exists());
    assert!(output_dir.join("batch").join("SampleB.java").exists());
    assert!(output_dir.join("batch").join("SampleA.report.json").exists());
    assert!(output_dir.join("batch").join("SampleB.report.md").exists());
    assert!(output_dir.join("snapshots").join("SampleA.java").exists());
    assert!(output_dir.join("snapshots").join("SampleB.java").exists());
    assert!(output_dir.join("metrics.csv").exists());
    assert!(output_dir.join("metrics.summary.json").exists());

    let second = Command::new("bash")
        .arg(&script)
        .arg(&input_dir)
        .arg(&output_dir)
        .arg("2")
        .env("RPG_JAVA_TARGET", "java21")
        .current_dir(&manifest_dir)
        .output()
        .expect("failed to execute script");
    assert!(
        second.status.success(),
        "script second run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&second.stdout),
        String::from_utf8_lossy(&second.stderr)
    );

    write_file(&input_dir.join("sample_a.rpg"), "EVAL A = 99\nWRITE REC_A\n");
    let third = Command::new("bash")
        .arg(&script)
        .arg(&input_dir)
        .arg(&output_dir)
        .arg("2")
        .env("RPG_JAVA_TARGET", "java21")
        .current_dir(&manifest_dir)
        .output()
        .expect("failed to execute script");
    assert!(
        !third.status.success(),
        "script must fail when snapshots mismatch without RPG_UPDATE_SNAPSHOTS=1"
    );
    let stderr = String::from_utf8_lossy(&third.stderr);
    assert!(stderr.contains("snapshot mismatch"), "stderr={stderr}");
}
