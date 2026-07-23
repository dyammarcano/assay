use clap::{Parser, Subcommand};
use core::{
    analyze, detect_native_modules, generate_sidecar, parse_appx_manifest, parse_electron,
    render_divergence, render_report, scaffold, Matrix, Profile, Source,
};
use std::path::{Path, PathBuf};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Exit code for a usage error (bad/missing arguments), distinct from a runtime failure.
const EXIT_USAGE: i32 = 2;

#[derive(Parser)]
#[command(
    name = "wrap-swap",
    about = "UWP/Electron \u{2192} Tauri parity toolkit"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Render the full capability/gap matrix as Markdown
    Report {
        #[arg(long)]
        matrix: Option<PathBuf>,
        #[arg(long)]
        out: Option<PathBuf>,
        /// Restrict to one source: uwp | electron
        #[arg(long)]
        source: Option<String>,
    },
    /// Analyze a capability profile (or parsed manifest) against the matrix
    Analyze {
        #[arg(long)]
        matrix: Option<PathBuf>,
        #[arg(long)]
        profile: Option<PathBuf>,
        #[arg(long)]
        appx: Option<PathBuf>,
        #[arg(long)]
        electron_pkg: Option<PathBuf>,
        #[arg(long)]
        electron_main: Option<PathBuf>,
        #[arg(long)]
        emit_profile: Option<PathBuf>,
    },
    /// Generate Rust/Tauri bridge scaffolding for a profile's gaps
    Scaffold {
        #[arg(long)]
        matrix: Option<PathBuf>,
        #[arg(long)]
        profile: Option<PathBuf>,
        #[arg(long)]
        appx: Option<PathBuf>,
        #[arg(long)]
        electron_pkg: Option<PathBuf>,
        #[arg(long)]
        electron_main: Option<PathBuf>,
        #[arg(long, default_value = "wrap-swap-out")]
        out_dir: PathBuf,
    },
    /// Detect Electron native modules and scaffold a sidecar migration kit
    Sidecar {
        #[arg(long)]
        electron_pkg: PathBuf,
        #[arg(long, default_value = "sidecar-out")]
        out_dir: PathBuf,
    },
}

fn read_file(p: &Path, what: &str) -> Result<String> {
    std::fs::read_to_string(p).map_err(|e| format!("{what} ({}): {e}", p.display()).into())
}

fn write_file(p: &Path, contents: &str, what: &str) -> Result<()> {
    std::fs::write(p, contents).map_err(|e| format!("{what} ({}): {e}", p.display()).into())
}

fn load_matrix(path: &Option<PathBuf>) -> Result<Matrix> {
    match path {
        Some(p) => {
            let s = read_file(p, "cannot read --matrix")?;
            Matrix::from_toml(&s)
                .map_err(|e| format!("invalid --matrix ({}): {e}", p.display()).into())
        }
        None => Ok(Matrix::embedded()),
    }
}

fn resolve_profile(
    profile: &Option<PathBuf>,
    appx: &Option<PathBuf>,
    epkg: &Option<PathBuf>,
    emain: &Option<PathBuf>,
) -> Result<Profile> {
    if let Some(p) = profile {
        let s = read_file(p, "cannot read --profile")?;
        return Profile::from_toml(&s)
            .map_err(|e| format!("invalid --profile ({}): {e}", p.display()).into());
    }
    if let Some(a) = appx {
        let xml = read_file(a, "cannot read --appx")?;
        return Ok(parse_appx_manifest(&xml));
    }
    if let (Some(pkg), Some(main)) = (epkg, emain) {
        let pj = read_file(pkg, "cannot read --electron-pkg")?;
        let ms = read_file(main, "cannot read --electron-main")?;
        return Ok(parse_electron(&pj, &ms));
    }
    eprintln!("error: provide --profile, --appx, or --electron-pkg + --electron-main");
    std::process::exit(EXIT_USAGE);
}

fn parse_source(source: &Option<String>) -> Result<Option<Source>> {
    match source.as_deref() {
        None => Ok(None),
        Some("uwp") => Ok(Some(Source::Uwp)),
        Some("electron") => Ok(Some(Source::Electron)),
        Some(other) => {
            eprintln!("error: --source must be 'uwp' or 'electron', got '{other}'");
            std::process::exit(EXIT_USAGE);
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
        Commands::Report {
            matrix,
            out,
            source,
        } => {
            let m = load_matrix(&matrix)?;
            let only = parse_source(&source)?;
            let md = render_report(&m, only);
            match out {
                Some(p) => write_file(&p, &md, "cannot write --out")?,
                None => print!("{md}"),
            }
        }
        Commands::Analyze {
            matrix,
            profile,
            appx,
            electron_pkg,
            electron_main,
            emit_profile,
        } => {
            let m = load_matrix(&matrix)?;
            let p = resolve_profile(&profile, &appx, &electron_pkg, &electron_main)?;
            if let Some(out) = emit_profile {
                write_file(&out, &p.to_toml(), "cannot write --emit-profile")?;
            }
            let a = analyze(&m, &p);
            println!("# Gap List\n");
            for g in &a.gaps {
                println!("- {} ({}): {:?}", g.name, g.id, g.tauri_path);
            }
            println!("\n{}", render_divergence(&a));
        }
        Commands::Scaffold {
            matrix,
            profile,
            appx,
            electron_pkg,
            electron_main,
            out_dir,
        } => {
            let m = load_matrix(&matrix)?;
            let p = resolve_profile(&profile, &appx, &electron_pkg, &electron_main)?;
            let a = analyze(&m, &p);
            let s = scaffold(&a);
            std::fs::create_dir_all(&out_dir)
                .map_err(|e| format!("cannot create --out-dir ({}): {e}", out_dir.display()))?;
            write_file(
                &out_dir.join("bridge.rs"),
                &s.rust,
                "cannot write bridge.rs",
            )?;
            write_file(
                &out_dir.join("deps.txt"),
                &s.cargo_deps.join("\n"),
                "cannot write deps.txt",
            )?;
            eprintln!("wrote bridge.rs + deps.txt to {}", out_dir.display());
        }
        Commands::Sidecar {
            electron_pkg,
            out_dir,
        } => {
            let pkg = read_file(&electron_pkg, "cannot read --electron-pkg")?;
            let modules = detect_native_modules(&pkg);
            let kit = generate_sidecar(&modules);
            let sidecar_src = out_dir.join("sidecar").join("src");
            std::fs::create_dir_all(&sidecar_src)
                .map_err(|e| format!("cannot create {}: {e}", sidecar_src.display()))?;
            write_file(
                &out_dir.join("sidecar").join("Cargo.toml"),
                &kit.cargo_toml,
                "cannot write sidecar/Cargo.toml",
            )?;
            write_file(
                &sidecar_src.join("main.rs"),
                &kit.main_rs,
                "cannot write sidecar main.rs",
            )?;
            write_file(
                &out_dir.join("sidecar_client.rs"),
                &kit.client_rs,
                "cannot write sidecar_client.rs",
            )?;
            write_file(
                &out_dir.join("MIGRATION.md"),
                &kit.migration_md,
                "cannot write MIGRATION.md",
            )?;
            write_file(
                &out_dir.join("tauri.conf.snippet.json"),
                &kit.tauri_conf_snippet,
                "cannot write tauri.conf.snippet.json",
            )?;
            eprintln!(
                "detected {} native module(s); wrote sidecar kit to {}",
                modules.len(),
                out_dir.display()
            );
        }
    }
    Ok(())
}
