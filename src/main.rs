mod ir;
mod parser;
mod report;
mod transpiler;

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Instant;

use ir::{IrProgram, ValueType};
use parser::{ParseMode, Program, SupportLevel};

struct Cli {
    input: Option<PathBuf>,
    batch_dir: Option<PathBuf>,
    output: Option<PathBuf>,
    output_dir: Option<PathBuf>,
    snapshot_dir: Option<PathBuf>,
    update_snapshots: bool,
    report_json: Option<PathBuf>,
    report_md: Option<PathBuf>,
    metrics_csv: Option<PathBuf>,
    class_name: String,
    mode: ParseMode,
    jobs: usize,
}

#[derive(Clone)]
struct BatchConfig {
    output_dir: PathBuf,
    snapshot_dir: Option<PathBuf>,
    update_snapshots: bool,
    mode: ParseMode,
}

struct BatchFileReport {
    class_name: String,
    input_bytes: u64,
    statements: usize,
    symbols: usize,
    todos: usize,
}

struct BatchJobResult {
    input: PathBuf,
    elapsed_ms: u128,
    result: Result<BatchFileReport, String>,
}

struct BatchMetricsRow {
    input: PathBuf,
    class_name: String,
    status: &'static str,
    input_bytes: u64,
    elapsed_ms: u128,
    statements: usize,
    symbols: usize,
    todos: usize,
    todo_rate: f64,
    error: Option<String>,
}

fn run() -> Result<(), String> {
    let args = parse_args()?;
    if let Some(batch_dir) = &args.batch_dir {
        return run_batch(&args, batch_dir);
    }

    let input = args
        .input
        .as_ref()
        .ok_or_else(|| String::from("--input is required unless --batch-dir is used"))?;
    let source = fs::read_to_string(input)
        .map_err(|e| format!("failed to read input file {}: {e}", input.display()))?;

    let program: Program = parser::parse_program_with_mode(&source, args.mode);
    let ir_program: IrProgram = ir::build_ir(&program);
    let java = transpiler::to_java(&ir_program, &args.class_name);

    if let Some(out) = &args.output {
        fs::write(out, &java)
            .map_err(|e| format!("failed to write output file {}: {e}", out.display()))?;
    } else {
        println!("{java}");
    }

    if let Some(snapshot_dir) = &args.snapshot_dir {
        let name = format!("{}.java", args.class_name);
        verify_or_update_snapshot(snapshot_dir, &name, &java, args.update_snapshots)?;
    }

    report::write_reports(
        &program,
        &ir_program,
        args.report_json.as_deref(),
        args.report_md.as_deref(),
    )?;

    print_summary(&program, &ir_program);
    Ok(())
}

