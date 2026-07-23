use clap::{Parser, Subcommand};
use std::path::PathBuf;
use webview_qa::{diff, render_report, EngineBlob};

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
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Diff { blob, out } => {
            let blobs: Vec<EngineBlob> = blob
                .iter()
                .map(|p| {
                    let s = std::fs::read_to_string(p)
                        .unwrap_or_else(|e| panic!("read {}: {e}", p.display()));
                    serde_json::from_str(&s)
                        .unwrap_or_else(|e| panic!("parse {}: {e}", p.display()))
                })
                .collect();
            let divergences = diff(&blobs);
            let report = render_report(&blobs, &divergences);
            match out {
                Some(p) => std::fs::write(&p, report).expect("write --out"),
                None => print!("{report}"),
            }
        }
    }
}
