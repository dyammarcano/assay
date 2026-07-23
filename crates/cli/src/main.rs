use clap::{Parser, Subcommand};
use std::path::PathBuf;
use wrapswap_core::{
    analyze, detect_native_modules, generate_sidecar, parse_appx_manifest, parse_electron,
    render_divergence, render_report, scaffold, Matrix, Profile, Source,
};

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

fn load_matrix(path: &Option<PathBuf>) -> Matrix {
    match path {
        Some(p) => {
            let s = std::fs::read_to_string(p).expect("read --matrix file");
            Matrix::from_toml(&s).expect("parse --matrix file")
        }
        None => Matrix::embedded(),
    }
}

fn resolve_profile(
    profile: &Option<PathBuf>,
    appx: &Option<PathBuf>,
    epkg: &Option<PathBuf>,
    emain: &Option<PathBuf>,
) -> Profile {
    if let Some(p) = profile {
        let s = std::fs::read_to_string(p).expect("read --profile");
        return Profile::from_toml(&s).expect("parse --profile");
    }
    if let Some(a) = appx {
        let xml = std::fs::read_to_string(a).expect("read --appx");
        return parse_appx_manifest(&xml);
    }
    if let (Some(pkg), Some(main)) = (epkg, emain) {
        let pj = std::fs::read_to_string(pkg).expect("read --electron-pkg");
        let ms = std::fs::read_to_string(main).expect("read --electron-main");
        return parse_electron(&pj, &ms);
    }
    eprintln!("error: provide --profile, --appx, or --electron-pkg + --electron-main");
    std::process::exit(2);
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Report {
            matrix,
            out,
            source,
        } => {
            let m = load_matrix(&matrix);
            let only = match source.as_deref() {
                None => None,
                Some("uwp") => Some(Source::Uwp),
                Some("electron") => Some(Source::Electron),
                Some(other) => {
                    eprintln!("error: --source must be 'uwp' or 'electron', got '{other}'");
                    std::process::exit(2);
                }
            };
            let md = render_report(&m, only);
            match out {
                Some(p) => std::fs::write(&p, md).expect("write --out file"),
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
            let m = load_matrix(&matrix);
            let p = resolve_profile(&profile, &appx, &electron_pkg, &electron_main);
            if let Some(out) = emit_profile {
                std::fs::write(&out, p.to_toml()).expect("write --emit-profile");
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
            let m = load_matrix(&matrix);
            let p = resolve_profile(&profile, &appx, &electron_pkg, &electron_main);
            let a = analyze(&m, &p);
            let s = scaffold(&a);
            std::fs::create_dir_all(&out_dir).expect("create --out-dir");
            std::fs::write(out_dir.join("bridge.rs"), &s.rust).expect("write bridge.rs");
            std::fs::write(out_dir.join("deps.txt"), s.cargo_deps.join("\n"))
                .expect("write deps.txt");
            eprintln!("wrote bridge.rs + deps.txt to {}", out_dir.display());
        }
        Commands::Sidecar {
            electron_pkg,
            out_dir,
        } => {
            let pkg = std::fs::read_to_string(&electron_pkg).expect("read --electron-pkg");
            let modules = detect_native_modules(&pkg);
            let kit = generate_sidecar(&modules);
            let sidecar_src = out_dir.join("sidecar").join("src");
            std::fs::create_dir_all(&sidecar_src).expect("create sidecar/src");
            std::fs::write(out_dir.join("sidecar").join("Cargo.toml"), &kit.cargo_toml)
                .expect("write sidecar/Cargo.toml");
            std::fs::write(sidecar_src.join("main.rs"), &kit.main_rs).expect("write main.rs");
            std::fs::write(out_dir.join("sidecar_client.rs"), &kit.client_rs)
                .expect("write sidecar_client.rs");
            std::fs::write(out_dir.join("MIGRATION.md"), &kit.migration_md)
                .expect("write MIGRATION.md");
            eprintln!(
                "detected {} native module(s); wrote sidecar kit to {}",
                modules.len(),
                out_dir.display()
            );
        }
    }
}
