use std::collections::{BTreeMap, BTreeSet};

use crate::parser::{Program, Stmt};

#[derive(Debug, Clone)]
pub struct IrProgram {
    pub statements: Vec<IrStmt>,
    pub symbols: Vec<SymbolInfo>,
}

#[derive(Debug, Clone)]
pub struct SymbolInfo {
    pub name: String,
    pub ty: ValueType,
    pub assigned_lines: Vec<usize>,
    pub referenced_lines: Vec<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueType {
    Number,
    Text,
    Bool,
    Unknown,
}

#[derive(Debug, Clone)]
pub enum IrStmt {
    Assign { left: String, right: String },
    If { cond: String },
    Else,
    EndIf,
    DoUntil { cond: String },
    EndDo,
    Call { proc_name: String },
    Write { target: String },
    Read { target: String },
    Todo { op: Option<String>, src: String },
}

pub fn build_ir(program: &Program) -> IrProgram {
    let mut ir_statements = Vec::new();
    let mut assigned: BTreeMap<String, ValueType> = BTreeMap::new();
    let mut assigned_lines_map: BTreeMap<String, BTreeSet<usize>> = BTreeMap::new();
    let mut referenced_lines_map: BTreeMap<String, BTreeSet<usize>> = BTreeMap::new();

    for (idx, stmt) in program.statements.iter().enumerate() {
        let line = *program.statement_lines.get(idx).unwrap_or(&0);
        match stmt {
            Stmt::Assign { left, right } => {
                let inferred = infer_expr_type(right, &assigned);
                if inferred != ValueType::Unknown || !assigned.contains_key(left) {
                    assigned.insert(left.clone(), inferred);
                }
                assigned_lines_map
                    .entry(left.clone())
                    .or_default()
                    .insert(line);
                mark_referenced_in_expr(right, line, &mut referenced_lines_map);
                if inferred == ValueType::Number {
                    hint_numeric_vars(right, &mut assigned);
                }
                ir_statements.push(IrStmt::Assign {
                    left: left.clone(),
                    right: right.clone(),
                });
            }
            Stmt::If { cond } => {
                mark_referenced_in_expr(cond, line, &mut referenced_lines_map);
                hint_condition_types(cond, &mut assigned);
                ir_statements.push(IrStmt::If { cond: cond.clone() });
            }
            Stmt::Else => ir_statements.push(IrStmt::Else),
            Stmt::EndIf => ir_statements.push(IrStmt::EndIf),
            Stmt::DoUntil { cond } => {
                mark_referenced_in_expr(cond, line, &mut referenced_lines_map);
                hint_condition_types(cond, &mut assigned);
                ir_statements.push(IrStmt::DoUntil { cond: cond.clone() });
            }
            Stmt::EndDo => ir_statements.push(IrStmt::EndDo),
            Stmt::Call { proc_name } => ir_statements.push(IrStmt::Call {
                proc_name: proc_name.clone(),
            }),
            Stmt::Write { target } => {
                referenced_lines_map
                    .entry(target.clone())
                    .or_default()
                    .insert(line);
                ir_statements.push(IrStmt::Write {
                    target: target.clone(),
                });
            }
            Stmt::Read { target } => {
                referenced_lines_map
                    .entry(target.clone())
                    .or_default()
                    .insert(line);
                ir_statements.push(IrStmt::Read {
                    target: target.clone(),
                });
            }
            Stmt::Operation { op, args } => {
                mark_referenced_in_expr(args, line, &mut referenced_lines_map);
                ir_statements.push(IrStmt::Todo {
                    op: Some(op.clone()),
                    src: args.clone(),
                });
            }
            Stmt::Raw(src) => ir_statements.push(IrStmt::Todo {
                op: None,
                src: src.clone(),
            }),
        }
    }

    let mut symbols = Vec::new();
    let mut all_names: BTreeSet<String> = BTreeSet::new();
    all_names.extend(assigned.keys().cloned());
    all_names.extend(assigned_lines_map.keys().cloned());
    all_names.extend(referenced_lines_map.keys().cloned());

    for name in all_names {
        let ty = assigned.get(&name).copied().unwrap_or(ValueType::Unknown);
        let assigned_lines = assigned_lines_map
            .get(&name)
            .map(|s| s.iter().copied().collect())
            .unwrap_or_default();
        let referenced_lines = referenced_lines_map
            .get(&name)
            .map(|s| s.iter().copied().collect())
            .unwrap_or_default();
        symbols.push(SymbolInfo {
            name,
            ty,
            assigned_lines,
            referenced_lines,
        });
    }

    IrProgram {
        statements: ir_statements,
        symbols,
    }
}

fn mark_referenced_in_expr(
    expr: &str,
    line: usize,
    referenced_lines_map: &mut BTreeMap<String, BTreeSet<usize>>,
) {
    let mut vars = BTreeSet::new();
    collect_vars(expr, &mut vars);
    for v in vars {
        referenced_lines_map.entry(v).or_default().insert(line);
    }
}

fn infer_expr_type(expr: &str, assigned: &BTreeMap<String, ValueType>) -> ValueType {
    let e = expr.trim();
    if e.is_empty() {
        return ValueType::Unknown;
    }
    if let Some(lit_ty) = infer_literal_type(e) {
        return lit_ty;
    }
    if let Some(ident) = extract_single_identifier(e) {
        if is_indicator_name(ident) {
            return ValueType::Bool;
        }
        if let Some(ty) = assigned.get(ident) {
            return *ty;
        }
    }

    let upper = e.to_ascii_uppercase();
    if upper.contains(" *CAT ") || e.contains("||") {
        return ValueType::Text;
    }
    if contains_comparator_expr(&upper, e) || contains_logical_expr(&upper) {
        return ValueType::Bool;
    }

    if e.starts_with('\'') && e.ends_with('\'') && e.len() >= 2 {
        return ValueType::Text;
    }
    if e.eq_ignore_ascii_case("true") || e.eq_ignore_ascii_case("false") {
        return ValueType::Bool;
    }
    if e.parse::<i64>().is_ok() || e.parse::<f64>().is_ok() {
        return ValueType::Number;
    }
    if e.contains('+') || e.contains('-') || e.contains('*') || e.contains('/') {
        return ValueType::Number;
    }
    ValueType::Unknown
}

fn collect_vars(expr: &str, acc: &mut BTreeSet<String>) {
    let chars: Vec<char> = expr.chars().collect();
    let mut i = 0usize;
    while i < chars.len() {
        if chars[i] == '\'' || chars[i] == '"' {
            let quote = chars[i];
            i += 1;
            while i < chars.len() && chars[i] != quote {
                i += 1;
            }
            if i < chars.len() {
                i += 1;
            }
            continue;
        }

        while i < chars.len()
            && !(chars[i].is_ascii_alphanumeric() || chars[i] == '_' || chars[i] == '*')
        {
            i += 1;
        }
        if i >= chars.len() {
            break;
        }

        let mut starred = false;
        if chars[i] == '*' {
            starred = true;
            i += 1;
        }
        let start = i;
        while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
            i += 1;
        }
        if start >= i {
            continue;
        }

        let tok: String = chars[start..i].iter().collect();
        let upper = tok.to_ascii_uppercase();
        if starred {
            if upper == "ON" || upper == "OFF" || upper == "BLANK" || upper == "ZEROS" {
                continue;
            }
            if is_indicator_name(&tok) {
                acc.insert(tok);
            }
            continue;
        }

        if tok.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        if tok.eq_ignore_ascii_case("true") || tok.eq_ignore_ascii_case("false") {
            continue;
        }
        if matches!(
            tok.to_ascii_uppercase().as_str(),
            "EQ" | "NE" | "GT" | "LT" | "GE" | "LE" | "AND" | "OR" | "NOT"
        ) {
            continue;
        }
        acc.insert(tok);
    }
}

