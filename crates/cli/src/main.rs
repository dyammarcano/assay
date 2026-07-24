use clap::{Parser, Subcommand};
use corelib::{
    analyze, detect_native_modules, discover, generate_sidecar, parse_appx_manifest, parse_electron,
    render_divergence, render_report, scaffold, AppKind, DiscoverOptions, InstalledApp, Matrix,
    Profile, Source,
};
use std::path::{Path, PathBuf};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Exit code for a usage error (bad/missing arguments), distinct from a runtime failure.
const EXIT_USAGE: i32 = 2;

#[derive(Parser)]
#[command(
    name = "assay",
    version,
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
        /// Analyze an installed app by name, resolved via `discover` (e.g. --app instagram)
        #[arg(long)]
        app: Option<String>,
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
        #[arg(long, default_value = "assay-out")]
        out_dir: PathBuf,
    },
    /// Find UWP/Electron apps installed on this machine and where they live
    Discover {
        /// Restrict to one kind: uwp | electron
        #[arg(long)]
        kind: Option<String>,
        /// Case-insensitive substring matched against the app name and package id
        #[arg(long)]
        filter: Option<String>,
        /// Only apps with a live process
        #[arg(long)]
        running: bool,
    },
    /// Run the complete flow for an app: detects UWP vs Electron and takes the matching path
    Run {
        /// Installed app to process, resolved via `discover` (e.g. --app instagram)
        #[arg(long)]
        app: String,
        #[arg(long, default_value = "assay-run")]
        out_dir: PathBuf,
        #[arg(long)]
        matrix: Option<PathBuf>,
    },
    /// Port a hosted-PWA package to a runnable Tauri v2 project
    Port {
        /// Installed app to port, resolved via `discover` (e.g. --app instagram)
        #[arg(long)]
        app: Option<String>,
        /// Or point straight at an AppxManifest.xml
        #[arg(long)]
        appx: Option<PathBuf>,
        #[arg(long, default_value = "tauri-port")]
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

/// Source extensions an Electron main process can be written in.
const JS_EXTS: [&str; 6] = ["js", "mjs", "cjs", "ts", "tsx", "jsx"];

/// Read the Electron main-process source: either a single file, or **every** JS/TS file in a
/// directory (recursively, skipping `node_modules` and dotted dirs).
///
/// Real main processes span many files, so a single-file read silently under-reports the
/// capabilities an app uses. Returns the combined source and how many files were read so the
/// caller can say so out loud.
fn read_electron_main(p: &Path) -> Result<(String, usize)> {
    if !p.is_dir() {
        return Ok((read_file(p, "cannot read --electron-main")?, 1));
    }
    let mut buf = String::new();
    let mut count = 0usize;
    collect_js_sources(p, &mut buf, &mut count)?;
    if count == 0 {
        return Err(format!(
            "no JS/TS source files found under --electron-main ({})",
            p.display()
        )
        .into());
    }
    Ok((buf, count))
}

fn collect_js_sources(dir: &Path, buf: &mut String, count: &mut usize) -> Result<()> {
    let entries =
        std::fs::read_dir(dir).map_err(|e| format!("cannot read {}: {e}", dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("cannot read {}: {e}", dir.display()))?;
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if path.is_dir() {
            // Dependencies are not the app's own main process; dotted dirs are noise.
            if name == "node_modules" || name.starts_with('.') {
                continue;
            }
            collect_js_sources(&path, buf, count)?;
        } else if path
            .extension()
            .and_then(|x| x.to_str())
            .is_some_and(|e| JS_EXTS.contains(&e))
        {
            // Unreadable individual files are skipped rather than failing the whole scan.
            if let Ok(text) = std::fs::read_to_string(&path) {
                buf.push('\n');
                buf.push_str(&text);
                *count += 1;
            }
        }
    }
    Ok(())
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
        let (ms, files) = read_electron_main(main)?;
        if files == 1 && !main.is_dir() {
            // Say it plainly: a one-file scan is very likely partial.
            eprintln!(
                "note: scanned 1 file ({}). Electron main processes usually span several \
files — pass the main-process DIRECTORY to --electron-main for full coverage.",
                main.display()
            );
        } else {
            eprintln!(
                "note: scanned {files} source file(s) under {}",
                main.display()
            );
        }
        return Ok(parse_electron(&pj, &ms));
    }
    eprintln!("error: provide --profile, --appx, or --electron-pkg + --electron-main");
    std::process::exit(EXIT_USAGE);
}

fn parse_kind(kind: &Option<String>) -> Option<AppKind> {
    match kind.as_deref() {
        None => None,
        Some("uwp") => Some(AppKind::Uwp),
        Some("electron") => Some(AppKind::Electron),
        Some(other) => {
            eprintln!("error: --kind must be 'uwp' or 'electron', got '{other}'");
            std::process::exit(EXIT_USAGE);
        }
    }
}

