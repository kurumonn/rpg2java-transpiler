use std::fs;
use std::path::Path;

use crate::ir::{IrProgram, IrStmt, ValueType};
use crate::parser::{Diagnostic, Program, SubroutineRoute, SupportLevel};

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
    out.push_str("  \"symbols\": [\n");
    for (idx, sym) in ir_program.symbols.iter().enumerate() {
        let comma = if idx + 1 == ir_program.symbols.len() {
            ""
        } else {
            ","
        };
        out.push_str(&format!(
            "    {{\"name\":\"{}\",\"type\":\"{}\",\"assigned_lines\":{},\"referenced_lines\":{}}}{}\n",
            escape_json(&sym.name),
            value_type_name(sym.ty),
            render_lines_json(&sym.assigned_lines),
            render_lines_json(&sym.referenced_lines),
            comma
        ));
    }
    out.push_str("  ],\n");
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
    out.push_str("  \"subroutine_routes\": [\n");
    for (idx, route) in program.subroutine_routes.iter().enumerate() {
        let comma = if idx + 1 == program.subroutine_routes.len() {
            ""
        } else {
            ","
        };
        out.push_str(&format!(
            "    {{\"caller\":\"{}\",\"callee\":\"{}\",\"line\":{}}}{}\n",
            escape_json(&route.caller),
            escape_json(&route.callee),
            route.line,
            comma
        ));
    }
    out.push_str("  ],\n");
    out.push_str("  \"diagnostics\": [\n");
    for (idx, d) in program.diagnostics.iter().enumerate() {
        let comma = if idx + 1 == program.diagnostics.len() {
            ""
        } else {
            ","
        };
        out.push_str(&format!(
            "    {{\"severity\":\"{}\",\"code\":\"{}\",\"line\":{},\"column\":{},\"message\":\"{}\"}}{}\n",
            d.severity.as_str(),
            escape_json(&d.code),
            opt_usize_json(d.line),
            opt_usize_json(d.column),
            escape_json(&d.message),
            comma
        ));
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

    out.push_str("## Symbols\n\n");
    out.push_str("| Symbol | Type | Assigned Lines | Referenced Lines |\n");
    out.push_str("|---|---|---|---|\n");
    for sym in &ir_program.symbols {
        out.push_str(&format!(
            "| {} | {} | {} | {} |\n",
            sym.name,
            value_type_name(sym.ty),
            lines_for_md(&sym.assigned_lines),
            lines_for_md(&sym.referenced_lines)
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

    out.push_str("## Subroutine Routes\n\n");
    if program.subroutine_routes.is_empty() {
        out.push_str("- none\n");
    } else {
        for route in &program.subroutine_routes {
            out.push_str(&format!("- {}\n", format_subroute_markdown(route)));
        }
    }
    out.push('\n');

    out.push_str("## Diagnostics\n\n");
    if program.diagnostics.is_empty() {
        out.push_str("- none\n");
    } else {
        for d in &program.diagnostics {
            out.push_str(&format!("- {}\n", format_diagnostic_markdown(d)));
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

fn opt_usize_json(value: Option<usize>) -> String {
    match value {
        Some(v) => v.to_string(),
        None => String::from("null"),
    }
}

fn format_diagnostic_markdown(d: &Diagnostic) -> String {
    let mut out = format!("[{}][{}]", d.severity.as_str(), d.code);
    if let Some(line) = d.line {
        out.push_str(&format!(" line {}", line));
        if let Some(col) = d.column {
            out.push_str(&format!(" col {}", col));
        }
    }
    out.push_str(&format!(": {}", d.message));
    out
}

fn format_subroute_markdown(route: &SubroutineRoute) -> String {
    format!("{} -> {} (line {})", route.caller, route.callee, route.line)
}

fn value_type_name(ty: ValueType) -> &'static str {
    match ty {
        ValueType::Number => "number",
        ValueType::Text => "text",
        ValueType::Bool => "bool",
        ValueType::Unknown => "unknown",
    }
}

fn render_lines_json(lines: &[usize]) -> String {
    let mut out = String::from("[");
    for (idx, line) in lines.iter().enumerate() {
        if idx > 0 {
            out.push(',');
        }
        out.push_str(&line.to_string());
    }
    out.push(']');
    out
}

fn lines_for_md(lines: &[usize]) -> String {
    if lines.is_empty() {
        return String::from("-");
    }
    lines
        .iter()
        .map(|v| v.to_string())
        .collect::<Vec<_>>()
        .join(",")
}
