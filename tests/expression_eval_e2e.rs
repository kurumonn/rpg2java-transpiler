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
fn logical_and_comparison_expression_translates_to_java_ops() {
    let base = make_temp_dir("expr-logical-comparator");
    let input = base.join("logical.rpg");
    let output_java = base.join("Logical.java");

    fs::write(
        &input,
        "EVAL A = 10\n\
EVAL B = 20\n\
IF A *GT 5 *AND B *LT 30\n\
CALLP OK_PROC\n\
ENDIF\n",
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
        .arg("Logical")
        .output()
        .expect("failed to execute binary");

    assert!(
        output.status.success(),
        "conversion should succeed\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let java = fs::read_to_string(&output_java).expect("read java");
    assert!(java.contains("if (A > 5 && B < 30) {"), "java={java}");
}

#[test]
fn string_condition_and_literals_become_valid_java_strings() {
    let base = make_temp_dir("expr-string-condition");
    let input = base.join("string_cond.rpg");
    let output_java = base.join("StringCond.java");

    fs::write(
        &input,
        "EVAL STATUS = 'OK'\n\
IF STATUS *EQ 'OK' *OR STATUS *EQ 'WARN'\n\
CALLP HANDLE\n\
ENDIF\n\
EVAL FLAG = *OFF\n",
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
        .arg("StringCond")
        .output()
        .expect("failed to execute binary");

    assert!(
        output.status.success(),
        "conversion should succeed\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let java = fs::read_to_string(&output_java).expect("read java");
    assert!(java.contains("STATUS = \"OK\";"), "java={java}");
    assert!(
        java.contains("if (STATUS == \"OK\" || STATUS == \"WARN\") {"),
        "java={java}"
    );
    assert!(java.contains("FLAG = false;"), "java={java}");
    assert!(
        !java.contains("((Number)STATUS).doubleValue()"),
        "string comparator should not force numeric cast: {java}"
    );
}
