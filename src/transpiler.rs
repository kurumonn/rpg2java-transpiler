use std::collections::{BTreeMap, BTreeSet};

use crate::ir::{IrProgram, IrStmt, ValueType};

pub fn to_java(program: &IrProgram, class_name: &str) -> String {
    let mut out = String::new();
    let class_name = sanitize_ident(class_name);
    let name_map = build_name_map(program);
    let mut type_map: BTreeMap<String, ValueType> = BTreeMap::new();
    for symbol in &program.symbols {
        type_map.insert(symbol.name.clone(), symbol.ty);
    }

    out.push_str("public class ");
    out.push_str(&class_name);
    out.push_str(" {\n");
    out.push_str("    public static void main(String[] args) {\n");

    for symbol in &program.symbols {
        let java_name = map_ident(&symbol.name, &name_map);
        let ty = match symbol.ty {
            ValueType::Number => "double",
            ValueType::Text => "String",
            ValueType::Bool => "boolean",
            ValueType::Unknown => "Object",
        };
        let init = match symbol.ty {
            ValueType::Number => " = 0;",
            ValueType::Text => " = \"\";",
            ValueType::Bool => " = false;",
            ValueType::Unknown => ";",
        };
        push_line(&mut out, 2, &format!("{ty} {java_name}{init}"));
    }
    if !program.symbols.is_empty() {
        out.push('\n');
    }

    let mut called_methods: BTreeSet<String> = BTreeSet::new();
    let mut indent = 2usize;
    for stmt in &program.statements {
        match stmt {
            IrStmt::Assign { left, right } => {
                let left_name = map_ident(left, &name_map);
                let left_ty = type_map.get(left).copied().unwrap_or(ValueType::Unknown);
                let right_expr = translate_expr(right, &name_map, &type_map, left_ty == ValueType::Number);
                push_line(&mut out, indent, &format!("{left_name} = {right_expr};"));
            }
            IrStmt::If { cond } => {
                let cond = translate_cond(cond, &name_map, &type_map);
                push_line(&mut out, indent, &format!("if ({cond}) {{"));
                indent += 1;
            }
            IrStmt::Else => {
                indent = indent.saturating_sub(1);
                push_line(&mut out, indent, "} else {");
                indent += 1;
            }
            IrStmt::EndIf => {
                indent = indent.saturating_sub(1);
                push_line(&mut out, indent, "}");
            }
            IrStmt::DoUntil { cond } => {
                let cond = translate_cond(cond, &name_map, &type_map);
                push_line(&mut out, indent, &format!("while (!({cond})) {{"));
                indent += 1;
            }
            IrStmt::EndDo => {
                indent = indent.saturating_sub(1);
                push_line(&mut out, indent, "}");
            }
            IrStmt::Call { proc_name } => {
                let proc = sanitize_ident(proc_name);
                called_methods.insert(proc.clone());
                push_line(&mut out, indent, &format!("{proc}();"));
            }
            IrStmt::Write { target } => {
                let target = map_ident(target, &name_map);
                push_line(&mut out, indent, &format!("writeRecord({target});"));
            }
            IrStmt::Read { target } => {
                let target = map_ident(target, &name_map);
                push_line(&mut out, indent, &format!("readRecord({target});"));
            }
            IrStmt::Todo { op, src } => {
                if let Some(op) = op {
                    push_line(&mut out, indent, &format!("// TODO({op}): {src}"));
                } else {
                    push_line(&mut out, indent, &format!("// TODO: {src}"));
                }
            }
        }
    }

    while indent > 2 {
        indent -= 1;
        push_line(&mut out, indent, "}");
    }

    out.push_str("    }\n");
    out.push_str("\n");
    for proc in called_methods {
        out.push_str("    private static void ");
        out.push_str(&proc);
        out.push_str("() {\n");
        out.push_str("        // TODO: migrated from CALLP\n");
        out.push_str("    }\n\n");
    }
    out.push_str("    private static void writeRecord(Object rec) {\n");
    out.push_str("        // TODO: replace with repository/output adapter\n");
    out.push_str("    }\n");
    out.push_str("\n");
    out.push_str("    private static void readRecord(Object rec) {\n");
    out.push_str("        // TODO: replace with repository/input adapter\n");
    out.push_str("    }\n");
    out.push_str("\n");
    out.push_str("    private static boolean truthy(Object v) {\n");
    out.push_str("        if (v == null) return false;\n");
    out.push_str("        if (v instanceof Boolean) return (Boolean) v;\n");
    out.push_str("        if (v instanceof Number) return ((Number) v).doubleValue() != 0.0;\n");
    out.push_str("        return !v.toString().isBlank();\n");
    out.push_str("    }\n");
    out.push_str("}\n");
    out
}

