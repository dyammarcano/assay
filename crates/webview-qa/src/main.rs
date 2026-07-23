use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};
use webview_qa::{
    diff, render_probe, render_report, ChromiumDriver, Config, EngineBlob, WebViewDriver,
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Parser)]
#[command(name = "webview-qa", about = "Cross-WebView divergence harness")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Diff two or more recorded per-engine probe blobs (JSON) into a report
    Diff {
        /// One --blob per engine capture (repeatable)
        #[arg(long, required = true)]
        blob: Vec<PathBuf>,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Render the injectable probe JS for an engine (a driver evals this in the page)
    Probe {
        /// Engine label recorded in the resulting blob (e.g. webview2)
        #[arg(long)]
        engine: String,
        /// webview-qa.toml; falls back to the built-in sample config
        #[arg(long)]
        config: Option<PathBuf>,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Write a starter webview-qa.toml
    InitConfig {
        #[arg(long, default_value = "webview-qa.toml")]
        out: PathBuf,
    },
    /// Capture a real EngineBlob from a live engine (headless Chromium/Edge)
    Capture {
        /// Page to load (http(s):// or file:///)
        #[arg(long)]
        url: String,
        /// Engine label recorded in the blob
        #[arg(long, default_value = "chromium-edge")]
        engine: String,
        /// Override the engine binary (defaults to auto-detected Edge/Chrome)
        #[arg(long)]
        exe: Option<PathBuf>,
        #[arg(long)]
        config: Option<PathBuf>,
        #[arg(long)]
        out: Option<PathBuf>,
    },
}

fn read_file(p: &Path, what: &str) -> Result<String> {
    std::fs::read_to_string(p).map_err(|e| format!("{what} ({}): {e}", p.display()).into())
}

fn emit(out: &Option<PathBuf>, text: &str) -> Result<()> {
    match out {
        Some(p) => std::fs::write(p, text)
            .map_err(|e| format!("cannot write --out ({}): {e}", p.display()).into()),
        None => {
            print!("{text}");
            Ok(())
        }
    }
}

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Diff { blob, out } => {
            let mut blobs: Vec<EngineBlob> = Vec::new();
            for p in &blob {
                let s = read_file(p, "cannot read --blob")?;
                let b: EngineBlob = serde_json::from_str(&s)
                    .map_err(|e| format!("invalid blob ({}): {e}", p.display()))?;
                blobs.push(b);
            }
            let divergences = diff(&blobs);
            emit(&out, &render_report(&blobs, &divergences))?;
        }
        Commands::Probe {
            engine,
            config,
            out,
        } => {
            let cfg = match &config {
                Some(p) => {
                    let s = read_file(p, "cannot read --config")?;
                    Config::from_toml(&s)
                        .map_err(|e| format!("invalid --config ({}): {e}", p.display()))?
                }
                None => Config::sample(),
            };
            emit(&out, &render_probe(&engine, &cfg))?;
        }
        Commands::InitConfig { out } => {
            std::fs::write(&out, Config::sample().to_toml())
                .map_err(|e| format!("cannot write {}: {e}", out.display()))?;
            eprintln!("wrote starter config to {}", out.display());
        }
        Commands::Capture {
            url,
            engine,
            exe,
            config,
            out,
        } => {
            let cfg = match &config {
                Some(p) => {
                    let s = read_file(p, "cannot read --config")?;
                    Config::from_toml(&s)
                        .map_err(|e| format!("invalid --config ({}): {e}", p.display()))?
                }
                None => Config::sample(),
            };
            let driver = match exe {
                Some(path) => ChromiumDriver {
                    exe: path,
                    engine: engine.clone(),
                },
                None => ChromiumDriver::detect()?.with_engine_label(engine.clone()),
            };
            let blob = driver.capture(&url, &cfg)?;
            let json = serde_json::to_string_pretty(&blob).expect("serialize blob");
            emit(&out, &format!("{json}\n"))?;
        }
    }
    Ok(())
}
