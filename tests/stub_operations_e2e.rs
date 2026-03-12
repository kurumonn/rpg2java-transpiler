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
fn fixed_stub_operations_use_stable_named_todo_args() {
    let base = make_temp_dir("fixed-stub-ops");
    let input = base.join("fixed_stub.rpg");
    let output_json = base.join("FixedStub.report.json");

    fs::write(
        &input,
        "000100C     CUSTKEY       SETLL     CUSTOMER\n\
000200C     CUSTKEY       CHAIN     CUSTOMER\n\
000300C                   READE     CUSTOMER\n\
000400C                   EXSR      LOGERR\n",
    )
    .expect("write input");

    let bin = env!("CARGO_BIN_EXE_rpg2java-transpiler");
    let output = Command::new(bin)
        .arg("--input")
        .arg(&input)
        .arg("--mode")
        .arg("fixed")
        .arg("--report-json")
        .arg(&output_json)
        .arg("--output")
        .arg(base.join("FixedStub.java"))
        .arg("--class-name")
        .arg("FixedStub")
        .output()
        .expect("failed to execute binary");

    assert!(
        output.status.success(),
        "conversion should succeed\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let body = fs::read_to_string(&output_json).expect("read report");
    assert!(
        body.contains("SETLL: key=CUSTKEY, file=CUSTOMER"),
        "body={body}"
    );
    assert!(
        body.contains("CHAIN: key=CUSTKEY, file=CUSTOMER"),
        "body={body}"
    );
    assert!(body.contains("READE: file=CUSTOMER"), "body={body}");
    assert!(body.contains("EXSR: subroutine=LOGERR"), "body={body}");
}

#[test]
fn free_stub_operations_are_counted_in_operation_matrix() {
    let base = make_temp_dir("free-stub-ops");
    let input = base.join("free_stub.rpg");
    let output_json = base.join("FreeStub.report.json");

    fs::write(
        &input,
        "CHAIN CUSTKEY CUSTOMER\n\
SETLL CUSTKEY CUSTOMER\n\
READE CUSTOMER\n\
EXSR LOGERR\n",
    )
    .expect("write input");

    let bin = env!("CARGO_BIN_EXE_rpg2java-transpiler");
    let output = Command::new(bin)
        .arg("--input")
        .arg(&input)
        .arg("--mode")
        .arg("free")
        .arg("--report-json")
        .arg(&output_json)
        .arg("--output")
        .arg(base.join("FreeStub.java"))
        .arg("--class-name")
        .arg("FreeStub")
        .output()
        .expect("failed to execute binary");

    assert!(
        output.status.success(),
        "conversion should succeed\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let body = fs::read_to_string(&output_json).expect("read report");
    assert!(body.contains("\"name\": \"CHAIN\""), "body={body}");
    assert!(body.contains("\"name\": \"SETLL\""), "body={body}");
    assert!(body.contains("\"name\": \"READE\""), "body={body}");
    assert!(body.contains("\"name\": \"EXSR\""), "body={body}");
}
