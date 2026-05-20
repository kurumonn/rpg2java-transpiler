mod ir;
mod parser;
mod report;
mod transpiler;

use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::Instant;

use ir::{IrProgram, ValueType};
use parser::{ParseMode, Program, SupportLevel};
use transpiler::JavaTarget;

struct Cli {
    input: Option<PathBuf>,
    batch_dir: Option<PathBuf>,
    output: Option<PathBuf>,
    output_dir: Option<PathBuf>,
    snapshot_dir: Option<PathBuf>,
    update_snapshots: bool,
    report_json: Option<PathBuf>,
    report_md: Option<PathBuf>,
    report_html: Option<PathBuf>,
    source_map: Option<PathBuf>,
    html_report: bool,
    source_map_report: bool,
    strict: bool,
    fail_on_todo_rate: Option<f64>,
    dry_run: bool,
    metrics_csv: Option<PathBuf>,
    perf_report_json: Option<PathBuf>,
    javac_check: bool,
    javac_cmd: String,
    class_name: String,
    java_target: JavaTarget,
    mode: ParseMode,
    jobs: usize,
}

#[derive(Clone)]
struct BatchConfig {
    output_dir: PathBuf,
    snapshot_dir: Option<PathBuf>,
    update_snapshots: bool,
    report_html: bool,
    source_map: bool,
    dry_run: bool,
    mode: ParseMode,
    java_target: JavaTarget,
    javac_check: bool,
    javac_cmd: String,
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
    if args.dry_run {
        eprintln!("dry-run: validation completed; no files will be written");
    }
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
    let transpiler::JavaOutput {
        java,
        source_map: source_map_entries,
    } = transpiler::to_java_with_source_map(&ir_program, &args.class_name, args.java_target);

    let output_path = if let Some(out) = &args.output {
        if args.dry_run {
            Some(out.clone())
        } else {
            fs::write(out, &java)
                .map_err(|e| format!("failed to write output file {}: {e}", out.display()))?;
            Some(out.clone())
        }
    } else {
        println!("{java}");
        None
    };

    let mut javac_check_summary: Option<report::JavacCheckSummary> = None;
    if args.javac_check && !args.dry_run {
        let out = output_path
            .as_ref()
            .ok_or_else(|| String::from("--javac-check requires --output in single-file mode"))?;
        javac_check_summary = Some(run_javac_check(out, &args.javac_cmd));
    }

    if let Some(snapshot_dir) = &args.snapshot_dir {
        let name = format!("{}.java", args.class_name);
        verify_or_update_snapshot(snapshot_dir, &name, &java, args.update_snapshots)?;
    }

    if !args.dry_run {
        report::write_reports(
            &program,
            &ir_program,
            args.java_target.as_str(),
            javac_check_summary.as_ref(),
            &source_map_entries,
            report::ReportOutputs {
                json_path: args.report_json.as_deref(),
                md_path: args.report_md.as_deref(),
                html_path: args.report_html.as_deref(),
                source_map_path: args.source_map.as_deref(),
            },
        )?;
    }

    enforce_todo_policy(&ir_program, args.strict, args.fail_on_todo_rate)?;

    if let Some(check) = javac_check_summary.as_ref()
        && !check.success
    {
        return Err(format!(
            "javac check failed for output with command '{}': {}",
            check.command, check.detail
        ));
    }

    print_summary(&program, &ir_program, args.java_target);
    Ok(())
}

