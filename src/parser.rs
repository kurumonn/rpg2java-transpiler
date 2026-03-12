use std::collections::HashMap;

#[derive(Debug, Clone, Copy)]
pub enum ParseMode {
    Auto,
    Free,
    Fixed,
}

#[derive(Debug, Clone)]
pub struct Program {
    pub statements: Vec<Stmt>,
    pub diagnostics: Vec<String>,
    pub op_stats: Vec<OperationStat>,
}

#[derive(Debug, Clone)]
pub struct OperationStat {
    pub name: String,
    pub count: usize,
    pub level: SupportLevel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupportLevel {
    Implemented,
    Stub,
    Planned,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Assign { left: String, right: String },
    If { cond: String },
    Else,
    EndIf,
    DoUntil { cond: String },
    EndDo,
    Call { proc_name: String },
    Write { target: String },
    Read { target: String },
    Operation { op: String, args: String },
    Raw(String),
}

pub fn parse_program_with_mode(source: &str, mode: ParseMode) -> Program {
    let actual_mode = match mode {
        ParseMode::Auto => detect_mode(source),
        v => v,
    };

    let mut statements = Vec::new();
    let mut diagnostics = Vec::new();
    let mut op_counts: HashMap<String, usize> = HashMap::new();

    for (line_no, raw_line) in source.lines().enumerate() {
        let parsed = match actual_mode {
            ParseMode::Free => parse_free_line(raw_line),
            ParseMode::Fixed => parse_fixed_line(raw_line, line_no + 1, &mut diagnostics),
            ParseMode::Auto => unreachable!(),
        };

        if let Some((stmt, op)) = parsed {
            if let Some(op_name) = op {
                *op_counts.entry(op_name).or_insert(0) += 1;
            }
            statements.push(stmt);
        }
    }

    let mut op_stats = Vec::new();
    for (name, count) in op_counts {
        op_stats.push(OperationStat {
            level: lookup_support_level(&name),
            name,
            count,
        });
    }
    op_stats.sort_by(|a, b| a.name.cmp(&b.name));

    Program {
        statements,
        diagnostics,
        op_stats,
    }
}

fn detect_mode(source: &str) -> ParseMode {
    let mut fixed_like = 0usize;
    let mut total = 0usize;
    for line in source.lines() {
        let l = line.trim_end();
        if l.is_empty() {
            continue;
        }
        total += 1;
        let spec = detect_spec_char(l);
        if matches!(
            spec,
            'H' | 'F' | 'E' | 'I' | 'L' | 'C' | 'O' | 'h' | 'f' | 'e' | 'i' | 'l' | 'c' | 'o'
        ) {
            fixed_like += 1;
        }
    }
    if total > 0 && fixed_like * 100 / total >= 50 {
        ParseMode::Fixed
    } else {
        ParseMode::Free
    }
}

fn parse_free_line(raw_line: &str) -> Option<(Stmt, Option<String>)> {
    let line = raw_line.trim();
    if line.is_empty() || line.starts_with('*') {
        return None;
    }
    Some(parse_common_line(line))
}

fn parse_fixed_line(
    raw_line: &str,
    line_no: usize,
    diagnostics: &mut Vec<String>,
) -> Option<(Stmt, Option<String>)> {
    if raw_line.trim().is_empty() {
        return None;
    }

    let spec = detect_spec_char(raw_line).to_ascii_uppercase();
    if spec == '*' {
        return None;
    }
    if spec != 'C' {
        diagnostics.push(format!(
            "line {}: unsupported fixed spec '{}'",
            line_no, spec
        ));
        return Some((Stmt::Raw(raw_line.trim().to_string()), None));
    }

    let opcode_buf = slice_cols(raw_line, 26, 35);
    let factor1_buf = slice_cols(raw_line, 12, 25);
    let factor2_buf = slice_cols(raw_line, 36, 49);
    let result_buf = slice_cols(raw_line, 50, 63);
    let opcode = opcode_buf.trim().to_ascii_uppercase();
    let factor1 = factor1_buf.trim();
    let factor2 = factor2_buf.trim();
    let result = result_buf.trim();

    if opcode.is_empty() {
        return Some((Stmt::Raw(raw_line.trim().to_string()), None));
    }

    let stmt = match opcode.as_str() {
        "EVAL" => {
            if !result.is_empty() && !factor2.is_empty() {
                Stmt::Assign {
                    left: result.to_string(),
                    right: factor2.to_string(),
                }
            } else {
                Stmt::Operation {
                    op: opcode.clone(),
                    args: collect_args(factor1, factor2, result),
                }
            }
        }
        "MOVEL" => {
            let right = if !factor2.is_empty() {
                factor2
            } else {
                factor1
            };
            if !right.is_empty() && !result.is_empty() {
                Stmt::Assign {
                    left: result.to_string(),
                    right: right.to_string(),
                }
            } else {
                Stmt::Operation {
                    op: opcode.clone(),
                    args: collect_args(factor1, factor2, result),
                }
            }
        }
        "IF" => {
            let cond = if !factor2.is_empty() {
                factor2
            } else {
                factor1
            };
            Stmt::If {
                cond: cond.to_string(),
            }
        }
        "DOU" => {
            let cond = if !factor2.is_empty() {
                factor2
            } else {
                factor1
            };
            Stmt::DoUntil {
                cond: cond.to_string(),
            }
        }
        "ENDIF" => Stmt::EndIf,
        "ENDDO" => Stmt::EndDo,
        "ELSE" => Stmt::Else,
        "CALLP" => {
            let proc_name = if !factor2.is_empty() { factor2 } else { result };
            Stmt::Call {
                proc_name: proc_name.to_string(),
            }
        }
        "READ" => {
            let target = if !factor2.is_empty() { factor2 } else { result };
            Stmt::Read {
                target: target.to_string(),
            }
        }
        "WRITE" => {
            let target = if !factor2.is_empty() { factor2 } else { result };
            Stmt::Write {
                target: target.to_string(),
            }
        }
        _ => Stmt::Operation {
            op: opcode.clone(),
            args: collect_args(factor1, factor2, result),
        },
    };

    Some((stmt, Some(opcode)))
}

fn detect_spec_char(line: &str) -> char {
    let col6 = line.chars().nth(5).unwrap_or(' ');
    if col6.is_ascii_digit() {
        line.chars().nth(6).unwrap_or(' ')
    } else {
        col6
    }
}

fn parse_common_line(line: &str) -> (Stmt, Option<String>) {
    let upper = line.to_ascii_uppercase();
    if upper.starts_with("EVAL ")
        && let Some((l, r)) = split_assign(&line[5..])
    {
        return (
            Stmt::Assign {
                left: l.to_string(),
                right: r.to_string(),
            },
            Some(String::from("EVAL")),
        );
    }
    if upper.starts_with("IF ") {
        return (
            Stmt::If {
                cond: line[3..].trim().to_string(),
            },
            Some(String::from("IF")),
        );
    }
    if upper == "ELSE" {
        return (Stmt::Else, Some(String::from("ELSE")));
    }
    if upper == "ENDIF" {
        return (Stmt::EndIf, Some(String::from("ENDIF")));
    }
    if upper.starts_with("DOU ") {
        return (
            Stmt::DoUntil {
                cond: line[4..].trim().to_string(),
            },
            Some(String::from("DOU")),
        );
    }
    if upper == "ENDDO" {
        return (Stmt::EndDo, Some(String::from("ENDDO")));
    }
    if upper.starts_with("CALLP ") {
        return (
            Stmt::Call {
                proc_name: line[6..].trim().to_string(),
            },
            Some(String::from("CALLP")),
        );
    }
    if upper.starts_with("WRITE ") {
        return (
            Stmt::Write {
                target: line[6..].trim().to_string(),
            },
            Some(String::from("WRITE")),
        );
    }
    if upper.starts_with("READ ") {
        return (
            Stmt::Read {
                target: line[5..].trim().to_string(),
            },
            Some(String::from("READ")),
        );
    }
    if upper.starts_with("MOVEL ") {
        let rest = line[6..].trim();
        let mut parts = rest.split_whitespace();
        if let (Some(r), Some(l)) = (parts.next(), parts.next()) {
            return (
                Stmt::Assign {
                    left: l.to_string(),
                    right: r.to_string(),
                },
                Some(String::from("MOVEL")),
            );
        }
    }
    (Stmt::Raw(line.to_string()), None)
}

pub fn lookup_support_level(op: &str) -> SupportLevel {
    match op.to_ascii_uppercase().as_str() {
        "EVAL" | "MOVEL" | "IF" | "ELSE" | "ENDIF" | "CALLP" | "READ" | "WRITE" | "DOU"
        | "ENDDO" => SupportLevel::Implemented,
        "CHAIN" | "SETLL" | "READE" | "EXSR" => SupportLevel::Stub,
        _ => SupportLevel::Planned,
    }
}

fn collect_args(factor1: &str, factor2: &str, result: &str) -> String {
    let mut parts = Vec::new();
    if !factor1.is_empty() {
        parts.push(factor1);
    }
    if !factor2.is_empty() {
        parts.push(factor2);
    }
    if !result.is_empty() {
        parts.push(result);
    }
    parts.join(", ")
}

fn split_assign(expr: &str) -> Option<(&str, &str)> {
    let (left, right) = expr.split_once('=')?;
    Some((left.trim(), right.trim()))
}

fn slice_cols(line: &str, start_col: usize, end_col: usize) -> String {
    let chars: Vec<char> = line.chars().collect();
    let start = start_col.saturating_sub(1);
    let end = end_col.min(chars.len());
    if start >= end || start >= chars.len() {
        return String::new();
    }
    chars[start..end].iter().collect()
}