fn render_discovery(apps: &[InstalledApp]) {
    if apps.is_empty() {
        println!("No matching apps found.");
        return;
    }
    for app in apps {
        let run = if app.is_running() {
            format!(" [running: {} pid(s)]", app.running_pids.len())
        } else {
            String::new()
        };
        let version = app.version.as_deref().unwrap_or("-");
        println!("{} ({}) {}{}", app.display_name, app.kind.as_str(), version, run);
        println!("  id:   {}", app.id);
        println!("  path: {}", app.install_root.display());
        match app.blocker() {
            // An app we cannot analyze is still listed, with the reason. Dropping it silently
            // would look identical to "you have no such app installed".
            Some(why) => println!("  NOT ANALYZABLE: {why}"),
            None => {
                if let Some(cmd) = app.analyze_command() {
                    println!("  {cmd}");
                }
            }
        }
        println!();
    }
    println!("{} app(s).", apps.len());
}

/// Resolve `--app <name>` to exactly one discovered app, or explain why not.
fn resolve_one_app(name: &str) -> Result<InstalledApp> {
    let apps = discover(&DiscoverOptions {
        filter: Some(name.to_string()),
        ..Default::default()
    });
    match apps.len() {
        0 => Err(format!(
            "no installed app matches '{name}' — run `assay discover` to see what is available"
        )
        .into()),
        1 => Ok(apps.into_iter().next().expect("length checked")),
        _ => {
            // Guessing which one the user meant risks porting the wrong app entirely.
            let names: Vec<String> = apps
                .iter()
                .map(|a| format!("{} ({})", a.display_name, a.id))
                .collect();
            Err(format!(
                "'{name}' is ambiguous — {} apps match:\n  {}",
                names.len(),
                names.join("\n  ")
            )
            .into())
        }
    }
}

/// Resolve `--app <name>` to its `AppxManifest.xml`.
fn resolve_app_manifest(name: &str) -> Result<PathBuf> {
    let app = resolve_one_app(name)?;
    if let Some(why) = app.blocker() {
        return Err(format!("'{}' cannot be ported: {why}", app.display_name).into());
    }
    app.manifest.clone().ok_or_else(|| {
        format!(
            "'{}' is an {} app; `port` currently handles hosted-PWA MSIX packages only",
            app.display_name,
            app.kind.as_str()
        )
        .into()
    })
}

