//! Find the apps that are actually installed on this machine, and where they live.
//!
//! Every other command in this toolkit needs a path handed to it — an `AppxManifest.xml`, or a
//! `package.json` plus a main-process directory. Finding those by hand is the tedious part, and
//! for UWP it is genuinely hard: `C:\Program Files\WindowsApps` denies directory listing to a
//! normal user, so the installed set cannot be enumerated by walking the filesystem.
//!
//! The way in is the per-user AppModel repository in the registry, which is readable without
//! elevation and records each package's root folder. Individual files *under* WindowsApps are
//! readable once you know the exact path, so a manifest reached this way opens fine.

use std::path::{Path, PathBuf};

/// Which porting story an app falls under. These are genuinely different problems: an Electron
/// app is Chromium-to-Chromium on Windows, whereas a UWP app renders native XAML.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppKind {
    Uwp,
    Electron,
}

impl AppKind {
    pub fn as_str(self) -> &'static str {
        match self {
            AppKind::Uwp => "uwp",
            AppKind::Electron => "electron",
        }
    }
}

/// One installed app, with everything needed to feed it to `analyze` / `scaffold`.
#[derive(Debug, Clone)]
pub struct InstalledApp {
    pub display_name: String,
    pub kind: AppKind,
    /// Package full name (UWP) or install-folder name (Electron).
    pub id: String,
    pub version: Option<String>,
    pub install_root: PathBuf,
    /// `AppxManifest.xml` — present only for UWP, and only when actually readable.
    pub manifest: Option<PathBuf>,
    /// Electron `package.json`.
    pub package_json: Option<PathBuf>,
    /// Electron main-process directory, resolved from `package.json`'s `main` field.
    pub main_dir: Option<PathBuf>,
    /// Set when the Electron app ships a packed `app.asar` instead of a loose `app/` directory.
    /// Such an app cannot be scanned without unpacking first, and we say so rather than
    /// reporting an empty result as if it were a clean bill of health.
    pub asar: Option<PathBuf>,
    /// PIDs of processes currently running out of this app's install root.
    pub running_pids: Vec<u32>,
}

impl InstalledApp {
    pub fn is_running(&self) -> bool {
        !self.running_pids.is_empty()
    }

    /// Why this app cannot be analyzed as-is, if that is the case.
    ///
    /// Returning the reason — rather than quietly omitting the app — is the point: a user who
    /// searched for an app by name needs to be told *why* it produced nothing.
    pub fn blocker(&self) -> Option<String> {
        match self.kind {
            AppKind::Uwp if self.manifest.is_none() => {
                Some("AppxManifest.xml is not readable (package may be for another user)".into())
            }
            AppKind::Electron if self.package_json.is_none() => Some(format!(
                "main process is packed in {} — unpack the asar first",
                self.asar
                    .as_deref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| "app.asar".into())
            )),
            _ => None,
        }
    }

    /// The ready-to-paste `assay analyze` invocation for this app, when one exists.
    pub fn analyze_command(&self) -> Option<String> {
        if self.blocker().is_some() {
            return None;
        }
        match self.kind {
            AppKind::Uwp => self
                .manifest
                .as_ref()
                .map(|m| format!("assay analyze --appx \"{}\"", m.display())),
            AppKind::Electron => match (&self.package_json, &self.main_dir) {
                (Some(p), Some(d)) => Some(format!(
                    "assay analyze --electron-pkg \"{}\" --electron-main \"{}\"",
                    p.display(),
                    d.display()
                )),
                _ => None,
            },
        }
    }
}

/// Options narrowing a discovery sweep.
#[derive(Debug, Clone, Default)]
pub struct DiscoverOptions {
    /// Restrict to one kind of app.
    pub kind: Option<AppKind>,
    /// Case-insensitive substring matched against the display name and the id.
    pub filter: Option<String>,
    /// Only report apps with at least one live process.
    pub running_only: bool,
}

