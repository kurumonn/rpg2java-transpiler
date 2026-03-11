use std::fs;
use std::path::Path;

use crate::ir::{IrProgram, IrStmt};
use crate::parser::{Program, SupportLevel};

#[derive(Debug, Clone)]
pub struct JavacCheckSummary {
    pub command: String,
    pub success: bool,
    pub exit_code: Option<i32>,
    pub detail: String,
}

pub fn write_reports(
    program: &Program,
    ir_program: &IrProgram,
    java_target: &str,
    javac_check: Option<&JavacCheckSummary>,
    json_path: Option<&Path>,
    md_path: Option<&Path>,
) -> Result<(), String> {
    if let Some(path) = json_path {
        let body = render_json(program, ir_program, java_target, javac_check);
        fs::write(path, body)
            .map_err(|e| format!("failed to write JSON report {}: {e}", path.display()))?;
    }
    if let Some(path) = md_path {
        let body = render_markdown(program, ir_program, java_target, javac_check);
        fs::write(path, body)
            .map_err(|e| format!("failed to write Markdown report {}: {e}", path.display()))?;
    }
    Ok(())
}

fn render_json(
    program: &Program,
    ir_program: &IrProgram,
    java_target: &str,
    javac_check: Option<&JavacCheckSummary>,
) -> String {
    let implemented = program
        .op_stats
        .iter()
        .filter(|s| s.level == SupportLevel::Implemented)
        .count();
    let stub = program
        .op_stats
        .iter()
        .filter(|s| s.level == SupportLevel::Stub)
        .count();
    let planned = program
        .op_stats
        .iter()
        .filter(|s| s.level == SupportLevel::Planned)
        .count();
    let todos = collect_todos(ir_program);

    let mut out = String::new();
    out.push_str("{\n");
    out.push_str(&format!(
        "  \"java_target\": \"{}\",\n",
        escape_json(java_target)
    ));
    if let Some(check) = javac_check {
        out.push_str("  \"javac_check\": {\n");
        out.push_str(&format!(
            "    \"command\": \"{}\",\n",
            escape_json(&check.command)
        ));
        out.push_str(&format!("    \"success\": {},\n", check.success));
        if let Some(code) = check.exit_code {
            out.push_str(&format!("    \"exit_code\": {},\n", code));
        } else {
            out.push_str("    \"exit_code\": null,\n");
        }
        out.push_str(&format!(
            "    \"detail\": \"{}\"\n",
            escape_json(&check.detail)
        ));
        out.push_str("  },\n");
    } else {
        out.push_str("  \"javac_check\": null,\n");
    }
    out.push_str(&format!(
        "  \"total_statements\": {},\n",
        program.statements.len()
    ));
    out.push_str(&format!(
        "  \"total_symbols\": {},\n",
        ir_program.symbols.len()
    ));
    out.push_str("  \"operation_summary\": {\n");
    out.push_str(&format!("    \"implemented\": {},\n", implemented));
    out.push_str(&format!("    \"stub\": {},\n", stub));
    out.push_str(&format!("    \"planned\": {}\n", planned));
    out.push_str("  },\n");
    out.push_str("  \"operations\": [\n");
    for (idx, stat) in program.op_stats.iter().enumerate() {
        let comma = if idx + 1 == program.op_stats.len() {
            ""
        } else {
            ","
        };
        out.push_str(&format!(
            "    {{\"name\": \"{}\", \"count\": {}, \"level\": \"{}\"}}{}\n",
            escape_json(&stat.name),
            stat.count,
            level_name(stat.level),
            comma
        ));
    }
    out.push_str("  ],\n");
    out.push_str("  \"todos\": [\n");
    for (idx, todo) in todos.iter().enumerate() {
        let comma = if idx + 1 == todos.len() { "" } else { "," };
        out.push_str(&format!("    \"{}\"{}\n", escape_json(todo), comma));
    }
    out.push_str("  ],\n");
    out.push_str("  \"diagnostics\": [\n");
    for (idx, d) in program.diagnostics.iter().enumerate() {
        let comma = if idx + 1 == program.diagnostics.len() {
            ""
        } else {
            ","
        };
        out.push_str(&format!("    \"{}\"{}\n", escape_json(d), comma));
    }
    out.push_str("  ]\n");
    out.push_str("}\n");
    out
}

fn render_markdown(
    program: &Program,
    ir_program: &IrProgram,
    java_target: &str,
    javac_check: Option<&JavacCheckSummary>,
) -> String {
    let mut out = String::new();
    out.push_str("# RPG to Java Migration Report\n\n");
    out.push_str("## Summary\n\n");
    out.push_str(&format!("- Java Target: {}\n", java_target));
    if let Some(check) = javac_check {
        let code = check
            .exit_code
            .map(|v| v.to_string())
            .unwrap_or_else(|| String::from("n/a"));
        out.push_str(&format!(
            "- Javac Check: {} (command: {}, exit_code: {})\n",
            if check.success { "success" } else { "failed" },
            check.command,
            code
        ));
        if !check.detail.is_empty() {
            out.push_str(&format!("- Javac Detail: {}\n", check.detail));
        }
    } else {
        out.push_str("- Javac Check: skipped\n");
    }
    out.push_str(&format!("- Statements: {}\n", program.statements.len()));
    out.push_str(&format!("- Symbols: {}\n", ir_program.symbols.len()));
    out.push_str(&format!("- Diagnostics: {}\n", program.diagnostics.len()));
    out.push('\n');

    out.push_str("## Operation Matrix\n\n");
    out.push_str("| Operation | Count | Level |\n");
    out.push_str("|---|---:|---|\n");
    for stat in &program.op_stats {
        out.push_str(&format!(
            "| {} | {} | {} |\n",
            stat.name,
            stat.count,
            level_name(stat.level)
        ));
    }
    out.push('\n');

    let todos = collect_todos(ir_program);
    out.push_str("## Unsupported / TODO Items\n\n");
    if todos.is_empty() {
        out.push_str("- none\n");
    } else {
        for t in &todos {
            out.push_str(&format!("- {}\n", t));
        }
    }
    out.push('\n');

    out.push_str("## Diagnostics\n\n");
    if program.diagnostics.is_empty() {
        out.push_str("- none\n");
    } else {
        for d in &program.diagnostics {
            out.push_str(&format!("- {}\n", d));
        }
    }
    out
}

fn collect_todos(ir_program: &IrProgram) -> Vec<String> {
    let mut todos = Vec::new();
    for stmt in &ir_program.statements {
        if let IrStmt::Todo { op, src } = stmt {
            if let Some(op) = op {
                todos.push(format!("{op}: {src}"));
            } else {
                todos.push(src.clone());
            }
        }
    }
    todos
}

fn level_name(level: SupportLevel) -> &'static str {
    match level {
        SupportLevel::Implemented => "implemented",
        SupportLevel::Stub => "stub",
        SupportLevel::Planned => "planned",
    }
}

fn escape_json(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}