fn hint_numeric_vars(expr: &str, assigned: &mut BTreeMap<String, ValueType>) {
    let mut vars = BTreeSet::new();
    collect_vars(expr, &mut vars);
    for v in vars {
        assigned.entry(v).or_insert(ValueType::Number);
    }
}

fn hint_condition_types(cond: &str, assigned: &mut BTreeMap<String, ValueType>) {
    let mut vars = BTreeSet::new();
    collect_vars(cond, &mut vars);
    for v in vars {
        if is_indicator_name(&v) {
            assigned.entry(v).or_insert(ValueType::Bool);
        }
    }

    if let Some((left, op, right)) = split_condition(cond) {
        let left = left.trim();
        let right = right.trim();
        let left_ident = extract_single_identifier(left);
        let right_ident = extract_single_identifier(right);
        let left_lit = infer_literal_type(left);
        let right_lit = infer_literal_type(right);

        if is_numeric_comparator(op) {
            if let Some(id) = left_ident {
                assigned.entry(id.to_string()).or_insert(ValueType::Number);
            }
            if let Some(id) = right_ident {
                assigned.entry(id.to_string()).or_insert(ValueType::Number);
            }
            return;
        }

        if let (Some(id), Some(ty)) = (left_ident, right_lit) {
            assigned.entry(id.to_string()).or_insert(ty);
        }
        if let (Some(id), Some(ty)) = (right_ident, left_lit) {
            assigned.entry(id.to_string()).or_insert(ty);
        }
    }
}