impl DiscoverOptions {
    fn matches(&self, app: &InstalledApp) -> bool {
        if let Some(k) = self.kind {
            if app.kind != k {
                return false;
            }
        }
        if self.running_only && !app.is_running() {
            return false;
        }
        if let Some(f) = &self.filter {
            let f = f.to_lowercase();
            if !app.display_name.to_lowercase().contains(&f) && !app.id.to_lowercase().contains(&f)
            {
                return false;
            }
        }
        true
    }
}

/// Discover installed apps on this machine.
///
/// Results are sorted running-first, then by kind, then by name — the app you are looking at is
/// usually the one you are asking about.
pub fn discover(opts: &DiscoverOptions) -> Vec<InstalledApp> {
    let procs = sys::running_processes();
    let mut apps = Vec::new();
    if opts.kind != Some(AppKind::Electron) {
        apps.extend(sys::discover_uwp());
    }
    if opts.kind != Some(AppKind::Uwp) {
        apps.extend(discover_electron(&electron_search_roots()));
    }
    for app in &mut apps {
        app.running_pids = procs
            .iter()
            .filter(|(_, path)| under(path, &app.install_root))
            .map(|(pid, _)| *pid)
            .collect();
    }
    apps.retain(|a| opts.matches(a));
    apps.sort_by(|a, b| {
        b.is_running()
            .cmp(&a.is_running())
            .then(a.kind.as_str().cmp(b.kind.as_str()))
            .then(a.display_name.to_lowercase().cmp(&b.display_name.to_lowercase()))
    });
    apps
}

fn under(path: &Path, root: &Path) -> bool {
    path.starts_with(root)
}

/// Where third-party Windows apps get installed. Per-user `Programs` first, since that is where
/// most Electron apps land.
fn electron_search_roots() -> Vec<PathBuf> {
    ["LOCALAPPDATA", "ProgramFiles", "ProgramFiles(x86)"]
        .iter()
        .filter_map(std::env::var_os)
        .map(PathBuf::from)
        .flat_map(|p| {
            let programs = p.join("Programs");
            if programs.is_dir() {
                vec![programs, p]
            } else {
                vec![p]
            }
        })
        .filter(|p| p.is_dir())
        .collect()
}

/// An Electron install is recognised by a `resources` directory holding either a loose `app/`
/// (analyzable) or a packed `app.asar` (not, without unpacking).
///
/// Depth is bounded: these live a couple of levels down at most, and an unbounded walk of
/// `Program Files` is slow enough to feel broken.
fn discover_electron(roots: &[PathBuf]) -> Vec<InstalledApp> {
    let mut out = Vec::new();
    for root in roots {
        walk_for_resources(root, 0, 4, &mut out);
    }
    out.sort_by(|a, b| a.install_root.cmp(&b.install_root));
    out.dedup_by(|a, b| a.install_root == b.install_root);
    out
}

fn walk_for_resources(dir: &Path, depth: usize, max_depth: usize, out: &mut Vec<InstalledApp>) {
    if depth > max_depth {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return; // Permission-denied directories are common under Program Files; skip quietly.
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.eq_ignore_ascii_case("resources") {
            if let Some(app) = electron_app_at(&path) {
                out.push(app);
            }
            continue;
        }
        if is_skipped_dir(&name) {
            continue;
        }
        walk_for_resources(&path, depth + 1, max_depth, out);
    }
}

