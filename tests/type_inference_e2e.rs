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
fn type_inference_propagates_number_text_bool_across_assignments() {
    let base = make_temp_dir("type-infer-propagation");
    let input = base.join("propagation.rpg");
    let output_java = base.join("Propagation.java");

    fs::write(
        &input,
        "EVAL SRC_N = 10\n\
EVAL DST_N = SRC_N\n\
EVAL SRC_T = 'ABC'\n\
EVAL DST_T = SRC_T\n\
EVAL SRC_B = *ON\n\
EVAL DST_B = SRC_B\n",
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
        .arg("--class-name")
        .arg("Propagation")
        .output()
        .expect("failed to execute binary");

    assert!(
        output.status.success(),
        "conversion should succeed\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let java = fs::read_to_string(&output_java).expect("read java");
    assert!(java.contains("double SRC_N = 0;"), "java={java}");
    assert!(java.contains("double DST_N = 0;"), "java={java}");
    assert!(java.contains("String SRC_T = \"\";"), "java={java}");
    assert!(java.contains("String DST_T = \"\";"), "java={java}");
    assert!(java.contains("boolean SRC_B = false;"), "java={java}");
    assert!(java.contains("boolean DST_B = false;"), "java={java}");
    assert!(java.contains("SRC_B = true;"), "java={java}");
}

#[test]
fn eq_condition_with_string_literal_marks_symbol_as_text() {
    let base = make_temp_dir("type-infer-eq-string");
    let input = base.join("eq_string.rpg");
    let output_java = base.join("EqString.java");

    fs::write(&input, "IF STATUS *EQ 'OK'\nENDIF\n").expect("write input");

    let bin = env!("CARGO_BIN_EXE_rpg2java-transpiler");
    let output = Command::new(bin)
        .arg("--input")
        .arg(&input)
        .arg("--mode")
        .arg("free")
        .arg("--output")
        .arg(&output_java)
        .arg("--class-name")
        .arg("EqString")
        .output()
        .expect("failed to execute binary");

    assert!(
        output.status.success(),
        "conversion should succeed\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let java = fs::read_to_string(&output_java).expect("read java");
    assert!(java.contains("String STATUS = \"\";"), "java={java}");
    assert!(!java.contains("double STATUS = 0;"), "java={java}");
}
