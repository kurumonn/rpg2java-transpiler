use std::fs;
use std::path::PathBuf;
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

fn collect_files_with_ext(dir: &PathBuf, ext: &str) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(read_dir) = fs::read_dir(dir) {
        for entry in read_dir.flatten() {
            let path = entry.path();
            if path.is_dir() {
                out.extend(collect_files_with_ext(&path, ext));
            } else if path
                .extension()
                .and_then(|s| s.to_str())
                .map(|s| s.eq_ignore_ascii_case(ext))
                .unwrap_or(false)
            {
                out.push(path);
            }
        }
    }
    out
}

#[test]
fn real_corpus_conversion_pipeline() {
    let Some(corpus_dir) = std::env::var_os("RPG_REAL_CORPUS_DIR") else {
        eprintln!("skip: RPG_REAL_CORPUS_DIR is not set");
        return;
    };
    let corpus_dir = PathBuf::from(corpus_dir);
    if !corpus_dir.exists() {
        eprintln!(
            "skip: RPG_REAL_CORPUS_DIR does not exist: {}",
            corpus_dir.display()
        );
        return;
    }

    let base = make_temp_dir("real-corpus");
    let output_dir = base.join("out");
    let snapshot_dir = base.join("snapshots");

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_rpg2java-transpiler"));
    cmd.arg("--batch-dir")
        .arg(&corpus_dir)
        .arg("--output-dir")
        .arg(&output_dir)
        .arg("--snapshot-dir")
        .arg(&snapshot_dir)
        .arg("--jobs")
        .arg(std::env::var("RPG_REAL_CORPUS_JOBS").unwrap_or_else(|_| String::from("4")));

    if std::env::var("RPG_UPDATE_SNAPSHOTS")
        .map(|v| v == "1")
        .unwrap_or(false)
    {
        cmd.arg("--update-snapshots");
    }

    let output = cmd.output().expect("failed to execute binary");
    assert!(
        output.status.success(),
        "real corpus conversion failed\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let java_files = collect_files_with_ext(&output_dir, "java");
    let report_files = collect_files_with_ext(&output_dir, "json");
    assert!(
        !java_files.is_empty(),
        "no converted java files found in {}",
        output_dir.display()
    );
    assert!(
        !report_files.is_empty(),
        "no report json files found in {}",
        output_dir.display()
    );

    for report in &report_files {
        let body = fs::read_to_string(report)
            .unwrap_or_else(|_| panic!("failed to read report {}", report.display()));
        assert!(
            body.contains("\"operation_summary\""),
            "missing operation_summary in report {}",
            report.display()
        );
    }
}