fn electron_app_at(resources: &Path) -> Option<InstalledApp> {
    let install_root = resources.parent()?.to_path_buf();
    let unpacked = resources.join("app").join("package.json");
    let asar = resources.join("app.asar");
    let (package_json, asar) = if unpacked.is_file() {
        (Some(unpacked), None)
    } else if asar.is_file() {
        (None, Some(asar))
    } else {
        return None;
    };

    let mut display_name = install_root
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "unknown".into());
    let mut version = None;
    let mut main_dir = None;

    if let Some(pj) = &package_json {
        if let Ok(text) = std::fs::read_to_string(pj) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
                if let Some(n) = v.get("productName").or_else(|| v.get("name")).and_then(|x| x.as_str()) {
                    display_name = n.to_string();
                }
                version = v.get("version").and_then(|x| x.as_str()).map(String::from);
                // `main` is a path relative to the package root; the main process is whatever
                // sits in its directory.
                let main = v.get("main").and_then(|x| x.as_str()).unwrap_or("index.js");
                let entry = resources.join("app").join(main);
                main_dir = entry.parent().map(Path::to_path_buf).filter(|d| d.is_dir());
            }
        }
    }

    Some(InstalledApp {
        id: install_root
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
        display_name,
        kind: AppKind::Electron,
        version,
        install_root,
        manifest: None,
        package_json,
        main_dir,
        asar,
        running_pids: Vec::new(),
    })
}

/// Split a package full name (`Name_Version_arch__hash`) into its name and version parts.
fn split_package_id(id: &str) -> (String, Option<String>) {
    let mut parts = id.split('_');
    let name = parts.next().unwrap_or(id).to_string();
    let version = parts.next().map(String::from);
    (name, version)
}

/// UWP display names are frequently an unresolved resource indirection rather than a name.
/// Two forms occur in the registry: a bare `ms-resource:appDisplayName`, and the indirect
/// `@{PackageFullName?ms-resource://...}` form. Neither is meaningful to a reader, so both fall
/// back to the package name.
fn clean_display_name(raw: Option<String>, package_id: &str) -> String {
    let (name, _) = split_package_id(package_id);
    match raw {
        Some(d)
            if !d.is_empty() && !d.starts_with("ms-resource:") && !d.starts_with("@{") =>
        {
            d
        }
        _ => name,
    }
}

/// Directories that never contain an installed app, but do contain thousands of copies of one.
///
/// `Temp` is the important entry: unpacked bundles and installer scratch space accumulate there,
/// and counting them inflates the result set by orders of magnitude while pointing the user at
/// paths that will be gone tomorrow.
const SKIP_DIRS: [&str; 6] = ["temp", "cache", "crashdumps", "node_modules", "logs", "backup"];

fn is_skipped_dir(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    SKIP_DIRS.contains(&lower.as_str()) || lower.starts_with('.')
}

#[cfg(windows)]
mod sys {
    use super::*;

    /// Per-user package repository. Readable without elevation, unlike WindowsApps itself.
    const REPO: &str = r"Software\Classes\Local Settings\Software\Microsoft\Windows\CurrentVersion\AppModel\Repository\Packages";