/// Resolve `--app <name>` to a profile by discovering it on this machine.
fn resolve_app(name: &str) -> Result<Profile> {
    let apps = discover(&DiscoverOptions {
        filter: Some(name.to_string()),
        ..Default::default()
    });
    match apps.len() {
        0 => Err(format!(
            "no installed app matches '{name}' — run `assay discover` to see what is available"
        )
        .into()),
        1 => {
            let app = &apps[0];
            if let Some(why) = app.blocker() {
                return Err(format!("'{}' cannot be analyzed: {why}", app.display_name).into());
            }
            eprintln!(
                "note: resolved '{name}' to {} ({}) at {}",
                app.display_name,
                app.kind.as_str(),
                app.install_root.display()
            );
            match app.kind {
                AppKind::Uwp => {
                    let path = app.manifest.as_ref().expect("blocker() checked readability");
                    let xml = read_file(path, "cannot read discovered AppxManifest.xml")?;
                    Ok(parse_appx_manifest(&xml))
                }
                AppKind::Electron => {
                    let pkg = app.package_json.as_ref().expect("blocker() checked presence");
                    let main = app.main_dir.as_ref().ok_or_else(|| {
                        format!("'{}' has no resolvable main-process directory", app.display_name)
                    })?;
                    let pj = read_file(pkg, "cannot read discovered package.json")?;
                    let (ms, files) = read_electron_main(main)?;
                    eprintln!("note: scanned {files} source file(s) under {}", main.display());
                    Ok(parse_electron(&pj, &ms))
                }
            }
        }
        _ => {
            // Guessing which one the user meant risks analyzing the wrong app entirely.
            let names: Vec<String> = apps
                .iter()
                .map(|a| format!("{} ({})", a.display_name, a.id))
                .collect();
            Err(format!(
                "'{name}' is ambiguous — {} apps match:\n  {}",
                apps.len(),
                names.join("\n  ")
            )
            .into())
        }
    }
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
        Commands::Discover {
            kind,
            filter,
            running,
        } => {
            let apps = discover(&DiscoverOptions {
                kind: parse_kind(&kind),
                filter,
                running_only: running,
            });
            render_discovery(&apps);
        }
        Commands::Analyze {
            matrix,
            app,
            profile,
            appx,
            electron_pkg,
            electron_main,
            emit_profile,
        } => {
            let m = load_matrix(&matrix)?;
            let p = match &app {
                Some(name) => resolve_app(name)?,
                None => resolve_profile(&profile, &appx, &electron_pkg, &electron_main)?,
            };
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
        Commands::Run {
            app,
            out_dir,
            matrix,
        } => {
            let m = load_matrix(&matrix)?;
            let found = resolve_one_app(&app)?;
            eprintln!(
                "resolved '{}' to {} ({}) at {}",
                app,
                found.display_name,
                found.kind.as_str(),
                found.install_root.display()
            );

            // The kind selects the flow. This is the whole point: the two paths are different
            // problems, not two settings of one problem.
            let result = match found.kind {
                AppKind::Uwp => {
                    let path = found.manifest.as_ref().ok_or_else(|| {
                        format!(
                            "'{}' has no readable AppxManifest.xml: {}",
                            found.display_name,
                            found.blocker().unwrap_or_else(|| "unknown".into())
                        )
                    })?;
                    corelib::run_uwp_flow(
                        &m,
                        &corelib::UwpInput {
                            app_name: found.display_name.clone(),
                            manifest_xml: read_file(path, "cannot read AppxManifest.xml")?,
                        },
                    )
                }
                AppKind::Electron => {
                    let pkg = match &found.package_json {
                        Some(p) => read_file(p, "cannot read package.json")?,
                        // A packed app still runs the flow: the report explains what was
                        // unavailable rather than the command simply refusing.
                        None => "{}".to_string(),
                    };
                    let (src, count) = match &found.main_dir {
                        Some(d) => read_electron_main(d)?,
                        None => (String::new(), 0),
                    };
                    corelib::run_electron_flow(
                        &m,
                        &corelib::ElectronInput {
                            app_name: found.display_name.clone(),
                            package_json: pkg,
                            main_source: src,
                            file_count: count,
                            packed_asar: found.asar.clone(),
                        },
                    )
                }
            };

            std::fs::create_dir_all(&out_dir)
                .map_err(|e| format!("cannot create --out-dir ({}): {e}", out_dir.display()))?;
            result
                .write_to(&out_dir)
                .map_err(|e| format!("cannot write output to {}: {e}", out_dir.display()))?;

            eprintln!("\n{} flow — {}", result.kind, result.app_name);
            for step in &result.steps {
                eprintln!("  [{}] {} — {}", step.status.label(), step.name, step.status.detail());
            }
            eprintln!(
                "\n{} artifact(s) + REPORT.md written to {}",
                result.artifacts.len(),
                out_dir.display()
            );
            let blocked = result.blocked();
            if !blocked.is_empty() {
                eprintln!(
                    "\n{} step(s) BLOCKED — the results are incomplete, see REPORT.md",
                    blocked.len()
                );
            }
        }
        Commands::Port {
            app,
            appx,
            out_dir,
        } => {
            let manifest = match (&app, &appx) {
                (Some(name), _) => resolve_app_manifest(name)?,
                (None, Some(p)) => p.clone(),
                (None, None) => {
                    eprintln!("error: provide --app <name> or --appx <path>");
                    std::process::exit(EXIT_USAGE);
                }
            };
            let xml = read_file(&manifest, "cannot read AppxManifest.xml")?;
            let Some(pwa) = corelib::detect_pwa(&xml) else {
                // Refusing is the honest outcome: a conventional UWP app renders native XAML,
                // and there is no mechanical port from that to HTML.
                return Err(format!(
                    "{} is not a hosted-PWA package, so it cannot be ported this way.\n\
                     `port` handles MSIX packages that host a URL in Edge (HostId=\"PWA\"). A \
                     conventional UWP app renders native XAML; run `assay analyze` to see which \
                     of its capabilities Tauri can reproduce.",
                    manifest.display()
                )
                .into());
            };
            let port = corelib::port_pwa_to_tauri(&pwa);

            let src = out_dir.join("src-tauri").join("src");
            std::fs::create_dir_all(&src)
                .map_err(|e| format!("cannot create {}: {e}", src.display()))?;
            // Present but empty: `tauri build` expects frontendDist to exist even when the
            // window loads a remote URL.
            std::fs::create_dir_all(out_dir.join("dist"))
                .map_err(|e| format!("cannot create dist: {e}"))?;

            let tauri_dir = out_dir.join("src-tauri");
            write_file(&tauri_dir.join("Cargo.toml"), &port.cargo_toml, "cannot write Cargo.toml")?;
            write_file(&tauri_dir.join("build.rs"), &port.build_rs, "cannot write build.rs")?;
            // Without this file tauri-build refuses to run at all.
            let icons = tauri_dir.join("icons");
            std::fs::create_dir_all(&icons)
                .map_err(|e| format!("cannot create {}: {e}", icons.display()))?;
            std::fs::write(icons.join("icon.ico"), corelib::pwa::placeholder_icon())
                .map_err(|e| format!("cannot write icon.ico: {e}"))?;
            write_file(&tauri_dir.join("tauri.conf.json"), &port.tauri_conf, "cannot write tauri.conf.json")?;
            write_file(&src.join("main.rs"), &port.main_rs, "cannot write main.rs")?;
            write_file(&out_dir.join("MIGRATION.md"), &port.migration_md, "cannot write MIGRATION.md")?;

            eprintln!(
                "ported '{}' -> {}\n  start URL: {}\n  NOT ported ({}):",
                pwa.name,
                out_dir.display(),
                pwa.start_url,
                port.not_ported.len()
            );
            for n in &port.not_ported {
                eprintln!("    - {n}");
            }
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