fn run_batch(args: &Cli, batch_dir: &Path) -> Result<(), String> {
    let output_dir = args
        .output_dir
        .clone()
        .unwrap_or_else(|| PathBuf::from("./out/batch"));
    fs::create_dir_all(&output_dir).map_err(|e| {
        format!(
            "failed to create output directory {}: {e}",
            output_dir.display()
        )
    })?;

    let mut entries: Vec<PathBuf> = Vec::new();
    for entry in fs::read_dir(batch_dir)
        .map_err(|e| format!("failed to read batch directory {}: {e}", batch_dir.display()))?
    {
        let entry = entry.map_err(|e| format!("failed to read directory entry: {e}"))?;
        let path = entry.path();
        if path.is_file() {
            entries.push(path);
        }
    }
    entries.sort();

    let mut targets = Vec::new();
    for input in entries {
        let ext = input
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        if ext == "rpg" || ext == "txt" {
            targets.push(input);
        }
    }
    if targets.is_empty() {
        eprintln!("batch complete: no input files (.rpg/.txt) found in {}", batch_dir.display());
        return Ok(());
    }

    let config = BatchConfig {
        output_dir: output_dir.clone(),
        snapshot_dir: args.snapshot_dir.clone(),
        update_snapshots: args.update_snapshots,
        mode: args.mode,
    };

    let results: Vec<BatchJobResult> = if args.jobs <= 1 {
        run_batch_sequential(&targets, &config)
    } else {
        run_batch_parallel(&targets, &config, args.jobs)
    };

    let mut ok = 0usize;
    let mut ng = 0usize;
    let mut metrics = Vec::new();
    for r in results {
        match r.result {
            Ok(info) => {
                ok += 1;
                let todo_rate = if info.statements == 0 {
                    0.0
                } else {
                    info.todos as f64 / info.statements as f64
                };
                metrics.push(BatchMetricsRow {
                    input: r.input.clone(),
                    class_name: info.class_name.clone(),
                    status: "ok",
                    input_bytes: info.input_bytes,
                    elapsed_ms: r.elapsed_ms,
                    statements: info.statements,
                    symbols: info.symbols,
                    todos: info.todos,
                    todo_rate,
                    error: None,
                });
                eprintln!(
                    "ok: {} -> {} (ms={}, statements={}, symbols={}, todos={})",
                    r.input.display(),
                    info.class_name,
                    r.elapsed_ms,
                    info.statements,
                    info.symbols,
                    info.todos
                );
            }
            Err(err) => {
                ng += 1;
                metrics.push(BatchMetricsRow {
                    input: r.input.clone(),
                    class_name: class_name_from_path(&r.input),
                    status: "ng",
                    input_bytes: 0,
                    elapsed_ms: r.elapsed_ms,
                    statements: 0,
                    symbols: 0,
                    todos: 0,
                    todo_rate: 0.0,
                    error: Some(err.clone()),
                });
                eprintln!("ng: {err}");
            }
        }
    }

    if let Some(metrics_path) = &args.metrics_csv {
        write_metrics_csv(metrics_path, &metrics, ok, ng)?;
        eprintln!("metrics: {}", metrics_path.display());
    }

    eprintln!(
        "batch complete: success={} failed={} output={} jobs={}",
        ok,
        ng,
        output_dir.display()
        ,
        args.jobs
    );
    if ng > 0 {
        return Err(format!("batch finished with {ng} failure(s)"));
    }
    Ok(())
}

fn run_batch_sequential(
    targets: &[PathBuf],
    config: &BatchConfig,
) -> Vec<BatchJobResult> {
    let mut out = Vec::new();
    for input in targets {
        let input = input.clone();
        let start = Instant::now();
        let result = process_batch_file(input.clone(), config.clone());
        out.push(BatchJobResult {
            input,
            elapsed_ms: start.elapsed().as_millis(),
            result,
        });
    }
    out
}

fn run_batch_parallel(
    targets: &[PathBuf],
    config: &BatchConfig,
    jobs: usize,
) -> Vec<BatchJobResult> {
    let worker_count = jobs.max(1).min(targets.len().max(1));
    let mut buckets: Vec<Vec<PathBuf>> = vec![Vec::new(); worker_count];
    for (i, path) in targets.iter().enumerate() {
        buckets[i % worker_count].push(path.clone());
    }

    let mut handles = Vec::new();
    for bucket in buckets {
        let cfg = config.clone();
        handles.push(thread::spawn(move || {
            let mut local = Vec::new();
            for input in bucket {
                let start = Instant::now();
                let result = process_batch_file(input.clone(), cfg.clone());
                local.push(BatchJobResult {
                    input,
                    elapsed_ms: start.elapsed().as_millis(),
                    result,
                });
            }
            local
        }));
    }

    let mut out = Vec::new();
    for h in handles {
        match h.join() {
            Ok(mut list) => out.append(&mut list),
            Err(_) => out.push(BatchJobResult {
                input: PathBuf::from("<worker>"),
                elapsed_ms: 0,
                result: Err(String::from("worker thread panicked")),
            }),
        }
    }
    out
}