    pub fn discover_uwp() -> Vec<InstalledApp> {
        let hkcu = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER);
        let Ok(packages) = hkcu.open_subkey(REPO) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for key_name in packages.enum_keys().flatten() {
            let Ok(key) = packages.open_subkey(&key_name) else {
                continue;
            };
            let package_id: String = key
                .get_value("PackageID")
                .unwrap_or_else(|_| key_name.clone());
            let Ok(root) = key.get_value::<String, _>("PackageRootFolder") else {
                continue;
            };
            let install_root = PathBuf::from(root);
            let manifest = install_root.join("AppxManifest.xml");
            let (_, version) = split_package_id(&package_id);
            out.push(InstalledApp {
                display_name: clean_display_name(key.get_value("DisplayName").ok(), &package_id),
                kind: AppKind::Uwp,
                id: package_id,
                version,
                // A manifest we cannot open is reported as absent, and `blocker()` explains it.
                manifest: std::fs::File::open(&manifest).ok().map(|_| manifest),
                install_root,
                package_json: None,
                main_dir: None,
                asar: None,
                running_pids: Vec::new(),
            });
        }
        out
    }

    /// Snapshot every process with its executable path, so apps can be matched by install root
    /// rather than by guessing at process names.
    pub fn running_processes() -> Vec<(u32, PathBuf)> {
        use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE, MAX_PATH};
        use windows_sys::Win32::System::Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
        };
        use windows_sys::Win32::System::ProcessStatus::GetModuleFileNameExW;
        use windows_sys::Win32::System::Threading::{
            OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
        };

        let mut out = Vec::new();
        // SAFETY: a toolhelp snapshot is a plain handle; every call below is checked and the
        // handle is closed on every path out.
        unsafe {
            let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
            if snap == INVALID_HANDLE_VALUE {
                return out;
            }
            let mut entry: PROCESSENTRY32W = std::mem::zeroed();
            entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
            while Process32NextW(snap, &mut entry) != 0 {
                let pid = entry.th32ProcessID;
                // Many system processes deny even limited query access; those are simply not
                // ours to match against.
                // In windows-sys 0.52 a HANDLE is an isize, and a null handle is 0.
                let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
                if h == 0 {
                    continue;
                }
                let mut buf = [0u16; MAX_PATH as usize];
                // A null HMODULE asks for the main executable's own path.
                let len = GetModuleFileNameExW(h, 0, buf.as_mut_ptr(), buf.len() as u32);
                CloseHandle(h);
                if len > 0 {
                    out.push((pid, PathBuf::from(String::from_utf16_lossy(&buf[..len as usize]))));
                }
            }
            CloseHandle(snap);
        }
        out
    }
}

#[cfg(not(windows))]
mod sys {
    use super::*;

    // Windows-first by project scope: other platforms are deferred by decision, not blocked.
    // Returning empty keeps the crate portable without pretending to support them.
    pub fn discover_uwp() -> Vec<InstalledApp> {
        Vec::new()
    }

