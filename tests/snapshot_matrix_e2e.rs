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

fn fixture_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("rpg_versions")
        .join(name)
}

fn snapshot_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("snapshots")
        .join("matrix")
        .join(name)
}

fn normalize_newline(s: &str) -> String {
    s.replace("\r\n", "\n")
}

fn assert_matches_snapshot(actual: &Path, snapshot: &Path) {
    let actual_body = fs::read_to_string(actual)
        .unwrap_or_else(|_| panic!("failed to read actual file: {}", actual.display()));
    let snapshot_body = fs::read_to_string(snapshot)
        .unwrap_or_else(|_| panic!("failed to read snapshot file: {}", snapshot.display()));
    assert_eq!(
        normalize_newline(&actual_body),
        normalize_newline(&snapshot_body),
        "snapshot mismatch\nactual={}\nsnapshot={}",
        actual.display(),
        snapshot.display()
    );
}

#[test]
fn snapshot_matrix_fixed_free_hybrid_outputs_are_stable() {
    struct Case {
        input: &'static str,
        class_name: &'static str,
        mode: &'static str,
    }

    let cases = [
        Case {
            input: "ile_rpg_free_proc.rpg",
            class_name: "IleRpgFreeProc",
            mode: "free",
        },
        Case {
            input: "rpg3_fixed_calc.rpg",
            class_name: "Rpg3FixedCalc",
            mode: "fixed",
        },
        Case {
            input: "as400_hybrid_style.rpg",
            class_name: "As400HybridStyle",
            mode: "auto",
        },
    ];

    let base = make_temp_dir("snapshot-matrix");
    let bin = env!("CARGO_BIN_EXE_rpg2java-transpiler");

    for c in cases {
        let input = fixture_path(c.input);
        let java = base.join(format!("{}.java", c.class_name));
        let report_json = base.join(format!("{}.report.json", c.class_name));
        let report_md = base.join(format!("{}.report.md", c.class_name));

        let output = Command::new(bin)
            .arg("--input")
            .arg(&input)
            .arg("--output")
            .arg(&java)
            .arg("--report-json")
            .arg(&report_json)
            .arg("--report-md")
            .arg(&report_md)
            .arg("--class-name")
            .arg(c.class_name)
            .arg("--mode")
            .arg(c.mode)
            .arg("--java-target")
            .arg("java21")
            .output()
            .expect("failed to execute binary");

        assert!(
            output.status.success(),
            "expected conversion success for {} ({})\nstderr:\n{}",
            c.class_name,
            c.mode,
            String::from_utf8_lossy(&output.stderr)
        );

        assert_matches_snapshot(&java, &snapshot_path(&format!("{}.java", c.class_name)));
        assert_matches_snapshot(
            &report_json,
            &snapshot_path(&format!("{}.report.json", c.class_name)),
        );
        assert_matches_snapshot(
            &report_md,
            &snapshot_path(&format!("{}.report.md", c.class_name)),
        );
    }
}
