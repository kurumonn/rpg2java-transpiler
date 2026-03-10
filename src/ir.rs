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
    let mut referenced: BTreeSet<String> = BTreeSet::new();

    for stmt in &program.statements {
        match stmt {
            Stmt::Assign { left, right } => {
                let inferred = infer_expr_type(right);
                assigned.insert(left.clone(), inferred);
                collect_vars(right, &mut referenced);
                if inferred == ValueType::Number {
                    hint_numeric_vars(right, &mut assigned);
                }
                ir_statements.push(IrStmt::Assign {
                    left: left.clone(),
                    right: right.clone(),
                });
            }
            Stmt::If { cond } => {
                collect_vars(cond, &mut referenced);
                if is_numeric_condition(cond) {
                    hint_numeric_vars(cond, &mut assigned);
                }
                ir_statements.push(IrStmt::If { cond: cond.clone() });
            }
            Stmt::Else => ir_statements.push(IrStmt::Else),
            Stmt::EndIf => ir_statements.push(IrStmt::EndIf),
            Stmt::DoUntil { cond } => {
                collect_vars(cond, &mut referenced);
                if is_numeric_condition(cond) {
                    hint_numeric_vars(cond, &mut assigned);
                }
                ir_statements.push(IrStmt::DoUntil { cond: cond.clone() });
            }
            Stmt::EndDo => ir_statements.push(IrStmt::EndDo),
            Stmt::Call { proc_name } => ir_statements.push(IrStmt::Call {
                proc_name: proc_name.clone(),
            }),
            Stmt::Write { target } => {
                referenced.insert(target.clone());
                ir_statements.push(IrStmt::Write {
                    target: target.clone(),
                });
            }
            Stmt::Read { target } => {
                referenced.insert(target.clone());
                ir_statements.push(IrStmt::Read {
                    target: target.clone(),
                });
            }
            Stmt::Operation { op, args } => ir_statements.push(IrStmt::Todo {
                op: Some(op.clone()),
                src: args.clone(),
            }),
            Stmt::Raw(src) => ir_statements.push(IrStmt::Todo {
                op: None,
                src: src.clone(),
            }),
        }
    }

    let mut symbols = Vec::new();
    let mut all_names: BTreeSet<String> = BTreeSet::new();
    all_names.extend(assigned.keys().cloned());
    all_names.extend(referenced);

    for name in all_names {
        let ty = assigned.get(&name).copied().unwrap_or(ValueType::Unknown);
        symbols.push(SymbolInfo { name, ty });
    }

    IrProgram {
        statements: ir_statements,
        symbols,
    }
}

fn infer_expr_type(expr: &str) -> ValueType {
    let e = expr.trim();
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
    for tok in expr.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_')) {
        if tok.is_empty() {
            continue;
        }
        let is_number = tok.chars().all(|c| c.is_ascii_digit());
        if is_number {
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
        acc.insert(tok.to_string());
    }
}

fn hint_numeric_vars(expr: &str, assigned: &mut BTreeMap<String, ValueType>) {
    let mut vars = BTreeSet::new();
    collect_vars(expr, &mut vars);
    for v in vars {
        assigned.entry(v).or_insert(ValueType::Number);
    }
}

fn is_numeric_condition(cond: &str) -> bool {
    let upper = cond.to_ascii_uppercase();
    upper.contains(" *EQ ")
        || upper.contains(" *NE ")
        || upper.contains(" *GT ")
        || upper.contains(" *LT ")
        || upper.contains(" *GE ")
        || upper.contains(" *LE ")
        || cond.contains("==")
        || cond.contains("!=")
        || cond.contains('>')
        || cond.contains('<')
}