    pub fn running_processes() -> Vec<(u32, PathBuf)> {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_id_splits_into_name_and_version() {
        let (n, v) = split_package_id("Facebook.InstagramBeta_42.0.23.0_neutral__8xx8rvfyw5nnt");
        assert_eq!(n, "Facebook.InstagramBeta");
        assert_eq!(v.as_deref(), Some("42.0.23.0"));
    }

    #[test]
    fn ms_resource_display_names_fall_back_to_the_package_name() {
        let got = clean_display_name(
            Some("ms-resource:appDisplayName".into()),
            "Microsoft.DesktopAppInstaller_1.29.279.0_x64__8wekyb3d8bbwe",
        );
        assert_eq!(got, "Microsoft.DesktopAppInstaller");
    }

    #[test]
    fn indirect_resource_display_names_fall_back_to_the_package_name() {
        // Observed on this machine: the registry stores the indirect form for system packages,
        // which leaked verbatim into the listing before this was handled.
        let got = clean_display_name(
            Some(
                "@{Microsoft.Windows.StartMenuExperienceHost_10.0.26100.4768_neutral_neutral_cw5n1h2txyewy?ms-resource://Microsoft.Windows.StartMenuExperienceHost/StartMenuExperienceHost/PkgDisplayName}"
                    .into(),
            ),
            "Microsoft.Windows.StartMenuExperienceHost_10.0.26100.4768_neutral_neutral_cw5n1h2txyewy",
        );
        assert_eq!(got, "Microsoft.Windows.StartMenuExperienceHost");
    }

    #[test]
    fn scratch_directories_are_not_walked() {
        // Temp held 1123 unpacked app.asar copies; without this the Electron count was garbage.
        assert!(is_skipped_dir("Temp"));
        assert!(is_skipped_dir("temp"));
        assert!(is_skipped_dir("Cache"));
        assert!(is_skipped_dir("node_modules"));
        assert!(is_skipped_dir(".git"));
        assert!(!is_skipped_dir("Programs"));
        assert!(!is_skipped_dir("resources"));
    }

    #[test]
    fn real_display_names_are_kept() {
        let got = clean_display_name(Some("Instagram".into()), "Facebook.InstagramBeta_42.0.0.0");
        assert_eq!(got, "Instagram");
    }

    #[test]
    fn a_packed_asar_is_reported_as_a_blocker_not_silently_skipped() {
        let app = InstalledApp {
            display_name: "Packed".into(),
            kind: AppKind::Electron,
            id: "packed".into(),
            version: None,
            install_root: PathBuf::from("C:\\app"),
            manifest: None,
            package_json: None,
            main_dir: None,
            asar: Some(PathBuf::from("C:\\app\\resources\\app.asar")),
            running_pids: Vec::new(),
        };
        assert!(app.blocker().unwrap().contains("unpack the asar"));
        assert!(app.analyze_command().is_none());
    }

    #[test]
    fn an_unreadable_uwp_manifest_is_reported_as_a_blocker() {
        let app = InstalledApp {
            display_name: "Locked".into(),
            kind: AppKind::Uwp,
            id: "Locked_1.0.0.0".into(),
            version: Some("1.0.0.0".into()),
            install_root: PathBuf::from("C:\\WindowsApps\\Locked"),
            manifest: None,
            package_json: None,
            main_dir: None,
            asar: None,
            running_pids: Vec::new(),
        };
        assert!(app.blocker().unwrap().contains("not readable"));
    }

    #[test]
    fn filters_match_on_both_display_name_and_id() {
        let app = InstalledApp {
            display_name: "Instagram".into(),
            kind: AppKind::Uwp,
            id: "Facebook.InstagramBeta_42.0.23.0".into(),
            version: None,
            install_root: PathBuf::from("C:\\x"),
            manifest: Some(PathBuf::from("C:\\x\\AppxManifest.xml")),
            package_json: None,
            main_dir: None,
            asar: None,
            running_pids: Vec::new(),
        };
        let by_name = DiscoverOptions {
            filter: Some("instagram".into()),
            ..Default::default()
        };
        let by_id = DiscoverOptions {
            filter: Some("facebook".into()),
            ..Default::default()
        };
        let miss = DiscoverOptions {
            filter: Some("telegram".into()),
            ..Default::default()
        };
        assert!(by_name.matches(&app));
        assert!(by_id.matches(&app));
        assert!(!miss.matches(&app));
    }

    #[test]
    fn running_only_excludes_idle_apps() {
        let mut app = InstalledApp {
            display_name: "Idle".into(),
            kind: AppKind::Electron,
            id: "idle".into(),
            version: None,
            install_root: PathBuf::from("C:\\x"),
            manifest: None,
            package_json: Some(PathBuf::from("C:\\x\\resources\\app\\package.json")),
            main_dir: Some(PathBuf::from("C:\\x\\resources\\app")),
            asar: None,
            running_pids: Vec::new(),
        };
        let opts = DiscoverOptions {
            running_only: true,
            ..Default::default()
        };
        assert!(!opts.matches(&app));
        app.running_pids.push(1234);
        assert!(opts.matches(&app));
    }

    #[test]
    fn analyze_command_is_emitted_for_each_kind() {
        let uwp = InstalledApp {
            display_name: "Instagram".into(),
            kind: AppKind::Uwp,
            id: "Facebook.InstagramBeta_42.0.23.0".into(),
            version: None,
            install_root: PathBuf::from("C:\\x"),
            manifest: Some(PathBuf::from("C:\\x\\AppxManifest.xml")),
            package_json: None,
            main_dir: None,
            asar: None,
            running_pids: Vec::new(),
        };
        assert!(uwp.analyze_command().unwrap().contains("--appx"));

        let el = InstalledApp {
            kind: AppKind::Electron,
            manifest: None,
            package_json: Some(PathBuf::from("C:\\y\\resources\\app\\package.json")),
            main_dir: Some(PathBuf::from("C:\\y\\resources\\app\\out")),
            ..uwp.clone()
        };
        let cmd = el.analyze_command().unwrap();
        assert!(cmd.contains("--electron-pkg") && cmd.contains("--electron-main"));
    }
}
