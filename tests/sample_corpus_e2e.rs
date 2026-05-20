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

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn sample_path(name: &str) -> PathBuf {
    repo_root().join("samples").join(name)
}

fn sample_dir() -> PathBuf {
    repo_root().join("samples")
}

fn convert_sample(
    base: &Path,
    sample_name: &str,
    class_name: &str,
    mode: &str,
) -> (PathBuf, PathBuf, PathBuf, PathBuf) {
    let input = sample_path(sample_name);
    let java = base.join(format!("{class_name}.java"));
    let report_json = base.join(format!("{class_name}.report.json"));
    let report_md = base.join(format!("{class_name}.report.md"));
    let source_map = base.join(format!("{class_name}.source-map.json"));

    let bin = env!("CARGO_BIN_EXE_rpg2java-transpiler");
    let output = Command::new(bin)
        .arg("convert")
        .arg("--input")
        .arg(&input)
        .arg("--output")
        .arg(&java)
        .arg("--class-name")
        .arg(class_name)
        .arg("--mode")
        .arg(mode)
        .arg("--report-json")
        .arg(&report_json)
        .arg("--report-md")
        .arg(&report_md)
        .arg("--source-map")
        .arg(&source_map)
        .output()
        .expect("failed to execute binary");

    assert!(
        output.status.success(),
        "conversion failed for {sample_name}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    (java, report_json, report_md, source_map)
}

fn assert_contains_all(body: &str, snippets: &[&str]) {
    for snippet in snippets {
        assert!(
            body.contains(snippet),
            "missing snippet `{snippet}` in:\n{body}"
        );
    }
}

fn compile_java(java_file: &Path, classes_dir: &Path) {
    fs::create_dir_all(classes_dir).expect("failed to create classes dir");
    let output = Command::new("javac")
        .arg("-encoding")
        .arg("UTF-8")
        .arg("-d")
        .arg(classes_dir)
        .arg(java_file)
        .output()
        .expect("failed to execute javac");

    assert!(
        output.status.success(),
        "javac failed for {}\nstderr:\n{}",
        java_file.display(),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn run_java(classes_dir: &Path, class_name: &str) {
    let output = Command::new("java")
        .arg("-cp")
        .arg(classes_dir)
        .arg(class_name)
        .output()
        .expect("failed to execute java");

    assert!(
        output.status.success(),
        "java failed for {class_name}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn free_basic_and_business_flow_compile_and_run() {
    let base = make_temp_dir("sample-free-roundtrip");
    let cases = [
        (
            "free_basic.rpg",
            "FreeBasic",
            "free",
            &[
                "double CUSTOMER_ID = 0;",
                "String CUSTOMER_NAME = \"\";",
                "double TOTAL = 0;",
                "SEND_NOTICE();",
                "readRecord(CUSTOMER_FILE);",
                "writeRecord(OUTPUT_FILE);",
            ][..],
        ),
        (
            "business_order_flow.rpg",
            "BusinessOrderFlow",
            "free",
            &[
                "double ITEM_COUNT = 0;",
                "double TOTAL = 0;",
                "SEND_ORDER_MAIL();",
                "PRINT_SUMMARY();",
                "writeRecord(ORDER_OUTPUT);",
                "writeRecord(ERROR_OUTPUT);",
            ][..],
        ),
    ];

    for (sample_name, class_name, mode, java_snippets) in cases {
        let (java, report_json, report_md, source_map) =
            convert_sample(&base, sample_name, class_name, mode);
        let java_body = fs::read_to_string(&java).expect("failed to read java output");
        assert_contains_all(&java_body, java_snippets);

        let json_body = fs::read_to_string(&report_json).expect("failed to read json report");
        assert_contains_all(
            &json_body,
            &["\"operation_summary\"", "\"source_map\"", "\"java_target\""],
        );

        let md_body = fs::read_to_string(&report_md).expect("failed to read markdown report");
        assert_contains_all(&md_body, &["## Source Map", "## Operation Matrix"]);

        let source_map_body = fs::read_to_string(&source_map).expect("failed to read source map");
        assert_contains_all(
            &source_map_body,
            &[
                "\"version\": \"rpg2java-source-map.v1\"",
                "\"rpg_line\":",
                "\"java_line\":",
            ],
        );

        let classes_dir = base.join(format!("{class_name}-classes"));
        compile_java(&java, &classes_dir);
        run_java(&classes_dir, class_name);
    }
}

#[test]
fn fixed_basic_and_stub_samples_compile_and_run() {
    let base = make_temp_dir("sample-fixed-roundtrip");
    let cases = [
        (
            "fixed_basic.rpg",
            "FixedBasic",
            "fixed",
            &[
                "SEND_NOTICE();",
                "readRecord(CUSTOMER_FILE);",
                "writeRecord(OUTPUT_FILE);",
                "while (!(",
            ][..],
            &[
                "\"name\": \"CALLP\"",
                "\"name\": \"READ\"",
                "\"name\": \"WRITE\"",
            ][..],
        ),
        (
            "free_with_stub_ops.rpg",
            "FreeWithStubOps",
            "free",
            &["PRINT_ORDER();", "writeRecord(ORDER_OUTPUT);"][..],
            &[
                "CHAIN: key=CUSTOMER_ID, file=CUSTOMER_FILE",
                "SETLL: key=ORDER_ID, file=ORDER_FILE",
                "READE: key=ORDER_ID, file=ORDER_FILE",
                "EXSR: subroutine=CALC_TOTAL",
            ][..],
        ),
        (
            "fixed_stub_ops.rpg",
            "FixedStubOps",
            "fixed",
            &["PRINT_ORDER();", "writeRecord(ORDER_OUTPUT);"][..],
            &[
                "CHAIN: key=CUSTOMER_ID, file=CUSTOMER_FILE",
                "SETLL: key=ORDER_ID, file=ORDER_FILE",
                "READE: key=ORDER_ID, file=ORDER_FILE",
                "EXSR: subroutine=CALC_TOTAL",
            ][..],
        ),
    ];

    for (sample_name, class_name, mode, java_snippets, report_snippets) in cases {
        let (java, report_json, report_md, source_map) =
            convert_sample(&base, sample_name, class_name, mode);
        let java_body = fs::read_to_string(&java).expect("failed to read java output");
        assert_contains_all(&java_body, java_snippets);

        let json_body = fs::read_to_string(&report_json).expect("failed to read json report");
        assert_contains_all(&json_body, report_snippets);

        let md_body = fs::read_to_string(&report_md).expect("failed to read markdown report");
        assert_contains_all(
            &md_body,
            &[
                "## Top Unsupported Operations",
                "## Unsupported / TODO Items",
            ],
        );

        let source_map_body = fs::read_to_string(&source_map).expect("failed to read source map");
        assert_contains_all(
            &source_map_body,
            &["\"version\": \"rpg2java-source-map.v1\""],
        );

        let classes_dir = base.join(format!("{class_name}-classes"));
        compile_java(&java, &classes_dir);
        run_java(&classes_dir, class_name);
    }
}

#[test]
fn hybrid_and_invalid_fixed_samples_surface_expected_diagnostics() {
    let base = make_temp_dir("sample-hybrid-invalid");

    let (hybrid_java, hybrid_json, hybrid_md, hybrid_map) =
        convert_sample(&base, "hybrid_mixed.rpg", "HybridMixed", "auto");
    let hybrid_java_body = fs::read_to_string(&hybrid_java).expect("failed to read java output");
    assert_contains_all(&hybrid_java_body, &["public class HybridMixed"]);
    let hybrid_json_body = fs::read_to_string(&hybrid_json).expect("failed to read json report");
    assert_contains_all(
        &hybrid_json_body,
        &["\"source_map\"", "\"todos\"", "\"diagnostics\""],
    );
    let hybrid_md_body = fs::read_to_string(&hybrid_md).expect("failed to read markdown report");
    assert_contains_all(&hybrid_md_body, &["## Diagnostics", "## Source Map"]);
    let hybrid_map_body = fs::read_to_string(&hybrid_map).expect("failed to read source map");
    assert_contains_all(
        &hybrid_map_body,
        &["\"version\": \"rpg2java-source-map.v1\""],
    );
    let hybrid_classes = base.join("HybridMixed-classes");
    compile_java(&hybrid_java, &hybrid_classes);
    run_java(&hybrid_classes, "HybridMixed");

    let (invalid_java, invalid_json, invalid_md, invalid_map) = convert_sample(
        &base,
        "invalid_fixed_alignment.rpg",
        "InvalidFixedAlignment",
        "fixed",
    );
    let invalid_json_body = fs::read_to_string(&invalid_json).expect("failed to read json report");
    assert_contains_all(
        &invalid_json_body,
        &[
            "PARSER_FIXED_OPCODE_MISALIGNED",
            "PARSER_FIXED_SHORT_LINE",
            "PARSER_FIXED_OPCODE_MISSING",
        ],
    );
    let invalid_md_body = fs::read_to_string(&invalid_md).expect("failed to read markdown report");
    assert_contains_all(&invalid_md_body, &["PARSER_FIXED_OPCODE_MISALIGNED"]);
    let invalid_map_body = fs::read_to_string(&invalid_map).expect("failed to read source map");
    assert_contains_all(
        &invalid_map_body,
        &["\"version\": \"rpg2java-source-map.v1\""],
    );
    let invalid_java_body = fs::read_to_string(&invalid_java).expect("failed to read java output");
    assert_contains_all(&invalid_java_body, &["public class InvalidFixedAlignment"]);
    let invalid_classes = base.join("InvalidFixedAlignment-classes");
    compile_java(&invalid_java, &invalid_classes);
    run_java(&invalid_classes, "InvalidFixedAlignment");
}

#[test]
fn sample_directory_batch_smoke_emits_reports_and_passes_javac() {
    let base = make_temp_dir("sample-batch-smoke");
    let output_dir = base.join("out");

    let bin = env!("CARGO_BIN_EXE_rpg2java-transpiler");
    let output = Command::new(bin)
        .arg("batch")
        .arg("--batch-dir")
        .arg(sample_dir())
        .arg("--output-dir")
        .arg(&output_dir)
        .arg("--html-report")
        .arg("--source-map-report")
        .arg("--javac-check")
        .arg("--fail-on-todo-rate")
        .arg("1.0")
        .output()
        .expect("failed to execute binary");

    assert!(
        output.status.success(),
        "batch sample smoke failed\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("batch complete:"), "stderr={stderr}");
    assert!(stderr.contains("success=7"), "stderr={stderr}");
    assert!(stderr.contains("failed=0"), "stderr={stderr}");

    let expected_classes = [
        "BusinessOrderFlow",
        "FixedBasic",
        "FixedStubOps",
        "FreeBasic",
        "FreeWithStubOps",
        "HybridMixed",
        "InvalidFixedAlignment",
    ];

    for class_name in expected_classes {
        assert!(
            output_dir.join(format!("{class_name}.java")).exists(),
            "missing java output for {class_name}"
        );
        assert!(
            output_dir
                .join(format!("{class_name}.report.json"))
                .exists(),
            "missing json report for {class_name}"
        );
        assert!(
            output_dir.join(format!("{class_name}.report.md")).exists(),
            "missing markdown report for {class_name}"
        );
        assert!(
            output_dir
                .join(format!("{class_name}.report.html"))
                .exists(),
            "missing html report for {class_name}"
        );
        assert!(
            output_dir
                .join(format!("{class_name}.source-map.json"))
                .exists(),
            "missing source map for {class_name}"
        );
    }
}