fn process_batch_file(input: PathBuf, config: BatchConfig) -> Result<BatchFileReport, String> {
    let metadata = fs::metadata(&input)
        .map_err(|e| format!("stat failed {}: {e}", input.display()))?;
    let source = fs::read_to_string(&input)
        .map_err(|e| format!("read failed {}: {e}", input.display()))?;
    let class_name = class_name_from_path(&input);

    let program: Program = parser::parse_program_with_mode(&source, config.mode);
    let ir_program: IrProgram = ir::build_ir(&program);
    let java = transpiler::to_java(&ir_program, &class_name);

    let java_name = format!("{class_name}.java");
    let java_out = config.output_dir.join(&java_name);
    fs::write(&java_out, &java)
        .map_err(|e| format!("write failed {}: {e}", java_out.display()))?;

    if let Some(snapshot_dir) = &config.snapshot_dir {
        verify_or_update_snapshot(snapshot_dir, &java_name, &java, config.update_snapshots)?;
    }

    let report_json = config.output_dir.join(format!("{class_name}.report.json"));
    let report_md = config.output_dir.join(format!("{class_name}.report.md"));
    report::write_reports(&program, &ir_program, Some(&report_json), Some(&report_md))?;

    let todos = ir_program
        .statements
        .iter()
        .filter(|s| matches!(s, ir::IrStmt::Todo { .. }))
        .count();

    Ok(BatchFileReport {
        class_name,
        input_bytes: metadata.len(),
        statements: program.statements.len(),
        symbols: ir_program.symbols.len(),
        todos,
    })
}

fn write_metrics_csv(
    path: &Path,
    rows: &[BatchMetricsRow],
    success: usize,
    failed: usize,
) -> Result<(), String> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)
        .map_err(|e| format!("failed to create metrics directory {}: {e}", parent.display()))?;

    let mut out = String::new();
    out.push_str(
        "status,input_path,class_name,input_bytes,elapsed_ms,ms_per_kib,size_band,statements,symbols,todos,todo_rate,error\n",
    );
    let ok_elapsed: Vec<u128> = rows
        .iter()
        .filter(|r| r.status == "ok")
        .map(|r| r.elapsed_ms)
        .collect();
    let p50 = percentile(&ok_elapsed, 50);
    let p95 = percentile(&ok_elapsed, 95);
    let p99 = percentile(&ok_elapsed, 99);
    for row in rows {
        let kib = (row.input_bytes as f64 / 1024.0).max(0.001);
        let ms_per_kib = row.elapsed_ms as f64 / kib;
        let size_band = size_band(row.input_bytes);
        out.push_str(&format!(
            "{},{},{},{},{},{:.4},{},{},{},{},{:.4},{}\n",
            row.status,
            csv_escape(&row.input.display().to_string()),
            csv_escape(&row.class_name),
            row.input_bytes,
            row.elapsed_ms,
            ms_per_kib,
            size_band,
            row.statements,
            row.symbols,
            row.todos,
            row.todo_rate,
            csv_escape(row.error.as_deref().unwrap_or(""))
        ));
    }
    out.push_str(&format!(
        "SUMMARY,,,,,,,{},{},,,\n",
        success, failed
    ));
    out.push_str(&format!(
        "PERCENTILE,,,{},,,,,,,,\n",
        format!("p50_ms={};p95_ms={};p99_ms={}", p50, p95, p99)
    ));
    append_size_band_summary(&mut out, rows);
    fs::write(path, out)
        .map_err(|e| format!("failed to write metrics CSV {}: {e}", path.display()))
}

fn percentile(values: &[u128], p: usize) -> u128 {
    if values.is_empty() {
        return 0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    let rank = ((p as f64 / 100.0) * (sorted.len().saturating_sub(1) as f64)).round() as usize;
    sorted[rank.min(sorted.len() - 1)]
}

fn size_band(bytes: u64) -> &'static str {
    match bytes {
        0..=1023 => "<1KiB",
        1024..=10_239 => "1-10KiB",
        10_240..=102_399 => "10-100KiB",
        102_400..=1_048_575 => "100KiB-1MiB",
        _ => ">=1MiB",
    }
}

fn append_size_band_summary(out: &mut String, rows: &[BatchMetricsRow]) {
    let bands = ["<1KiB", "1-10KiB", "10-100KiB", "100KiB-1MiB", ">=1MiB"];
    for band in bands {
        let band_rows: Vec<&BatchMetricsRow> = rows
            .iter()
            .filter(|r| r.status == "ok" && size_band(r.input_bytes) == band)
            .collect();
        if band_rows.is_empty() {
            continue;
        }

        let mut elapsed: Vec<u128> = band_rows.iter().map(|r| r.elapsed_ms).collect();
        elapsed.sort_unstable();
        let avg_ms = elapsed.iter().map(|v| *v as f64).sum::<f64>() / elapsed.len() as f64;
        let med_ms = percentile(&elapsed, 50);
        let avg_todo = band_rows.iter().map(|r| r.todo_rate).sum::<f64>() / band_rows.len() as f64;

        out.push_str(&format!(
            "SUMMARY_BAND,{},{},,avg_ms={:.3};median_ms={};avg_todo_rate={:.4},,,,,,,\n",
            band_rows.len(),
            band,
            avg_ms,
            med_ms,
            avg_todo
        ));
    }
}

fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

fn verify_or_update_snapshot(
    snapshot_dir: &Path,
    file_name: &str,
    content: &str,
    update: bool,
) -> Result<(), String> {
    fs::create_dir_all(snapshot_dir).map_err(|e| {
        format!(
            "failed to create snapshot directory {}: {e}",
            snapshot_dir.display()
        )
    })?;
    let snapshot = snapshot_dir.join(file_name);
    if update || !snapshot.exists() {
        fs::write(&snapshot, content)
            .map_err(|e| format!("failed to write snapshot {}: {e}", snapshot.display()))?;
        eprintln!("snapshot updated: {}", snapshot.display());
        return Ok(());
    }
    let current = fs::read_to_string(&snapshot)
        .map_err(|e| format!("failed to read snapshot {}: {e}", snapshot.display()))?;
    if current != content {
        return Err(format!(
            "snapshot mismatch: {} (run with --update-snapshots to refresh)",
            snapshot.display()
        ));
    }
    eprintln!("snapshot ok: {}", snapshot.display());
    Ok(())
}

fn class_name_from_path(path: &Path) -> String {
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("MainProgram");
    let mut out = String::new();
    let mut upper = true;
    for ch in stem.chars() {
        if ch.is_ascii_alphanumeric() {
            if upper {
                out.push(ch.to_ascii_uppercase());
                upper = false;
            } else {
                out.push(ch);
            }
        } else {
            upper = true;
        }
    }
    if out.is_empty() {
        String::from("MainProgram")
    } else {
        out
    }
}

fn parse_args() -> Result<Cli, String> {
    let mut input: Option<PathBuf> = None;
    let mut batch_dir: Option<PathBuf> = None;
    let mut output: Option<PathBuf> = None;
    let mut output_dir: Option<PathBuf> = None;
    let mut snapshot_dir: Option<PathBuf> = None;
    let mut update_snapshots = false;
    let mut report_json: Option<PathBuf> = None;
    let mut report_md: Option<PathBuf> = None;
    let mut metrics_csv: Option<PathBuf> = None;
    let mut class_name = String::from("MainProgram");
    let mut mode = ParseMode::Auto;
    let mut jobs: usize = 1;

    let args: Vec<String> = env::args().collect();
    let mut i = 1usize;
    while i < args.len() {
        match args[i].as_str() {
            "--input" | "-i" => {
                i += 1;
                let value = args.get(i).ok_or_else(|| String::from("missing value for --input"))?;
                input = Some(PathBuf::from(value));
            }
            "--batch-dir" => {
                i += 1;
                let value =
                    args.get(i).ok_or_else(|| String::from("missing value for --batch-dir"))?;
                batch_dir = Some(PathBuf::from(value));
            }
            "--output" | "-o" => {
                i += 1;
                let value = args
                    .get(i)
                    .ok_or_else(|| String::from("missing value for --output"))?;
                output = Some(PathBuf::from(value));
            }
            "--output-dir" => {
                i += 1;
                let value =
                    args.get(i).ok_or_else(|| String::from("missing value for --output-dir"))?;
                output_dir = Some(PathBuf::from(value));
            }
            "--snapshot-dir" => {
                i += 1;
                let value = args
                    .get(i)
                    .ok_or_else(|| String::from("missing value for --snapshot-dir"))?;
                snapshot_dir = Some(PathBuf::from(value));
            }
            "--update-snapshots" => {
                update_snapshots = true;
            }
            "--class-name" => {
                i += 1;
                let value = args
                    .get(i)
                    .ok_or_else(|| String::from("missing value for --class-name"))?;
                class_name = value.clone();
            }
            "--report-json" => {
                i += 1;
                let value = args
                    .get(i)
                    .ok_or_else(|| String::from("missing value for --report-json"))?;
                report_json = Some(PathBuf::from(value));
            }
            "--report-md" => {
                i += 1;
                let value = args
                    .get(i)
                    .ok_or_else(|| String::from("missing value for --report-md"))?;
                report_md = Some(PathBuf::from(value));
            }
            "--metrics-csv" => {
                i += 1;
                let value = args
                    .get(i)
                    .ok_or_else(|| String::from("missing value for --metrics-csv"))?;
                metrics_csv = Some(PathBuf::from(value));
            }
            "--mode" => {
                i += 1;
                let value =
                    args.get(i).ok_or_else(|| String::from("missing value for --mode"))?;
                mode = parse_mode(value)?;
            }
            "--jobs" => {
                i += 1;
                let value =
                    args.get(i).ok_or_else(|| String::from("missing value for --jobs"))?;
                jobs = value
                    .parse::<usize>()
                    .map_err(|_| format!("invalid --jobs '{}'", value))?;
                if jobs == 0 {
                    return Err(String::from("--jobs must be >= 1"));
                }
            }
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            unknown => return Err(format!("unknown argument: {unknown}")),
        }
        i += 1;
    }

    if input.is_some() && batch_dir.is_some() {
        return Err(String::from("--input and --batch-dir are mutually exclusive"));
    }
    if input.is_none() && batch_dir.is_none() {
        return Err(String::from("either --input or --batch-dir is required"));
    }
    if batch_dir.is_some() && output.is_some() {
        return Err(String::from("--output is not used in --batch-dir mode, use --output-dir"));
    }
    if batch_dir.is_some() && (report_json.is_some() || report_md.is_some()) {
        return Err(String::from(
            "--report-json/--report-md are per-file in batch mode and auto-generated to --output-dir",
        ));
    }
    if batch_dir.is_none() && metrics_csv.is_some() {
        return Err(String::from("--metrics-csv is available only in --batch-dir mode"));
    }

    Ok(Cli {
        input,
        batch_dir,
        output,
        output_dir,
        snapshot_dir,
        update_snapshots,
        report_json,
        report_md,
        metrics_csv,
        class_name,
        mode,
        jobs,
    })
}