fn run_batch(args: &Cli, batch_dir: &Path) -> Result<(), String> {
    let output_dir = args
        .output_dir
        .clone()
        .unwrap_or_else(|| PathBuf::from("./out/batch"));
    if !args.dry_run {
        fs::create_dir_all(&output_dir).map_err(|e| {
            format!(
                "failed to create output directory {}: {e}",
                output_dir.display()
            )
        })?;
    }

    let mut targets: Vec<PathBuf> = Vec::new();
    for entry in fs::read_dir(batch_dir).map_err(|e| {
        format!(
            "failed to read batch directory {}: {e}",
            batch_dir.display()
        )
    })? {
        let entry = entry.map_err(|e| format!("failed to read directory entry: {e}"))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        if ext == "rpg" || ext == "txt" {
            targets.push(path);
        }
    }
    targets.sort();
    if targets.is_empty() {
        eprintln!(
            "batch complete: no input files (.rpg/.txt) found in {}",
            batch_dir.display()
        );
        return Ok(());
    }

    let config = BatchConfig {
        output_dir: output_dir.clone(),
        snapshot_dir: args.snapshot_dir.clone(),
        update_snapshots: args.update_snapshots,
        report_html: args.html_report,
        source_map: args.source_map_report,
        dry_run: args.dry_run,
        mode: args.mode,
        java_target: args.java_target,
        javac_check: args.javac_check,
        javac_cmd: args.javac_cmd.clone(),
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
        if !args.dry_run {
            write_metrics_csv(metrics_path, &metrics, ok, ng)?;
        }
        eprintln!("metrics: {}", metrics_path.display());
    }
    if let Some(report_path) = resolve_perf_report_path(args) {
        if !args.dry_run {
            write_perf_report_json(&report_path, &metrics, ok, ng, args.jobs)?;
        }
        eprintln!("perf report: {}", report_path.display());
    }
    enforce_batch_todo_policy(&metrics, args.strict, args.fail_on_todo_rate)?;

    eprintln!(
        "batch complete: success={} failed={} output={} jobs={}",
        ok,
        ng,
        output_dir.display(),
        args.jobs
    );
    if ng > 0 {
        return Err(format!("batch finished with {ng} failure(s)"));
    }
    Ok(())
}

