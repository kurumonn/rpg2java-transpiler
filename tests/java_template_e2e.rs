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
fn symbols_do_not_collide_with_class_name_or_main_args() {
    let base = make_temp_dir("java-template-symbol-collision");
    let input = base.join("sample.rpg");
    let output_java = base.join("Collision.java");

    fs::write(
        &input,
        "EVAL args = 1\nEVAL Collision = args\nIF Collision *GT 0\nENDIF\n",
    )
    .expect("failed to write fixture");

    let bin = env!("CARGO_BIN_EXE_rpg2java-transpiler");
    let output = Command::new(bin)
        .arg("--input")
        .arg(&input)
        .arg("--output")
        .arg(&output_java)
        .arg("--class-name")
        .arg("Collision")
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
    assert!(java_body.contains("public class Collision"));
    assert!(java_body.contains("double args_2 = 0;"), "java={java_body}");
    assert!(java_body.contains("double Collision_2 = 0;"), "java={java_body}");
    assert!(java_body.contains("Collision_2 = args_2;"), "java={java_body}");
}

#[test]
fn callp_method_names_do_not_collide_with_builtin_helpers() {
    let base = make_temp_dir("java-template-callp-collision");
    let input = base.join("sample.rpg");
    let output_java = base.join("CallpCollision.java");

    fs::write(
        &input,
        "CALLP writeRecord\nCALLP readRecord\nCALLP truthy\nCALLP main\n",
    )
    .expect("failed to write fixture");

    let bin = env!("CARGO_BIN_EXE_rpg2java-transpiler");
    let output = Command::new(bin)
        .arg("--input")
        .arg(&input)
        .arg("--output")
        .arg(&output_java)
        .arg("--class-name")
        .arg("CallpCollision")
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
    assert!(java_body.contains("writeRecord_2();"), "java={java_body}");
    assert!(java_body.contains("readRecord_2();"), "java={java_body}");
    assert!(java_body.contains("truthy_2();"), "java={java_body}");
    assert!(java_body.contains("main_2();"), "java={java_body}");
    assert!(
        java_body.contains("private static void writeRecord_2()"),
        "java={java_body}"
    );
    assert!(
        java_body.contains("private static void readRecord_2()"),
        "java={java_body}"
    );
    assert!(
        java_body.contains("private static void truthy_2()"),
        "java={java_body}"
    );
    assert!(
        java_body.contains("private static void main_2()"),
        "java={java_body}"
    );
    assert!(
        java_body.contains("private static void writeRecord(Object rec)"),
        "java={java_body}"
    );
    assert!(
        java_body.contains("private static void readRecord(Object rec)"),
        "java={java_body}"
    );
    assert!(
        java_body.contains("private static boolean truthy(Object v)"),
        "java={java_body}"
    );
}
