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
fn unsupported_rpg_tokens_fallback_to_compilable_java_fragments() {
    let base = make_temp_dir("javac-fallback");
    let input = base.join("sample.rpg");
    let output_java = base.join("Fallback.java");

    fs::write(
        &input,
        "EVAL B = 0\nEVAL B = B *UNKNOWN 1\nIF B *MYSTERY 1\nWRITE B\nENDIF\n",
    )
    .expect("failed to write fixture");

    let bin = env!("CARGO_BIN_EXE_rpg2java-transpiler");
    let output = Command::new(bin)
        .arg("--input")
        .arg(&input)
        .arg("--output")
        .arg(&output_java)
        .arg("--class-name")
        .arg("Fallback")
        .arg("--mode")
        .arg("free")
        .output()
        .expect("failed to execute binary");

    assert!(
        output.status.success(),
        "expected conversion success\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let java_body = fs::read_to_string(&output_java).expect("failed to read java output");
    assert!(
        java_body.contains("// TODO(FALLBACK_EXPR): B *UNKNOWN 1"),
        "java={java_body}"
    );
    assert!(
        java_body.contains("// TODO(FALLBACK_COND): B *MYSTERY 1"),
        "java={java_body}"
    );
    assert!(java_body.contains("B = 0;"), "java={java_body}");
    assert!(java_body.contains("if (false) {"), "java={java_body}");
}

#[test]
fn cat_operator_is_translated_to_java_concat() {
    let base = make_temp_dir("cat-operator");
    let input = base.join("sample.rpg");
    let output_java = base.join("Cat.java");

    fs::write(&input, "EVAL MSG = 'A' *CAT 'B'\n").expect("failed to write fixture");

    let bin = env!("CARGO_BIN_EXE_rpg2java-transpiler");
    let output = Command::new(bin)
        .arg("--input")
        .arg(&input)
        .arg("--output")
        .arg(&output_java)
        .arg("--class-name")
        .arg("Cat")
        .arg("--mode")
        .arg("free")
        .output()
        .expect("failed to execute binary");

    assert!(
        output.status.success(),
        "expected conversion success\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let java_body = fs::read_to_string(&output_java).expect("failed to read java output");
    assert!(java_body.contains("String MSG = \"\";"), "java={java_body}");
    assert!(
        java_body.contains("MSG = \"A\" + \"B\";"),
        "java={java_body}"
    );
}