fn run_batch_sequential(targets: &[PathBuf], config: &BatchConfig) -> Vec<BatchJobResult> {
    let mut out = Vec::with_capacity(targets.len());
    for input in targets {
        let start = Instant::now();
        let result = process_batch_file(input, config);
        out.push(BatchJobResult {
            input: input.clone(),
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
            let mut local = Vec::with_capacity(bucket.len());
            for input in bucket {
                let start = Instant::now();
                let result = process_batch_file(&input, &cfg);
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

fn process_batch_file(input: &Path, config: &BatchConfig) -> Result<BatchFileReport, String> {
    let metadata =
        fs::metadata(input).map_err(|e| format!("stat failed {}: {e}", input.display()))?;
    let source =
        fs::read_to_string(input).map_err(|e| format!("read failed {}: {e}", input.display()))?;
    let class_name = class_name_from_path(input);

    let program: Program = parser::parse_program_with_mode(&source, config.mode);
    let ir_program: IrProgram = ir::build_ir(&program);
    let transpiler::JavaOutput {
        java,
        source_map: source_map_entries,
    } = transpiler::to_java_with_source_map(&ir_program, &class_name, config.java_target);

    let java_name = format!("{class_name}.java");
    let java_out = config.output_dir.join(&java_name);
    if !config.dry_run {
        fs::write(&java_out, &java)
            .map_err(|e| format!("write failed {}: {e}", java_out.display()))?;
    }

    let mut javac_check_summary: Option<report::JavacCheckSummary> = None;
    if config.javac_check && !config.dry_run {
        javac_check_summary = Some(run_javac_check(&java_out, &config.javac_cmd));
    }

    if let Some(snapshot_dir) = &config.snapshot_dir {
        if config.dry_run {
            eprintln!("dry-run: skip snapshot write/check for {}", java_name);
        } else {
            verify_or_update_snapshot(snapshot_dir, &java_name, &java, config.update_snapshots)?;
        }
    }

    let report_json = config.output_dir.join(format!("{class_name}.report.json"));
    let report_md = config.output_dir.join(format!("{class_name}.report.md"));
    let report_html = config
        .report_html
        .then(|| config.output_dir.join(format!("{class_name}.report.html")));
    let source_map_path = config.source_map.then(|| {
        config
            .output_dir
            .join(format!("{class_name}.source-map.json"))
    });
    if !config.dry_run {
        report::write_reports(
            &program,
            &ir_program,
            config.java_target.as_str(),
            javac_check_summary.as_ref(),
            &source_map_entries,
            report::ReportOutputs {
                json_path: Some(&report_json),
                md_path: Some(&report_md),
                html_path: report_html.as_deref(),
                source_map_path: source_map_path.as_deref(),
            },
        )?;
    }

    if let Some(check) = javac_check_summary.as_ref()
        && !check.success
    {
        return Err(format!(
            "javac check failed for {} with command '{}': {}",
            java_out.display(),
            check.command,
            check.detail
        ));
    }

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
    fs::create_dir_all(parent).map_err(|e| {
        format!(
            "failed to create metrics directory {}: {e}",
            parent.display()
        )
    })?;

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
    out.push_str(&format!("SUMMARY,,,,,,,{},{},,,\n", success, failed));
    out.push_str(&format!(
        "PERCENTILE,,,p50_ms={};p95_ms={};p99_ms={};,,,,,,,\n",
        p50, p95, p99
    ));
    append_size_band_summary(&mut out, rows);
    fs::write(path, out).map_err(|e| format!("failed to write metrics CSV {}: {e}", path.display()))
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
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("MainProgram");
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
    let mut raw_args: Vec<String> = env::args().collect();
    if raw_args.get(1).map(|s| s.as_str()) == Some("doctor") {
        print_doctor();
        std::process::exit(0);
    }
    if matches!(
        raw_args.get(1).map(|s| s.as_str()),
        Some("convert" | "batch" | "report" | "verify" | "coverage")
    ) {
        raw_args.remove(1);
    }

    let mut input: Option<PathBuf> = None;
    let mut batch_dir: Option<PathBuf> = None;
    let mut output: Option<PathBuf> = None;
    let mut output_dir: Option<PathBuf> = None;
    let mut snapshot_dir: Option<PathBuf> = None;
    let mut update_snapshots = false;
    let mut report_json: Option<PathBuf> = None;
    let mut report_md: Option<PathBuf> = None;
    let mut report_html: Option<PathBuf> = None;
    let mut source_map: Option<PathBuf> = None;
    let mut html_report = false;
    let mut source_map_report = false;
    let mut strict = false;
    let mut fail_on_todo_rate: Option<f64> = None;
    let mut dry_run = false;
    let mut metrics_csv: Option<PathBuf> = None;
    let mut perf_report_json: Option<PathBuf> = None;
    let mut javac_check = false;
    let mut javac_cmd = String::from("javac");
    let mut class_name = String::from("MainProgram");
    let mut java_target = JavaTarget::Java21;
    let mut mode = ParseMode::Auto;
    let mut jobs: usize = 1;

    let args: Vec<String> = raw_args;
    let mut i = 1usize;
    while i < args.len() {
        match args[i].as_str() {
            "--input" | "-i" => {
                i += 1;
                let value = args
                    .get(i)
                    .ok_or_else(|| String::from("missing value for --input"))?;
                input = Some(PathBuf::from(value));
            }
            "--batch-dir" => {
                i += 1;
                let value = args
                    .get(i)
                    .ok_or_else(|| String::from("missing value for --batch-dir"))?;
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
                let value = args
                    .get(i)
                    .ok_or_else(|| String::from("missing value for --output-dir"))?;
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
            "--java-target" => {
                i += 1;
                let value = args
                    .get(i)
                    .ok_or_else(|| String::from("missing value for --java-target"))?;
                java_target = transpiler::parse_java_target(value)?;
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
            "--report-html" => {
                i += 1;
                let value = args
                    .get(i)
                    .ok_or_else(|| String::from("missing value for --report-html"))?;
                report_html = Some(PathBuf::from(value));
            }
            "--source-map" => {
                i += 1;
                let value = args
                    .get(i)
                    .ok_or_else(|| String::from("missing value for --source-map"))?;
                source_map = Some(PathBuf::from(value));
            }
            "--html-report" => {
                html_report = true;
            }
            "--source-map-report" => {
                source_map_report = true;
            }
            "--strict" => {
                strict = true;
            }
            "--fail-on-todo-rate" => {
                i += 1;
                let value = args
                    .get(i)
                    .ok_or_else(|| String::from("missing value for --fail-on-todo-rate"))?;
                let parsed = value
                    .parse::<f64>()
                    .map_err(|_| format!("invalid --fail-on-todo-rate '{}'", value))?;
                if !(0.0..=1.0).contains(&parsed) {
                    return Err(String::from(
                        "--fail-on-todo-rate must be between 0.0 and 1.0",
                    ));
                }
                fail_on_todo_rate = Some(parsed);
            }
            "--dry-run" => {
                dry_run = true;
            }
            "--redact" => {
                // Reserved for commercial report masking. Current parser never executes source.
            }
            "--config" | "--profile" | "--baseline" | "--sarif" => {
                i += 1;
                args.get(i)
                    .ok_or_else(|| format!("missing value for {}", args[i - 1]))?;
            }
            "--metrics-csv" => {
                i += 1;
                let value = args
                    .get(i)
                    .ok_or_else(|| String::from("missing value for --metrics-csv"))?;
                metrics_csv = Some(PathBuf::from(value));
            }
            "--perf-report-json" => {
                i += 1;
                let value = args
                    .get(i)
                    .ok_or_else(|| String::from("missing value for --perf-report-json"))?;
                perf_report_json = Some(PathBuf::from(value));
            }
            "--javac-check" => {
                javac_check = true;
            }
            "--javac-cmd" => {
                i += 1;
                let value = args
                    .get(i)
                    .ok_or_else(|| String::from("missing value for --javac-cmd"))?;
                javac_cmd = value.clone();
            }
            "--mode" => {
                i += 1;
                let value = args
                    .get(i)
                    .ok_or_else(|| String::from("missing value for --mode"))?;
                mode = parse_mode(value)?;
            }
            "--jobs" => {
                i += 1;
                let value = args
                    .get(i)
                    .ok_or_else(|| String::from("missing value for --jobs"))?;
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
        return Err(String::from(
            "--input and --batch-dir are mutually exclusive",
        ));
    }
    if input.is_none() && batch_dir.is_none() {
        return Err(String::from("either --input or --batch-dir is required"));
    }
    if batch_dir.is_some() && output.is_some() {
        return Err(String::from(
            "--output is not used in --batch-dir mode, use --output-dir",
        ));
    }
    if batch_dir.is_some() && (report_json.is_some() || report_md.is_some()) {
        return Err(String::from(
            "--report-json/--report-md are per-file in batch mode and auto-generated to --output-dir",
        ));
    }
    if batch_dir.is_some() && report_html.is_some() {
        return Err(String::from(
            "--report-html is per-file in batch mode and auto-generated when --html-report is used",
        ));
    }
    if batch_dir.is_some() && source_map.is_some() {
        return Err(String::from(
            "--source-map is per-file in batch mode and auto-generated when --source-map-report is used",
        ));
    }
    if batch_dir.is_none() && metrics_csv.is_some() {
        return Err(String::from(
            "--metrics-csv is available only in --batch-dir mode",
        ));
    }
    if batch_dir.is_none() && perf_report_json.is_some() {
        return Err(String::from(
            "--perf-report-json is available only in --batch-dir mode",
        ));
    }
    if input.is_some() && batch_dir.is_none() && javac_check && output.is_none() {
        return Err(String::from(
            "--javac-check requires --output in single-file mode",
        ));
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
        report_html,
        source_map,
        html_report,
        source_map_report,
        strict,
        fail_on_todo_rate,
        dry_run,
        metrics_csv,
        perf_report_json,
        javac_check,
        javac_cmd,
        class_name,
        java_target,
        mode,
        jobs,
    })
}

fn print_help() {
    println!("rpg2java-transpiler");
    println!("Commands:");
    println!("  convert   Convert one RPG source file");
    println!("  batch     Convert a directory of RPG source files");
    println!("  report    Reserved alias for report-oriented conversion flows");
    println!("  verify    Reserved alias for verification-oriented conversion flows");
    println!("  coverage  Reserved alias for coverage-oriented batch flows");
    println!("  doctor    Print local environment diagnostics");
    println!("Single-file mode:");
    println!(
        "  rpg2java-transpiler convert --input <file> [--output <file>] [--class-name <name>] [--java-target java21|java25-stable] [--mode auto|free|fixed]"
    );
    println!(
        "                         [--report-json <file>] [--report-md <file>] [--report-html <file>] [--source-map <file>]"
    );
    println!(
        "                         [--snapshot-dir <dir>] [--update-snapshots] [--javac-check] [--javac-cmd <cmd>] [--strict] [--fail-on-todo-rate <0.0-1.0>] [--dry-run]"
    );
    println!("Batch mode:");
    println!(
        "  rpg2java-transpiler batch --batch-dir <dir> [--output-dir <dir>] [--java-target java21|java25-stable] [--mode auto|free|fixed] [--jobs <n>]"
    );
    println!(
        "                         [--snapshot-dir <dir>] [--update-snapshots] [--metrics-csv <file>] [--perf-report-json <file>] [--html-report] [--source-map-report] [--javac-check] [--javac-cmd <cmd>] [--strict] [--fail-on-todo-rate <0.0-1.0>] [--dry-run]"
    );
}

fn print_doctor() {
    println!("rpg2java doctor");
    println!("version: {}", env!("CARGO_PKG_VERSION"));
    println!("java targets: java21, java25-stable");
    println!("reports: json, markdown, html, source-map");
    println!("security: no dynamic RPG execution; local file conversion only");
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

fn print_summary(program: &Program, ir_program: &IrProgram, target: JavaTarget) {
    eprintln!("java target: {}", target.as_str());
    if !program.diagnostics.is_empty() {
        eprintln!("diagnostics:");
        for d in &program.diagnostics {
            eprintln!("  - {}", d.as_text());
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
    if !program.subroutine_routes.is_empty() {
        eprintln!("subroutine routes:");
        for route in &program.subroutine_routes {
            eprintln!(
                "  - {} -> {} (line {})",
                route.caller, route.callee, route.line
            );
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

fn enforce_todo_policy(
    ir_program: &IrProgram,
    strict: bool,
    fail_on_todo_rate: Option<f64>,
) -> Result<(), String> {
    let todos = ir_program
        .statements
        .iter()
        .filter(|s| matches!(s, ir::IrStmt::Todo { .. }))
        .count();
    let total = ir_program.statements.len();
    if strict && todos > 0 {
        return Err(format!("strict mode failed: {} TODO item(s) found", todos));
    }
    if let Some(limit) = fail_on_todo_rate {
        let rate = if total == 0 {
            0.0
        } else {
            todos as f64 / total as f64
        };
        if rate > limit {
            return Err(format!(
                "TODO rate {:.4} exceeds threshold {:.4}",
                rate, limit
            ));
        }
    }
    Ok(())
}

fn enforce_batch_todo_policy(
    metrics: &[BatchMetricsRow],
    strict: bool,
    fail_on_todo_rate: Option<f64>,
) -> Result<(), String> {
    let todos = metrics.iter().map(|r| r.todos).sum::<usize>();
    let statements = metrics.iter().map(|r| r.statements).sum::<usize>();
    if strict && todos > 0 {
        return Err(format!("strict mode failed: {} TODO item(s) found", todos));
    }
    if let Some(limit) = fail_on_todo_rate {
        let rate = if statements == 0 {
            0.0
        } else {
            todos as f64 / statements as f64
        };
        if rate > limit {
            return Err(format!(
                "batch TODO rate {:.4} exceeds threshold {:.4}",
                rate, limit
            ));
        }
    }
    Ok(())
}

fn run_javac_check(java_file: &Path, javac_cmd: &str) -> report::JavacCheckSummary {
    let mut summary = report::JavacCheckSummary {
        command: javac_cmd.to_string(),
        success: false,
        exit_code: None,
        detail: String::new(),
    };
    let mut parts = javac_cmd.split_whitespace();
    let Some(command) = parts.next() else {
        summary.detail = String::from("failed to execute command: empty command");
        return summary;
    };
    let output = match Command::new(command).args(parts).arg(java_file).output() {
        Ok(out) => out,
        Err(err) => {
            summary.detail = format!("failed to execute command: {err}");
            return summary;
        }
    };
    summary.success = output.status.success();
    summary.exit_code = output.status.code();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let joined = if stderr.is_empty() {
        stdout
    } else if stdout.is_empty() {
        stderr
    } else {
        format!("{stderr}\n{stdout}")
    };
    summary.detail = truncate_detail(&joined, 1200);
    summary
}

fn truncate_detail(input: &str, max: usize) -> String {
    if input.chars().count() <= max {
        return input.to_string();
    }
    let mut buf = String::new();
    for ch in input.chars().take(max) {
        buf.push(ch);
    }
    buf.push_str(" ...<truncated>");
    buf
}

fn resolve_perf_report_path(args: &Cli) -> Option<PathBuf> {
    if let Some(path) = &args.perf_report_json {
        return Some(path.clone());
    }
    args.metrics_csv
        .as_ref()
        .map(|p| derive_perf_report_path_from_metrics(p.as_path()))
}

fn derive_perf_report_path_from_metrics(metrics_path: &Path) -> PathBuf {
    let parent = metrics_path.parent().unwrap_or_else(|| Path::new("."));
    let stem = metrics_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("metrics");
    parent.join(format!("{stem}.summary.json"))
}

fn write_perf_report_json(
    path: &Path,
    rows: &[BatchMetricsRow],
    success: usize,
    failed: usize,
    jobs: usize,
) -> Result<(), String> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent).map_err(|e| {
        format!(
            "failed to create perf report directory {}: {e}",
            parent.display()
        )
    })?;

    let ok_rows: Vec<&BatchMetricsRow> = rows.iter().filter(|r| r.status == "ok").collect();
    let ng_rows: Vec<&BatchMetricsRow> = rows.iter().filter(|r| r.status == "ng").collect();
    let elapsed: Vec<u128> = ok_rows.iter().map(|r| r.elapsed_ms).collect();
    let p50 = percentile(&elapsed, 50);
    let p95 = percentile(&elapsed, 95);
    let p99 = percentile(&elapsed, 99);
    let avg_ms = if ok_rows.is_empty() {
        0.0
    } else {
        ok_rows.iter().map(|r| r.elapsed_ms as f64).sum::<f64>() / ok_rows.len() as f64
    };
    let avg_todo = if ok_rows.is_empty() {
        0.0
    } else {
        ok_rows.iter().map(|r| r.todo_rate).sum::<f64>() / ok_rows.len() as f64
    };
    let avg_ms_per_kib = if ok_rows.is_empty() {
        0.0
    } else {
        ok_rows
            .iter()
            .map(|r| {
                let kib = (r.input_bytes as f64 / 1024.0).max(0.001);
                r.elapsed_ms as f64 / kib
            })
            .sum::<f64>()
            / ok_rows.len() as f64
    };

    let mut slow_rows = ok_rows.clone();
    slow_rows.sort_by(|a, b| b.elapsed_ms.cmp(&a.elapsed_ms));
    slow_rows.truncate(5);

    let mut todo_rows = ok_rows.clone();
    todo_rows.sort_by(|a, b| {
        b.todo_rate
            .partial_cmp(&a.todo_rate)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    todo_rows.truncate(5);

    let threshold_failed = 0usize;
    let threshold_p95_ms = p50.saturating_mul(2).max(20);
    let threshold_avg_todo_rate = 0.20_f64;
    let threshold_jobs_min = if rows.len() >= 4 { 2usize } else { 1usize };

    let signal_failed = failed > threshold_failed;
    let signal_slow_tail = p95 > threshold_p95_ms;
    let signal_high_todo = avg_todo >= threshold_avg_todo_rate;
    let signal_low_parallelism = jobs < threshold_jobs_min;

    let mut recommendations: Vec<String> = Vec::new();
    if signal_failed {
        if !ng_rows.is_empty() {
            let categories = collect_failure_categories(&ng_rows);
            let keys: Vec<String> = categories.iter().map(|c| c.0.clone()).collect();
            recommendations.push(format!(
                "失敗カテゴリ: {}。category別の対応を優先してください。",
                keys.join(", ")
            ));
        }
        recommendations.push(format!(
            "変換失敗が {} 件あります。失敗ファイルの文字コード・構文差分を先に解消してください。",
            failed
        ));
    }
    if signal_low_parallelism {
        recommendations.push(String::from(
            "並列度が低いです。CPUコア数に応じて --jobs を上げて処理時間を短縮してください。",
        ));
        recommendations.push(format!(
            "根拠: jobs={} < threshold_jobs_min={}",
            jobs, threshold_jobs_min
        ));
    }
    if signal_slow_tail {
        recommendations.push(format!(
            "p95 が偏っています（p50={}ms, p95={}ms, threshold={}ms）。重いファイルを先に分離して段階実行してください。",
            p50, p95, threshold_p95_ms
        ));
    }
    if signal_high_todo {
        recommendations.push(format!(
            "TODO率が高めです（avg_todo_rate={:.4}, threshold={:.4}）。未対応命令（CHAIN/SETLL/READE/EXSR 等）の優先実装で手戻りを減らしてください。",
            avg_todo, threshold_avg_todo_rate
        ));
    }
    if recommendations.is_empty() {
        recommendations.push(String::from("現時点で重大なボトルネックは見当たりません。"));
    }

    let mut out = String::new();
    out.push_str("{\n");
    out.push_str("  \"summary\": {\n");
    out.push_str(&format!("    \"total\": {},\n", rows.len()));
    out.push_str(&format!("    \"success\": {},\n", success));
    out.push_str(&format!("    \"failed\": {},\n", failed));
    out.push_str(&format!("    \"jobs\": {},\n", jobs));
    out.push_str(&format!("    \"p50_ms\": {},\n", p50));
    out.push_str(&format!("    \"p95_ms\": {},\n", p95));
    out.push_str(&format!("    \"p99_ms\": {},\n", p99));
    out.push_str(&format!("    \"avg_ms\": {:.3},\n", avg_ms));
    out.push_str(&format!("    \"avg_todo_rate\": {:.4},\n", avg_todo));
    out.push_str(&format!("    \"avg_ms_per_kib\": {:.4}\n", avg_ms_per_kib));
    out.push_str("  },\n");
    out.push_str("  \"thresholds\": {\n");
    out.push_str(&format!("    \"failed_max\": {},\n", threshold_failed));
    out.push_str(&format!("    \"p95_ms_warn\": {},\n", threshold_p95_ms));
    out.push_str(&format!(
        "    \"avg_todo_rate_warn\": {:.4},\n",
        threshold_avg_todo_rate
    ));
    out.push_str(&format!("    \"jobs_min\": {}\n", threshold_jobs_min));
    out.push_str("  },\n");
    out.push_str("  \"signals\": {\n");
    out.push_str(&format!("    \"has_failures\": {},\n", signal_failed));
    out.push_str(&format!("    \"slow_tail\": {},\n", signal_slow_tail));
    out.push_str(&format!("    \"high_todo\": {},\n", signal_high_todo));
    out.push_str(&format!(
        "    \"low_parallelism\": {}\n",
        signal_low_parallelism
    ));
    out.push_str("  },\n");
    out.push_str("  \"slow_files\": [\n");
    for (idx, row) in slow_rows.iter().enumerate() {
        let comma = if idx + 1 == slow_rows.len() { "" } else { "," };
        out.push_str(&format!(
            "    {{\"class_name\":\"{}\",\"input\":\"{}\",\"elapsed_ms\":{},\"todo_rate\":{:.4}}}{}\n",
            json_escape(&row.class_name),
            json_escape(&row.input.display().to_string()),
            row.elapsed_ms,
            row.todo_rate,
            comma
        ));
    }
    out.push_str("  ],\n");
    out.push_str("  \"high_todo_files\": [\n");
    for (idx, row) in todo_rows.iter().enumerate() {
        let comma = if idx + 1 == todo_rows.len() { "" } else { "," };
        out.push_str(&format!(
            "    {{\"class_name\":\"{}\",\"input\":\"{}\",\"todo_rate\":{:.4},\"statements\":{},\"todos\":{}}}{}\n",
            json_escape(&row.class_name),
            json_escape(&row.input.display().to_string()),
            row.todo_rate,
            row.statements,
            row.todos,
            comma
        ));
    }
    out.push_str("  ],\n");
    out.push_str("  \"failure_categories\": [\n");
    let categories = collect_failure_categories(&ng_rows);
    for (idx, (key, label, count, samples)) in categories.iter().enumerate() {
        let comma = if idx + 1 == categories.len() { "" } else { "," };
        out.push_str(&format!(
            "    {{\"key\":\"{}\",\"label\":\"{}\",\"count\":{},\"samples\":[",
            json_escape(key),
            json_escape(label),
            count
        ));
        for (sidx, sample) in samples.iter().enumerate() {
            let scomma = if sidx + 1 == samples.len() { "" } else { "," };
            out.push_str(&format!("\"{}\"{}", json_escape(sample), scomma));
        }
        out.push_str(&format!("]}}{}\n", comma));
    }
    out.push_str("  ],\n");
    out.push_str("  \"recommendations\": [\n");
    for (idx, rec) in recommendations.iter().enumerate() {
        let comma = if idx + 1 == recommendations.len() {
            ""
        } else {
            ","
        };
        out.push_str(&format!("    \"{}\"{}\n", json_escape(rec), comma));
    }
    out.push_str("  ]\n");
    out.push_str("}\n");

    fs::write(path, out).map_err(|e| format!("failed to write perf report {}: {e}", path.display()))
}

fn collect_failure_categories(
    ng_rows: &[&BatchMetricsRow],
) -> Vec<(String, String, usize, Vec<String>)> {
    let mut map: BTreeMap<String, (String, usize, Vec<String>)> = BTreeMap::new();
    for row in ng_rows {
        let err = row.error.as_deref().unwrap_or("unknown error");
        let (key, label) = classify_failure_reason(err);
        let entry = map
            .entry(key.to_string())
            .or_insert_with(|| (label.to_string(), 0usize, Vec::new()));
        entry.1 += 1;
        if entry.2.len() < 3 {
            entry.2.push(format!(
                "{}: {}",
                row.input.display(),
                truncate_detail(err, 160)
            ));
        }
    }
    map.into_iter()
        .map(|(k, (label, count, samples))| (k, label, count, samples))
        .collect()
}

fn classify_failure_reason(error: &str) -> (&'static str, &'static str) {
    let lower = error.to_ascii_lowercase();
    if lower.contains("snapshot mismatch") {
        return ("snapshot_mismatch", "snapshot mismatch");
    }
    if lower.contains("javac check failed") {
        return ("javac_check_failed", "javac check failed");
    }
    if lower.contains("valid utf-8") || (lower.contains("read failed") && lower.contains("utf-8")) {
        return ("input_decode", "input decode");
    }
    if lower.contains("read failed") {
        return ("input_read", "input read");
    }
    if lower.contains("write failed") || lower.contains("failed to write") {
        return ("output_write", "output write");
    }
    if lower.contains("stat failed") {
        return ("input_stat", "input stat");
    }
    if lower.contains("worker thread panicked") {
        return ("worker_panic", "worker panic");
    }
    ("other", "other")
}

fn json_escape(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err}");
        print_help();
        std::process::exit(1);
    }
}
