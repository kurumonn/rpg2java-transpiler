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
fn fixed_exsr_routes_are_reported_in_json_and_markdown() {
    let base = make_temp_dir("fixed-exsr-routes");
    let input = base.join("fixed_exsr.rpg");
    let output_java = base.join("FixedExsr.java");
    let output_json = base.join("FixedExsr.report.json");
    let output_md = base.join("FixedExsr.report.md");

    fs::write(
        &input,
        "000100C                   EXSR      LOGERR\n\
000200C                   EXSR      RETRY01\n",
    )
    .expect("write input");

    let bin = env!("CARGO_BIN_EXE_rpg2java-transpiler");
    let output = Command::new(bin)
        .arg("--input")
        .arg(&input)
        .arg("--mode")
        .arg("fixed")
        .arg("--output")
        .arg(&output_java)
        .arg("--report-json")
        .arg(&output_json)
        .arg("--report-md")
        .arg(&output_md)
        .arg("--class-name")
        .arg("FixedExsr")
        .output()
        .expect("failed to execute binary");

    assert!(
        output.status.success(),
        "conversion should succeed\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let json = fs::read_to_string(&output_json).expect("read report json");
    assert!(json.contains("\"subroutine_routes\""), "json={json}");
    assert!(
        json.contains("\"caller\":\"MAIN\",\"callee\":\"LOGERR\",\"line\":1"),
        "json={json}"
    );
    assert!(
        json.contains("\"caller\":\"MAIN\",\"callee\":\"RETRY01\",\"line\":2"),
        "json={json}"
    );

    let md = fs::read_to_string(&output_md).expect("read report markdown");
    assert!(md.contains("## Subroutine Routes"), "md={md}");
    assert!(md.contains("MAIN -> LOGERR (line 1)"), "md={md}");
    assert!(md.contains("MAIN -> RETRY01 (line 2)"), "md={md}");
}

#[test]
fn free_exsr_route_is_reported() {
    let base = make_temp_dir("free-exsr-route");
    let input = base.join("free_exsr.rpg");
    let output_json = base.join("FreeExsr.report.json");

    fs::write(&input, "EXSR RECOVER\n").expect("write input");

    let bin = env!("CARGO_BIN_EXE_rpg2java-transpiler");
    let output = Command::new(bin)
        .arg("--input")
        .arg(&input)
        .arg("--mode")
        .arg("free")
        .arg("--output")
        .arg(base.join("FreeExsr.java"))
        .arg("--report-json")
        .arg(&output_json)
        .arg("--class-name")
        .arg("FreeExsr")
        .output()
        .expect("failed to execute binary");

    assert!(
        output.status.success(),
        "conversion should succeed\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let json = fs::read_to_string(&output_json).expect("read report json");
    assert!(
        json.contains("\"caller\":\"MAIN\",\"callee\":\"RECOVER\",\"line\":1"),
        "json={json}"
    );
}
