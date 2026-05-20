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
    pub statement_lines: Vec<usize>,
    pub diagnostics: Vec<Diagnostic>,
    pub op_stats: Vec<OperationStat>,
    pub subroutine_routes: Vec<SubroutineRoute>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Warning,
}

impl DiagnosticSeverity {
    pub fn as_str(self) -> &'static str {
        match self {
            DiagnosticSeverity::Warning => "warning",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub severity: DiagnosticSeverity,
    pub code: String,
    pub message: String,
    pub line: Option<usize>,
    pub column: Option<usize>,
}

impl Diagnostic {
    pub fn as_text(&self) -> String {
        let mut out = format!("[{}][{}]", self.severity.as_str(), self.code);
        if let Some(line) = self.line {
            out.push_str(&format!(" line {}", line));
            if let Some(col) = self.column {
                out.push_str(&format!(" col {}", col));
            }
            out.push(':');
        }
        out.push(' ');
        out.push_str(&self.message);
        out
    }
}

#[derive(Debug, Clone)]
pub struct OperationStat {
    pub name: String,
    pub count: usize,
    pub level: SupportLevel,
}

#[derive(Debug, Clone)]
pub struct SubroutineRoute {
    pub caller: String,
    pub callee: String,
    pub line: usize,
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
    let mut statement_lines = Vec::new();
    let mut diagnostics = Vec::new();
    let mut op_counts: HashMap<String, usize> = HashMap::new();
    let mut subroutine_routes = Vec::new();

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
            if let Some(route) = extract_subroutine_route(&stmt, line_no + 1) {
                subroutine_routes.push(route);
            }
            statements.push(stmt);
            statement_lines.push(line_no + 1);
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
        statement_lines,
        diagnostics,
        op_stats,
        subroutine_routes,
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
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<(Stmt, Option<String>)> {
    if raw_line.trim().is_empty() {
        return None;
    }

    let spec = detect_spec_char(raw_line).to_ascii_uppercase();
    if spec == '*' {
        return None;
    }
    if spec != 'C' {
        push_diagnostic(
            diagnostics,
            DiagnosticSeverity::Warning,
            "PARSER_FIXED_UNSUPPORTED_SPEC",
            Some(line_no),
            None,
            format!("unsupported fixed spec '{}'", spec),
        );
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
        if raw_line.chars().count() < 26 {
            push_diagnostic(
                diagnostics,
                DiagnosticSeverity::Warning,
                "PARSER_FIXED_SHORT_LINE",
                Some(line_no),
                Some(26),
                "fixed C-spec line is too short for opcode field (expected col 26-35)",
            );
        }
        if let Some((detected_opcode, col)) = find_known_opcode_with_col(raw_line) {
            push_diagnostic(
                diagnostics,
                DiagnosticSeverity::Warning,
                "PARSER_FIXED_OPCODE_MISALIGNED",
                Some(line_no),
                Some(col),
                format!(
                    "opcode '{}' is outside fixed opcode field (expected col 26-35)",
                    detected_opcode
                ),
            );
        } else if raw_line.trim().len() > 7 {
            push_diagnostic(
                diagnostics,
                DiagnosticSeverity::Warning,
                "PARSER_FIXED_OPCODE_MISSING",
                Some(line_no),
                Some(26),
                "opcode not found in fixed opcode field",
            );
        }
        return Some((Stmt::Raw(raw_line.trim().to_string()), None));
    }
    if !is_known_opcode(&opcode)
        && let Some((detected_opcode, col)) = find_known_opcode_with_col(raw_line)
        && !(26..=35).contains(&col)
    {
        push_diagnostic(
            diagnostics,
            DiagnosticSeverity::Warning,
            "PARSER_FIXED_OPCODE_MISALIGNED",
            Some(line_no),
            Some(col),
            format!(
                "opcode '{}' is outside fixed opcode field (expected col 26-35)",
                detected_opcode
            ),
        );
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
                cond: normalize_fixed_condition(cond),
            }
        }
        "DOU" => {
            let cond = if !factor2.is_empty() {
                factor2
            } else {
                factor1
            };
            Stmt::DoUntil {
                cond: normalize_fixed_condition(cond),
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
        "CHAIN" | "SETLL" | "READE" | "EXSR" => Stmt::Operation {
            op: opcode.clone(),
            args: format_stub_args(&opcode, factor1, factor2, result),
        },
        _ => Stmt::Operation {
            op: opcode.clone(),
            args: collect_args(factor1, factor2, result),
        },
    };

    Some((stmt, Some(opcode)))
}

fn find_known_opcode_with_col(line: &str) -> Option<(String, usize)> {
    let upper = line.to_ascii_uppercase();
    let chars: Vec<char> = upper.chars().collect();
    let mut i = 0usize;
    while i < chars.len() {
        if chars[i].is_ascii_whitespace() {
            i += 1;
            continue;
        }
        let start = i;
        while i < chars.len() && !chars[i].is_ascii_whitespace() {
            i += 1;
        }
        let token: String = chars[start..i].iter().collect();
        if is_known_opcode(&token) {
            return Some((token, start + 1));
        }
    }
    None
}

fn is_known_opcode(token: &str) -> bool {
    matches!(
        token,
        "EVAL"
            | "MOVEL"
            | "IF"
            | "ELSE"
            | "ENDIF"
            | "DOU"
            | "ENDDO"
            | "CALLP"
            | "READ"
            | "WRITE"
            | "CHAIN"
            | "SETLL"
            | "READE"
            | "EXSR"
    )
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
    if upper.starts_with("CHAIN ") {
        let rest = line[6..].trim();
        return (
            Stmt::Operation {
                op: String::from("CHAIN"),
                args: format_stub_args_from_free("CHAIN", rest),
            },
            Some(String::from("CHAIN")),
        );
    }
    if upper.starts_with("SETLL ") {
        let rest = line[6..].trim();
        return (
            Stmt::Operation {
                op: String::from("SETLL"),
                args: format_stub_args_from_free("SETLL", rest),
            },
            Some(String::from("SETLL")),
        );
    }
    if upper.starts_with("READE ") {
        let rest = line[6..].trim();
        return (
            Stmt::Operation {
                op: String::from("READE"),
                args: format_stub_args_from_free("READE", rest),
            },
            Some(String::from("READE")),
        );
    }
    if upper.starts_with("EXSR ") {
        let rest = line[5..].trim();
        return (
            Stmt::Operation {
                op: String::from("EXSR"),
                args: format_stub_args_from_free("EXSR", rest),
            },
            Some(String::from("EXSR")),
        );
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

fn format_stub_args(op: &str, factor1: &str, factor2: &str, result: &str) -> String {
    match op {
        "CHAIN" | "SETLL" => {
            let key = if !factor1.is_empty() {
                factor1
            } else {
                factor2
            };
            let file = if !factor1.is_empty() {
                if !factor2.is_empty() { factor2 } else { result }
            } else if !result.is_empty() {
                result
            } else {
                factor2
            };
            join_named_args(&[("key", key), ("file", file)])
        }
        "READE" => {
            let key = factor1;
            let file = if !factor2.is_empty() { factor2 } else { result };
            let named = join_named_args(&[("key", key), ("file", file)]);
            if named.is_empty() {
                collect_args(factor1, factor2, result)
            } else {
                named
            }
        }
        "EXSR" => {
            let sub = if !factor2.is_empty() {
                factor2
            } else if !result.is_empty() {
                result
            } else {
                factor1
            };
            let named = join_named_args(&[("subroutine", sub)]);
            if named.is_empty() {
                collect_args(factor1, factor2, result)
            } else {
                named
            }
        }
        _ => collect_args(factor1, factor2, result),
    }
}

fn format_stub_args_from_free(op: &str, rest: &str) -> String {
    let mut parts = rest.split_whitespace();
    match op {
        "CHAIN" | "SETLL" => {
            let key = parts.next().unwrap_or("");
            let file = parts.next().unwrap_or("");
            let named = join_named_args(&[("key", key), ("file", file)]);
            if named.is_empty() {
                rest.trim().to_string()
            } else {
                named
            }
        }
        "READE" => {
            let first = parts.next().unwrap_or("");
            let second = parts.next().unwrap_or("");
            let named = if second.is_empty() {
                join_named_args(&[("file", first)])
            } else {
                join_named_args(&[("key", first), ("file", second)])
            };
            if named.is_empty() {
                rest.trim().to_string()
            } else {
                named
            }
        }
        "EXSR" => {
            let target = parts.next().unwrap_or("");
            let named = join_named_args(&[("subroutine", target)]);
            if named.is_empty() {
                rest.trim().to_string()
            } else {
                named
            }
        }
        _ => rest.trim().to_string(),
    }
}

fn join_named_args(pairs: &[(&str, &str)]) -> String {
    let mut out = Vec::new();
    for (k, v) in pairs {
        let value = v.trim();
        if value.is_empty() {
            continue;
        }
        out.push(format!("{k}={value}"));
    }
    out.join(", ")
}

fn push_diagnostic(
    diagnostics: &mut Vec<Diagnostic>,
    severity: DiagnosticSeverity,
    code: &str,
    line: Option<usize>,
    column: Option<usize>,
    message: impl Into<String>,
) {
    diagnostics.push(Diagnostic {
        severity,
        code: code.to_string(),
        message: message.into(),
        line,
        column,
    });
}

fn extract_subroutine_route(stmt: &Stmt, line_no: usize) -> Option<SubroutineRoute> {
    match stmt {
        Stmt::Operation { op, args } if op.eq_ignore_ascii_case("EXSR") => {
            let callee = extract_exsr_callee(args)?;
            Some(SubroutineRoute {
                caller: String::from("MAIN"),
                callee,
                line: line_no,
            })
        }
        _ => None,
    }
}

fn extract_exsr_callee(args: &str) -> Option<String> {
    let trimmed = args.trim();
    if trimmed.is_empty() {
        return None;
    }
    for piece in trimmed.split(',') {
        let p = piece.trim();
        if let Some(value) = p.strip_prefix("subroutine=") {
            let callee = value.trim();
            if !callee.is_empty() {
                return Some(callee.to_string());
            }
        }
    }
    let first = trimmed.split_whitespace().next()?;
    Some(first.to_string())
}

fn split_assign(expr: &str) -> Option<(&str, &str)> {
    let (left, right) = expr.split_once('=')?;
    Some((left.trim(), right.trim()))
}

fn normalize_fixed_condition(cond: &str) -> String {
    let trimmed = cond.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let upper = trimmed.to_ascii_uppercase();
    if let Some(ind) = parse_positive_indicator_token(&upper) {
        return format!("*IN{ind}");
    }
    if let Some(ind) = parse_negative_indicator_token(&upper) {
        return format!("NOT *IN{ind}");
    }
    trimmed.to_string()
}

fn parse_positive_indicator_token(token: &str) -> Option<String> {
    if token.len() == 2 && token.chars().all(|c| c.is_ascii_digit()) {
        return Some(token.to_string());
    }
    if token.len() == 4 && token.starts_with("IN") && token[2..].chars().all(|c| c.is_ascii_digit())
    {
        return Some(token[2..].to_string());
    }
    if token.len() == 5
        && token.starts_with("*IN")
        && token[3..].chars().all(|c| c.is_ascii_digit())
    {
        return Some(token[3..].to_string());
    }
    None
}

fn parse_negative_indicator_token(token: &str) -> Option<String> {
    if token.len() == 3 && token.starts_with('N') && token[1..].chars().all(|c| c.is_ascii_digit())
    {
        return Some(token[1..].to_string());
    }
    let rest = token.strip_prefix("NOT ")?;
    parse_positive_indicator_token(rest)
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