fn print_help() {
    println!("rpg2java-transpiler");
    println!("Single-file mode:");
    println!("  rpg2java-transpiler --input <file> [--output <file>] [--class-name <name>] [--mode auto|free|fixed]");
    println!("                         [--report-json <file>] [--report-md <file>]");
    println!("                         [--snapshot-dir <dir>] [--update-snapshots]");
    println!("Batch mode:");
    println!("  rpg2java-transpiler --batch-dir <dir> [--output-dir <dir>] [--mode auto|free|fixed] [--jobs <n>]");
    println!("                         [--snapshot-dir <dir>] [--update-snapshots] [--metrics-csv <file>]");
}

fn parse_mode(value: &str) -> Result<ParseMode, String> {
    match value.to_ascii_lowercase().as_str() {
        "auto" => Ok(ParseMode::Auto),
        "free" => Ok(ParseMode::Free),
        "fixed" => Ok(ParseMode::Fixed),
        _ => Err(format!(
            "invalid mode '{}', expected auto|free|fixed",
            value
        )),
    }
}

fn print_summary(program: &Program, ir_program: &IrProgram) {
    if !program.diagnostics.is_empty() {
        eprintln!("diagnostics:");
        for d in &program.diagnostics {
            eprintln!("  - {d}");
        }
    }
    if !program.op_stats.is_empty() {
        eprintln!("operation matrix:");
        for stat in &program.op_stats {
            let level = match stat.level {
                SupportLevel::Implemented => "implemented",
                SupportLevel::Stub => "stub",
                SupportLevel::Planned => "planned",
            };
            eprintln!("  - {:<10} count={} level={}", stat.name, stat.count, level);
        }
    }
    if !ir_program.symbols.is_empty() {
        eprintln!("symbols:");
        for sym in &ir_program.symbols {
            let ty = match sym.ty {
                ValueType::Number => "number",
                ValueType::Text => "text",
                ValueType::Bool => "bool",
                ValueType::Unknown => "unknown",
            };
            eprintln!("  - {:<20} {}", sym.name, ty);
        }
    }
}

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err}");
        print_help();
        std::process::exit(1);
    }
}