fn push_line(buf: &mut String, indent: usize, line: &str) {
    buf.push_str(&"    ".repeat(indent));
    buf.push_str(line);
    buf.push('\n');
}

fn normalize_cond(cond: &str) -> String {
    cond.replace(" *EQ ", " == ")
        .replace(" *NE ", " != ")
        .replace(" *GT ", " > ")
        .replace(" *LT ", " < ")
        .replace(" *GE ", " >= ")
        .replace(" *LE ", " <= ")
}

fn translate_cond(
    cond: &str,
    name_map: &BTreeMap<String, String>,
    type_map: &BTreeMap<String, ValueType>,
) -> String {
    let c = normalize_cond(cond);
    if contains_comparator(&c) {
        return translate_expr(&c, name_map, type_map, true);
    }
    let expr = translate_expr(&c, name_map, type_map, false);
    format!("truthy({expr})")
}

fn translate_expr(
    expr: &str,
    name_map: &BTreeMap<String, String>,
    type_map: &BTreeMap<String, ValueType>,
    numeric_context: bool,
) -> String {
    let mut out = String::new();
    let chars: Vec<char> = expr.chars().collect();
    let mut i = 0usize;

    while i < chars.len() {
        let ch = chars[i];
        if ch.is_ascii_alphabetic() || ch == '_' {
            let start = i;
            i += 1;
            while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let raw: String = chars[start..i].iter().collect();
            let mapped = map_ident(&raw, name_map);
            if numeric_context {
                let ty = type_map.get(&raw).copied().unwrap_or(ValueType::Unknown);
                if ty == ValueType::Number {
                    out.push_str(&mapped);
                } else {
                    out.push_str("((Number)");
                    out.push_str(&mapped);
                    out.push_str(").doubleValue()");
                }
            } else {
                out.push_str(&mapped);
            }
            continue;
        }
        out.push(ch);
        i += 1;
    }
    out
}

fn contains_comparator(expr: &str) -> bool {
    expr.contains("==")
        || expr.contains("!=")
        || expr.contains(">=")
        || expr.contains("<=")
        || expr.contains('>')
        || expr.contains('<')
}

fn build_name_map(program: &IrProgram) -> BTreeMap<String, String> {
    let mut used = BTreeSet::new();
    let mut map = BTreeMap::new();
    for symbol in &program.symbols {
        let mut base = sanitize_ident(&symbol.name);
        if base == "main" {
            base = String::from("mainValue");
        }
        let mut candidate = base.clone();
        let mut idx = 1usize;
        while used.contains(&candidate) {
            idx += 1;
            candidate = format!("{base}_{idx}");
        }
        used.insert(candidate.clone());
        map.insert(symbol.name.clone(), candidate);
    }
    map
}

fn map_ident(raw: &str, map: &BTreeMap<String, String>) -> String {
    if let Some(m) = map.get(raw) {
        return m.clone();
    }
    sanitize_ident(raw)
}

fn sanitize_ident(raw: &str) -> String {
    let mut out = String::new();
    for (i, c) in raw.chars().enumerate() {
        let valid = c.is_ascii_alphanumeric() || c == '_';
        if valid {
            if i == 0 && c.is_ascii_digit() {
                out.push('_');
            }
            out.push(c);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        String::from("value")
    } else {
        out
    }
}
