mod compressors;

use compression_core::{CompressionStats, ImageCompressor, compression_stats, load_image};
use compressors::{compressor_names, select_compressors};
use std::fmt::Write as _;
use std::fs::File;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::time::Instant;

const IMAGE_EXTS: &[&str] = &[
    "png", "jpg", "jpeg", "bmp", "gif", "webp", "tif", "tiff", "ppm", "pgm", "qoi",
];

/// One image run against the ordered list of selected compressors.
/// `cells[i]` is `None` if compressor `i` failed on this image.
pub struct Row {
    pub path: PathBuf,
    pub cells: Vec<Option<CompressionStats>>,
}

fn is_image(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| IMAGE_EXTS.iter().any(|x| x.eq_ignore_ascii_case(e)))
        .unwrap_or(false)
}

/// A file path returns just itself; a directory is walked recursively. Sorted
/// for stable output.
fn collect_images(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if root.is_file() {
        if is_image(root) {
            out.push(root.to_path_buf());
        }
        return out;
    }
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                stack.push(p);
            } else if is_image(&p) {
                out.push(p);
            }
        }
    }
    out.sort();
    out
}

fn run_one(path: &Path, compressor: &dyn ImageCompressor) -> Result<CompressionStats, String> {
    let image = load_image(&path.to_path_buf())?;
    let t0 = Instant::now();
    let compressed = compressor.compress(&image)?;
    let compress_ms = t0.elapsed().as_secs_f64() * 1000.0;
    let t1 = Instant::now();
    let decompressed = compressor.decompress(&compressed)?;
    let decompress_ms = t1.elapsed().as_secs_f64() * 1000.0;
    Ok(compression_stats(
        &image,
        &compressed,
        &decompressed,
        compress_ms,
        decompress_ms,
    ))
}