fn split_condition(cond: &str) -> Option<(&str, &str, &str)> {
    let ops = [
        (" *GE ", "*GE"),
        (" *LE ", "*LE"),
        (" *GT ", "*GT"),
        (" *LT ", "*LT"),
        (" *EQ ", "*EQ"),
        (" *NE ", "*NE"),
        (">=", ">="),
        ("<=", "<="),
        ("==", "=="),
        ("!=", "!="),
        (">", ">"),
        ("<", "<"),
    ];
    for (needle, op) in ops {
        if let Some(idx) = cond.find(needle) {
            let left = &cond[..idx];
            let right = &cond[idx + needle.len()..];
            return Some((left, op, right));
        }
    }
    None
}

fn is_numeric_comparator(op: &str) -> bool {
    matches!(op, ">" | "<" | ">=" | "<=" | "*GT" | "*LT" | "*GE" | "*LE")
}

fn infer_literal_type(expr: &str) -> Option<ValueType> {
    let e = expr.trim();
    if e.is_empty() {
        return None;
    }
    if (e.starts_with('\'') && e.ends_with('\'') && e.len() >= 2)
        || (e.starts_with('"') && e.ends_with('"') && e.len() >= 2)
    {
        return Some(ValueType::Text);
    }
    if e.eq_ignore_ascii_case("true")
        || e.eq_ignore_ascii_case("false")
        || e.eq_ignore_ascii_case("*ON")
        || e.eq_ignore_ascii_case("*OFF")
    {
        return Some(ValueType::Bool);
    }
    if e.parse::<i64>().is_ok() || e.parse::<f64>().is_ok() {
        return Some(ValueType::Number);
    }
    None
}

fn extract_single_identifier(expr: &str) -> Option<&str> {
    let e = expr.trim();
    if e.is_empty() {
        return None;
    }
    let mut s = e;
    if let Some(rest) = s.strip_prefix('*') {
        s = rest;
    }
    if s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Some(s);
    }
    None
}

fn is_indicator_name(name: &str) -> bool {
    let up = name.to_ascii_uppercase();
    if !up.starts_with("IN") {
        return false;
    }
    let digits = &up[2..];
    !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit())
}

fn contains_logical_expr(upper: &str) -> bool {
    upper.contains(" AND ")
        || upper.contains(" OR ")
        || upper.starts_with("NOT ")
        || upper.starts_with("*NOT")
}

fn contains_comparator_expr(upper: &str, raw: &str) -> bool {
    upper.contains(" *EQ ")
        || upper.contains(" *NE ")
        || upper.contains(" *GT ")
        || upper.contains(" *LT ")
        || upper.contains(" *GE ")
        || upper.contains(" *LE ")
        || raw.contains("==")
        || raw.contains("!=")
        || raw.contains(">=")
        || raw.contains("<=")
        || raw.contains('>')
        || raw.contains('<')
}