/// Run every image against every selected compressor, calling `on_item` just
/// before each (image, compressor) pair runs (for progress reporting).
pub fn run_matrix_with<F: FnMut(usize, usize, &Path, &str)>(
    paths: &[PathBuf],
    compressors: &[(&'static str, Box<dyn ImageCompressor>)],
    mut on_item: F,
) -> Vec<Row> {
    let total = paths.len() * compressors.len();
    let mut done = 0;
    paths
        .iter()
        .map(|path| Row {
            path: path.clone(),
            cells: compressors
                .iter()
                .map(|(name, c)| {
                    on_item(done, total, path, name);
                    let cell = run_one(path, c.as_ref()).ok();
                    done += 1;
                    cell
                })
                .collect(),
        })
        .collect()
}

/// Run the matrix with no progress reporting. Shared by CSV export and the TUI.
pub fn run_matrix(
    paths: &[PathBuf],
    compressors: &[(&'static str, Box<dyn ImageCompressor>)],
) -> Vec<Row> {
    run_matrix_with(paths, compressors, |_, _, _, _| {})
}

/// In-place progress bar on stderr (kept off stdout so results/CSV stay clean).
fn print_progress(done: usize, total: usize, label: &str) {
    const WIDTH: usize = 30;
    let filled = if total == 0 { WIDTH } else { done * WIDTH / total };
    let pct = if total == 0 { 100 } else { done * 100 / total };
    let bar = format!("{}{}", "█".repeat(filled), "░".repeat(WIDTH - filled));
    // `{:<28.28}` pads to a fixed width and truncates, so a shorter label
    // fully overwrites the previous (longer) one on the same line.
    eprint!("\r[{bar}] {done}/{total} ({pct}%) {label:<28.28}");
    let _ = std::io::stderr().flush();
}

/// Run the matrix while drawing the progress bar; used by `average`/`compare`.
fn run_with_progress(
    paths: &[PathBuf],
    compressors: &[(&'static str, Box<dyn ImageCompressor>)],
) -> Vec<Row> {
    let total = paths.len() * compressors.len();
    eprintln!(
        "Testing {} image(s) × {} compressor(s)…",
        paths.len(),
        compressors.len()
    );
    let rows = run_matrix_with(paths, compressors, |done, total, path, name| {
        print_progress(done, total, &format!("{} · {name}", short_name(path)));
    });
    print_progress(total, total, "done");
    eprintln!();
    rows
}

fn short_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned())
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max.saturating_sub(1)])
    }
}

fn ratio_pct(s: &CompressionStats) -> f64 {
    s.compressed_size as f64 / s.raw_size as f64 * 100.0
}

/// The category an image belongs to: the directory directly under `images/`
/// (`math`, `real-life`, …). Images outside the library fall back to the name
/// of their containing directory.
fn category(path: &Path) -> String {
    let dirs: Vec<String> = path
        .parent()
        .map(|p| {
            p.components()
                .filter_map(|c| c.as_os_str().to_str())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    if let Some(i) = dirs.iter().rposition(|c| c == DEFAULT_IMAGE_DIR) {
        return dirs
            .get(i + 1)
            .cloned()
            .unwrap_or_else(|| DEFAULT_IMAGE_DIR.to_string());
    }
    dirs.last().cloned().unwrap_or_else(|| "(root)".to_string())
}

/// Rows bucketed by category, in first-seen order (paths are sorted, so this
/// keeps categories alphabetical).
fn group_by_category(rows: &[Row]) -> Vec<(String, Vec<&Row>)> {
    let mut groups: Vec<(String, Vec<&Row>)> = Vec::new();
    for row in rows {
        let cat = category(&row.path);
        match groups.iter_mut().find(|(c, _)| *c == cat) {
            Some((_, group)) => group.push(row),
            None => groups.push((cat, vec![row])),
        }
    }
    groups
}

/// Average of one metric over a category, for compressor `i`. `None` if every
/// image in the category failed on that compressor.
fn avg(group: &[&Row], i: usize, pick: fn(&CompressionStats) -> f64) -> Option<f64> {
    let vals: Vec<f64> = group
        .iter()
        .filter_map(|r| r.cells[i].as_ref())
        .map(pick)
        .collect();
    if vals.is_empty() {
        None
    } else {
        Some(vals.iter().sum::<f64>() / vals.len() as f64)
    }
}

const METRICS: [(&str, &str, fn(&CompressionStats) -> f64); 3] = [
    ("Avg ratio", "%", ratio_pct),
    ("Avg compress", "ms", |s| s.compress_ms),
    ("Avg decompress", "ms", |s| s.decompress_ms),
];

/// One category × compressor table for a single metric.
fn format_metric_table(
    groups: &[(String, Vec<&Row>)],
    names: &[&str],
    title: &str,
    suffix: &str,
    pick: fn(&CompressionStats) -> f64,
) -> String {
    let col = 14usize;
    let label_w = 24usize;
    let mut out = String::new();

    let _ = writeln!(out, "{title} ({suffix})");
    let _ = write!(out, "{:<label_w$}", "Category");
    for n in names {
        let _ = write!(out, "{n:>col$}");
    }
    out.push('\n');
    out.push_str(&"-".repeat(label_w + names.len() * col));
    out.push('\n');

    for (cat, group) in groups {
        let label = format!("{cat} ({})", group.len());
        let _ = write!(out, "{:<label_w$}", truncate(&label, label_w - 1));
        for i in 0..names.len() {
            let cell = match avg(group, i, pick) {
                Some(v) => format!("{v:.1}{suffix}"),
                None => "error".to_string(),
            };
            let _ = write!(out, "{cell:>col$}");
        }
        out.push('\n');
    }
    out
}

/// Side-by-side comparison: one table per metric, averaged per image category.
fn format_compare_table(rows: &[Row], names: &[&str]) -> String {
    let groups = group_by_category(rows);
    let mut out = String::new();
    for (title, suffix, pick) in METRICS {
        out.push_str(&format_metric_table(&groups, names, title, suffix, pick));
        out.push('\n');
    }
    out
}

fn write_csv(path: &Path, rows: &[Row], names: &[&str]) -> std::io::Result<()> {
    let mut f = File::create(path)?;
    writeln!(
        f,
        "image,compressor,raw_bytes,compressed_bytes,ratio_pct,png_bytes,compress_ms,decompress_ms,lossless"
    )?;
    for row in rows {
        let img = row.path.to_string_lossy().replace('"', "\"\"");
        for (name, cell) in names.iter().zip(&row.cells) {
            if let Some(s) = cell {
                let png = s.png_size.map(|p| p.to_string()).unwrap_or_default();
                writeln!(
                    f,
                    "\"{img}\",{name},{},{},{:.3},{png},{:.3},{:.3},{}",
                    s.raw_size,
                    s.compressed_size,
                    ratio_pct(s),
                    s.compress_ms,
                    s.decompress_ms,
                    s.lossless,
                )?;
            }
        }
    }
    Ok(())
}

// ── Output modes ─────────────────────────────────────────────────────────────

/// How to render the results. Inferred from the input unless forced by an
/// explicit `image`/`average`/`compare` word on the command line.
#[derive(Clone, Copy, PartialEq)]
enum Mode {
    /// Full per-image stat block. Default for a single image.
    Detail,
    /// Per-image line plus an average block, one section per compressor.
    /// Default when a directory is run against a single compressor.
    Average,
    /// Side-by-side ratio table. Default when several compressors are involved.
    Compare,
}

fn print_detail(rows: &[Row], names: &[&str]) {
    for row in rows {
        for (name, cell) in names.iter().zip(&row.cells) {
            println!("=== {} ({name}) ===", row.path.display());
            match cell {
                Some(s) => s.print(),
                None => println!("error"),
            }
            println!();
        }
    }
}

/// Per-category averages for each compressor. Individual image results are not
/// printed here — use `--csv` for the raw per-image rows.
fn print_average(rows: &[Row], names: &[&str], label: &str) {
    let groups = group_by_category(rows);
    for (i, name) in names.iter().enumerate() {
        println!("=== {name} — {label} ===");
        for (cat, group) in &groups {
            let fmt = |v: Option<f64>, suffix: &str| match v {
                Some(v) => format!("{v:.1}{suffix}"),
                None => "error".to_string(),
            };
            let lossless = group
                .iter()
                .filter(|r| r.cells[i].as_ref().is_some_and(|s| s.lossless))
                .count();
            println!(
                "  {:<20} {:>3} img | ratio {:>8} | compress {:>9} | decompress {:>9} | lossless {lossless}/{}",
                truncate(cat, 20),
                group.len(),
                fmt(avg(group, i, ratio_pct), "%"),
                fmt(avg(group, i, |s| s.compress_ms), "ms"),
                fmt(avg(group, i, |s| s.decompress_ms), "ms"),
                group.len(),
            );
        }
        println!();
    }
}

fn run_cli(cli: Cli) -> Result<(), String> {
    let spec = if cli.compressors.is_empty() {
        "all".to_string()
    } else {
        cli.compressors.join(",")
    };
    let compressors = select_compressors(&spec).map_err(|name| {
        format!(
            "Unknown compressor '{name}'. Known: {}",
            compressor_names().join(", ")
        )
    })?;
    let names: Vec<&str> = compressors.iter().map(|(n, _)| *n).collect();

    // No path given: fall back to the bundled image library.
    let targets = if cli.targets.is_empty() {
        let default = PathBuf::from(DEFAULT_IMAGE_DIR);
        if !default.exists() {
            return Err(format!(
                "No image path given and default '{DEFAULT_IMAGE_DIR}' does not exist"
            ));
        }
        vec![default]
    } else {
        cli.targets
    };

    let mut paths: Vec<PathBuf> = targets.iter().flat_map(|t| collect_images(t)).collect();
    paths.sort();
    paths.dedup();
    let label = targets
        .iter()
        .map(|t| t.display().to_string())
        .collect::<Vec<_>>()
        .join(", ");
    if paths.is_empty() {
        return Err(format!("No images found in {label}"));
    }

    let mode = cli.mode.unwrap_or({
        if paths.len() == 1 {
            Mode::Detail
        } else if names.len() == 1 {
            Mode::Average
        } else {
            Mode::Compare
        }
    });

    let rows = if paths.len() * names.len() <= 1 {
        run_matrix(&paths, &compressors)
    } else {
        run_with_progress(&paths, &compressors)
    };

    match mode {
        Mode::Detail => print_detail(&rows, &names),
        Mode::Average => print_average(&rows, &names, &label),
        Mode::Compare => print!("{}", format_compare_table(&rows, &names)),
    }

    if let Some(path) = &cli.csv {
        write_csv(path, &rows, &names)
            .map_err(|e| format!("Failed to write CSV {}: {e}", path.display()))?;
        eprintln!("Wrote CSV: {}", path.display());
    }
    Ok(())
}

// ── Arg parsing ──────────────────────────────────────────────────────────────

const DEFAULT_IMAGE_DIR: &str = "images";

#[derive(Default)]
struct Cli {
    targets: Vec<PathBuf>,
    compressors: Vec<String>,
    mode: Option<Mode>,
    csv: Option<PathBuf>,
}

enum Parsed {
    Usage,
    Run(Cli),
}

/// True if every comma-separated part of `spec` names a known compressor (or
/// `all`), so a bare word like `huffman` or `rle,png` can be recognised as a
/// compressor selection rather than a path.
fn is_compressor_spec(spec: &str) -> bool {
    let parts: Vec<&str> = spec.split(',').map(str::trim).filter(|s| !s.is_empty()).collect();
    !parts.is_empty()
        && parts.iter().all(|p| {
            p.eq_ignore_ascii_case("all")
                || compressor_names().iter().any(|n| n.eq_ignore_ascii_case(p))
        })
}

/// Positional arguments classify themselves: an existing path is a target, a
/// known compressor name selects a compressor, and `image`/`average`/`compare`
/// force the output mode. Order does not matter, and both may repeat.
fn parse_args(args: &[String]) -> Result<Parsed, String> {
    let mut cli = Cli::default();
    let mut i = 0;
    while i < args.len() {
        let arg = args[i].as_str();
        // Value of an option flag, if this argument is one.
        let next = args.get(i + 1).cloned();
        match arg {
            "-h" | "--help" | "help" => return Ok(Parsed::Usage),
            "image" => cli.mode = Some(Mode::Detail),
            "average" | "avg" => cli.mode = Some(Mode::Average),
            "compare" => cli.mode = Some(Mode::Compare),
            "-c" | "--compressor" => {
                let v = next.ok_or("--compressor needs a value")?;
                cli.compressors
                    .extend(v.split(',').map(|s| s.trim().to_string()));
                i += 1;
            }
            "--csv" => {
                cli.csv = Some(PathBuf::from(next.ok_or("--csv needs a value")?));
                i += 1;
            }
            other if other.starts_with('-') => return Err(format!("Unknown option '{other}'")),
            other => {
                // A real path wins over a name clash, so an image literally
                // called `png` still works.
                if Path::new(other).exists() {
                    cli.targets.push(PathBuf::from(other));
                } else if is_compressor_spec(other) {
                    cli.compressors.extend(other.split(',').map(|s| s.trim().to_string()));
                } else {
                    return Err(format!(
                        "'{other}' is neither an existing path nor a known compressor. Known: {}",
                        compressor_names().join(", ")
                    ));
                }
            }
        }
        i += 1;
    }
    Ok(Parsed::Run(cli))
}

fn usage() {
    eprintln!("Usage: runner [<path>…] [<compressor>…] [image|average|compare] [--csv <file>]");
    eprintln!();
    eprintln!("Paths and compressor names may be given in any order:");
    eprintln!("  runner images/math/01_rle_solid_white.png Huffman   — stats for one image");
    eprintln!("  runner images/math RLE                              — averages over a directory");
    eprintln!("  runner images/math RLE Huffman                      — the two side by side");
    eprintln!("  runner images/math                                  — all compressors side by side");
    eprintln!("  runner Huffman                                      — all of '{DEFAULT_IMAGE_DIR}'");
    eprintln!();
    eprintln!(
        "Compressors: {} (or 'all', the default). Directories are walked recursively.",
        compressor_names().join(", ")
    );
    eprintln!("  image | average | compare   force the output format instead of inferring it");
    eprintln!("  --csv <file>                also write per-image/per-compressor rows as CSV");
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        return usage();
    }
    let result = match parse_args(&args) {
        Ok(Parsed::Usage) => return usage(),
        Ok(Parsed::Run(cli)) => run_cli(cli),
        Err(e) => Err(e),
    };
    if let Err(e) = result {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
