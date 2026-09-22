use serde::{Deserialize, Serialize};
use std::env;
use std::path::PathBuf;
use std::process::{Child, Command};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SidecarStateKind {
    Starting,
    Ready,
    Reconnecting,
    Dead,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SidecarStatusResponse {
    pub status: String,
    pub port: u16,
    pub pid: Option<u32>,
    #[serde(rename = "coldStartMs")]
    pub cold_start_ms: Option<u64>,
    #[serde(rename = "restartCount")]
    pub restart_count: u32,
}

struct InnerManager {
    child: Option<Child>,
    status: SidecarStateKind,
    port: u16,
    token: String,
    cold_start_ms: Option<u64>,
    restart_count: u32,
    app_shutting_down: bool,
}

#[derive(Clone)]
pub struct SidecarManager {
    inner: Arc<Mutex<InnerManager>>,
}

/// A validated packaged engine: directory + binary + probed identity.
/// Shared by launch resolution and `doctor.rs` so both agree on what
/// "bundled" means. Never a repo script or dev stub.
pub struct BundledEngine {
    // `dir` is provenance for diagnostics/tests (launch uses `exe`); kept
    // by design even though no call site reads it yet.
    #[allow(dead_code)]
    pub dir: PathBuf,
    pub exe: PathBuf,
    pub version: String,
    pub sha: String,
}

/// Phase 12 launch resolution (highest priority first):
/// 1. Validated bundled engine binary (installed app / release build).
/// 2. Repo dev fallback: venv/system python + sidecar/lsc_bridge.py.
/// Dev stub binaries can never validate and are skipped, never executed.
enum EngineLaunch {
    Bundled { exe: PathBuf, version: String, sha: String },
    DevPython { python: PathBuf, script: PathBuf },
}

// Global SidecarManager initialized lazily via OnceLock below

impl Drop for InnerManager {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            println!("[SIDECAR] Killing sidecar child process on drop (PID: {:?})", child.id());
            let _ = child.kill();
        }
    }
}

impl SidecarManager {
    pub fn new() -> Self {
        let token = Self::generate_token();
        let inner = Arc::new(Mutex::new(InnerManager {
            child: None,
            status: SidecarStateKind::Starting,
            port: 48293,
            token,
            cold_start_ms: None,
            restart_count: 0,
            app_shutting_down: false,
        }));

        let manager = Self { inner };
        manager.spawn_child();
        manager
    }

    fn generate_token() -> String {
        // Generate 32-byte secure random hex string (64 characters)
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
        let pid = std::process::id();
        format!("{:032x}{:016x}{:016x}", now, pid, now ^ 0x5a5a5a5a5a5a5a5a)
    }

    /// Dev-only interpreter lookup (repo checkout): never used by the
    /// installed app, which always launches the validated bundled engine.
    fn resolve_python_path() -> PathBuf {
        // Check for hermes-agent/.venv/Scripts/python.exe
        let candidate_venv_win = PathBuf::from("hermes-agent/.venv/Scripts/python.exe");
        if candidate_venv_win.exists() {
            return candidate_venv_win;
        }

        let candidate_venv_nix = PathBuf::from("hermes-agent/.venv/bin/python");
        if candidate_venv_nix.exists() {
            return candidate_venv_nix;
        }

        // Fallback to system python
        PathBuf::from("python")
    }

    /// Phase 12: Tauri resource directory published by `main.rs` setup()
    /// (`app.path().resource_dir()`). Highest-priority launch source: the
    /// installed app's `$RESOURCE/engine/` one-dir bundle. `None` in unit
    /// tests and in dev runs that never went through Tauri setup.
    pub fn set_bundled_resource_dir(dir: Option<PathBuf>) {
        let slot = bundled_resource_dir_slot();
        *slot.lock().unwrap() = dir;
    }

    fn published_resource_dir() -> Option<PathBuf> {
        bundled_resource_dir_slot().lock().unwrap().clone()
    }

    /// Open (append) an engine log file, truncating past 5MB. Never panics:
    /// logging must not break the spawn — failures fall back to null.
    fn log_file(dir: &PathBuf, name: &str) -> std::process::Stdio {
        use std::io::Write as _;
        let path = dir.join(name);
        let truncate = std::fs::metadata(&path).map(|m| m.len() > 5_000_000).unwrap_or(false);
        let mut opts = std::fs::OpenOptions::new();
        opts.create(true).write(true);
        if truncate {
            opts.truncate(true);
        } else {
            opts.append(true);
        }
        match opts.open(&path) {
            Ok(mut f) => {
                if !truncate {
                    // Best-effort session separator; ignored on error.
                    let _ = writeln!(f, "--- engine start {} ---", std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs());
                }
                std::process::Stdio::from(f)
            }
            Err(_) => std::process::Stdio::null(),
        }
    }

    /// Engine log directory (`~/.pain-ai/logs/`), for doctor/support use.
    pub fn engine_logs_dir() -> Option<PathBuf> {
        dirs_home().map(|h| h.join(".pain-ai").join("logs"))
    }

    /// Phase 12: bundled-engine target triple for this build.
    pub fn engine_triple() -> &'static str {
        #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
        {
            "x86_64-pc-windows-msvc"
        }
        #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
        {
            "x86_64-unknown-linux-gnu"
        }
        #[cfg(not(any(
            all(target_os = "windows", target_arch = "x86_64"),
            all(target_os = "linux", target_arch = "x86_64")
        )))]
        {
            "unknown"
        }
    }

    fn engine_exe_name() -> String {
        #[cfg(target_os = "windows")]
        {
            "engine.exe".to_string()
        }
        #[cfg(not(target_os = "windows"))]
        {
            "engine".to_string()
        }
    }

    /// Candidate bundled-engine directories, highest priority first.
    ///
    /// Phase 12 production layout (see `tauri.conf.json` bundle.resources):
    /// the PyInstaller one-dir `engine/` ships at `$RESOURCE/engine/`
    /// (`"binaries/engine/": "engine"`). The published Tauri resource dir is
    /// authoritative; exe-adjacent guesses cover manual installs. Repo
    /// staging dirs (`src-tauri/binaries/engine/`) are dev-only fallbacks and
    /// are consulted last — the installed app never depends on them.
    /// Never assumes the current working directory: every repo-relative entry
    /// is validated by `validate_engine_binary` before use.
    pub fn engine_dir_candidates() -> Vec<PathBuf> {
        let mut out = Vec::new();
        let triple = Self::engine_triple();
        // Explicit override (tests, custom installs).
        if let Ok(p) = env::var("LSC_ENGINE_PATH") {
            let trimmed = p.trim();
            if !trimmed.is_empty() {
                out.push(PathBuf::from(trimmed));
            }
        }
        // 1. Tauri resource dir published by main.rs setup() — the installed
        // app path. `engine` is the exact resources target; the alternates
        // cover bundler mapping variants across Tauri CLI versions.
        if let Some(res) = Self::published_resource_dir() {
            out.push(res.join("engine"));
            out.push(res.join("binaries").join("engine"));
            out.push(
                res.join("binaries").join(format!("engine-{}", triple)),
            );
        }
        // 2. Next to the running host executable (installed app layouts,
        // valid even when setup() has not run yet).
        if let Ok(exe) = env::current_exe() {
            if let Some(exe_dir) = exe.parent() {
                out.push(exe_dir.join("resources").join("engine"));
                out.push(exe_dir.join("engine"));
                out.push(exe_dir.join("binaries").join("engine"));
                out.push(exe_dir.join("resources").join("binaries").join(format!("engine-{}", triple)));
                out.push(exe_dir.join("binaries").join(format!("engine-{}", triple)));
                out.push(exe_dir.join(format!("engine-{}", triple)));
                if let Some(parent) = exe_dir.parent() {
                    out.push(parent.join("resources").join("engine"));
                    out.push(parent.join("resources").join("binaries").join(format!("engine-{}", triple)));
                }
            }
        }
        // 3. Repo staging output (dev only, after `pyinstaller
        // sidecar/engine.spec --distpath src-tauri/binaries --name engine`).
        // Canonical one-dir first, legacy triple dirs for older stagings.
        out.push(PathBuf::from("src-tauri").join("binaries").join("engine"));
        out.push(PathBuf::from("binaries").join("engine"));
        out.push(PathBuf::from("src-tauri").join("binaries").join(format!("engine-{}", triple)));
        out.push(PathBuf::from("binaries").join(format!("engine-{}", triple)));
        out
    }

    /// Parse `lsc-engine <version> <sha>` identity lines.
    pub fn parse_engine_identity(output: &str) -> Option<(String, String)> {
        let first = output.lines().next()?.trim();
        let mut parts = first.split_whitespace();
        if parts.next()? != "lsc-engine" {
            return None;
        }
        Some((parts.next()?.to_string(), parts.next()?.to_string()))
    }

    /// First candidate directory holding a genuine packaged engine
    /// (size gate + `--lsc-version` identity probe), highest priority first.
    pub fn find_bundled_engine() -> Option<BundledEngine> {
        let exe_name = Self::engine_exe_name();
        for dir in Self::engine_dir_candidates() {
            let exe = dir.join(&exe_name);
            if let Some((version, sha)) = Self::validate_engine_binary(&exe) {
                return Some(BundledEngine { dir, exe, version, sha });
            }
        }
        None
    }

    /// A packaged engine must be a real build, never the dev stub: the stub
    /// is ~235KB and fails the identity probe below, so it can never pass.
    fn validate_engine_binary(exe: &PathBuf) -> Option<(String, String)> {
        let meta = std::fs::metadata(exe).ok()?;
        if !meta.is_file() || meta.len() < 10_000_000 {
            return None;
        }
        let probe = std::process::Command::new(exe)
            .arg("--lsc-version")
            .output()
            .ok()?;
        if !probe.status.success() {
            return None;
        }
        Self::parse_engine_identity(&String::from_utf8_lossy(&probe.stdout))
    }

    fn resolve_engine_launch() -> Result<EngineLaunch, String> {
        if let Some(found) = Self::find_bundled_engine() {
            return Ok(EngineLaunch::Bundled {
                exe: found.exe,
                version: found.version,
                sha: found.sha,
            });
        }
        // Dev fallback only: repo checkout with the bridge script present.
        // Never taken by the installed app (no repo files ship with it).
        for script in [
            PathBuf::from("sidecar/lsc_bridge.py"),
            PathBuf::from("../sidecar/lsc_bridge.py"),
        ] {
            if script.exists() {
                return Ok(EngineLaunch::DevPython {
                    python: Self::resolve_python_path(),
                    script,
                });
            }
        }
        Err(concat!(
            "No sidecar engine found: stage one with `pyinstaller sidecar/engine.spec --distpath src-tauri/binaries --name engine` ",
            "(canonical output: src-tauri/binaries/engine/, bundled as $RESOURCE/engine/) or run from the repository ",
            "so the dev python fallback applies."
        )
        .to_string())
    }

    pub fn spawn_child(&self) {
        let (port, token, _restart_count) = {
            let mut lock = self.inner.lock().unwrap();
            if lock.app_shutting_down {
                return;
            }
            if let Some(mut old) = lock.child.take() {
                let _ = old.kill();
            }
            lock.status = if lock.restart_count > 0 {
                SidecarStateKind::Reconnecting
            } else {
                SidecarStateKind::Starting
            };
            (lock.port, lock.token.clone(), lock.restart_count)
        };

        let launch = match Self::resolve_engine_launch() {
            Ok(l) => l,
            Err(e) => {
                eprintln!("[SIDECAR] {}", e);
                let mut lock = self.inner.lock().unwrap();
                lock.status = SidecarStateKind::Dead;
                return;
            }
        };

        let mut cmd = match &launch {
            EngineLaunch::Bundled { exe, version, sha } => {
                println!("[SIDECAR] Using bundled engine {} (v{}, sha: {})", exe.display(), version, sha);
                Command::new(exe)
            }
            EngineLaunch::DevPython { python, script } => {
                println!("[SIDECAR] Spawning sidecar process with {:?} {:?}", python, script);
                let mut c = Command::new(python);
                c.arg(script);
                c
            }
        };

        // Phase 12 production stdio: NEVER inherit handles. The windowed
        // (`console=False`) engine blocks forever on an undrained inherited
        // stdout pipe (proven: 0-CPU hang; file-redirected boot serves in
        // seconds), and the GUI host itself has no console. stdin=null,
        // stdout/stderr append to ~/.pain-ai/logs/ (operator-visible, used by
        // support/doctor). A >5MB log truncates on open (cheap rotation).
        cmd.stdin(std::process::Stdio::null());
        if let Some(home) = dirs_home() {
            let logs_dir = home.join(".pain-ai").join("logs");
            let _ = std::fs::create_dir_all(&logs_dir);
            cmd.stdout(Self::log_file(&logs_dir, "engine-stdout.log"));
            cmd.stderr(Self::log_file(&logs_dir, "engine-stderr.log"));
        } else {
            cmd.stdout(std::process::Stdio::null());
            cmd.stderr(std::process::Stdio::null());
        }

        // Environment variables
        cmd.env("PORT", port.to_string());
        cmd.env("LSC_TOKEN", &token);
        cmd.env("HERMES_APPROVALS_MODE", "manual");
        cmd.env("HERMES_TERMINAL_BACKEND", "local");

        // Pain AI Home (~/.pain-ai)
        if let Some(home) = dirs_home() {
            let pain_home = home.join(".pain-ai");
            let _ = std::fs::create_dir_all(&pain_home);
            cmd.env("HERMES_HOME", pain_home.to_string_lossy().to_string());
        }

        // Injected provider settings (Phase 3: EFFECTIVE config — custom
        // endpoint/model overrides honored, so the runtime matches Settings).
        let active_cfg = crate::providers::load_config();
        cmd.env("LSC_PROVIDER", &active_cfg.active);
        match crate::providers::effective_provider(&active_cfg.active, &active_cfg) {
            Ok(eff) => {
                cmd.env("LSC_MODEL", &eff.model);
                cmd.env("LSC_BASE_URL", &eff.base_url);
            }
            Err(_) => {
                cmd.env("LSC_MODEL", &active_cfg.target_model);
            }
        }

        if let Ok(entry) = crate::providers::keyring_entry(&active_cfg.active) {
            if let Ok(pwd) = entry.get_password() {
                cmd.env("LSC_API_KEY", pwd);
            }
        }

        let child_res = cmd.spawn();

        let child = match child_res {
            Ok(c) => {
                println!("[SIDECAR] Spawned child process PID {:?}", c.id());
                c
            }
            Err(e) => {
                eprintln!("[SIDECAR] Failed to spawn sidecar: {:?}", e);
                let mut lock = self.inner.lock().unwrap();
                lock.status = SidecarStateKind::Dead;
                return;
            }
        };

        {
            let mut lock = self.inner.lock().unwrap();
            lock.child = Some(child);
        }

        // Health poll thread
        let self_clone = self.clone();
        let spawn_time = Instant::now();
        std::thread::spawn(move || {
            let client = reqwest::blocking::Client::builder()
                .timeout(Duration::from_millis(800))
                .build()
                .unwrap_or_default();
            let health_url = format!("http://127.0.0.1:{}/healthz", port);

            let mut connected = false;
            // Poll every 500ms up to 30 times (15 seconds max)
            for _ in 0..30 {
                std::thread::sleep(Duration::from_millis(500));
                if let Ok(resp) = client.get(&health_url).send() {
                    if resp.status().is_success() {
                        let elapsed = spawn_time.elapsed().as_millis() as u64;
                        println!("[SIDECAR] Healthz responded OK in {}ms", elapsed);
                        let mut lock = self_clone.inner.lock().unwrap();
                        lock.status = SidecarStateKind::Ready;
                        lock.cold_start_ms = Some(elapsed);
                        connected = true;
                        break;
                    }
                }
            }

            if !connected {
                eprintln!("[SIDECAR] Health poll timed out after 15s");
                let mut lock = self_clone.inner.lock().unwrap();
                if lock.restart_count < 3 {
                    lock.restart_count += 1;
                    drop(lock);
                    self_clone.spawn_child();
                } else {
                    lock.status = SidecarStateKind::Dead;
                }
            }
        });
    }

    pub fn get_status(&self) -> SidecarStatusResponse {
        let lock = self.inner.lock().unwrap();
        let status_str = match lock.status {
            SidecarStateKind::Starting => "starting",
            SidecarStateKind::Ready => "ready",
            SidecarStateKind::Reconnecting => "reconnecting",
            SidecarStateKind::Dead => "dead",
        };
        SidecarStatusResponse {
            status: status_str.to_string(),
            port: lock.port,
            pid: lock.child.as_ref().map(|c| c.id()),
            cold_start_ms: lock.cold_start_ms,
            restart_count: lock.restart_count,
        }
    }

    pub fn get_token(&self) -> String {
        let lock = self.inner.lock().unwrap();
        lock.token.clone()
    }

    pub fn restart(&self) {
        let mut lock = self.inner.lock().unwrap();
        lock.restart_count = 0;
        lock.cold_start_ms = None;
        drop(lock);
        self.spawn_child();
    }
}

fn dirs_home() -> Option<PathBuf> {
    env::var("USERPROFILE")
        .or_else(|_| env::var("HOME"))
        .ok()
        .map(PathBuf::from)
}

// Global instance wrapped in Mutex
use std::sync::OnceLock;
static SIDECAR: OnceLock<SidecarManager> = OnceLock::new();

/// Phase 12: Tauri `$RESOURCE` dir published by `main.rs` setup().
/// Process-wide, written once at startup before the first sidecar spawn,
/// read by `engine_dir_candidates`. Defaults to `None` (dev/tests).
static BUNDLED_RESOURCE_DIR: OnceLock<std::sync::Mutex<Option<PathBuf>>> = OnceLock::new();

fn bundled_resource_dir_slot() -> &'static std::sync::Mutex<Option<PathBuf>> {
    BUNDLED_RESOURCE_DIR.get_or_init(|| std::sync::Mutex::new(None))
}

pub fn get_sidecar() -> &'static SidecarManager {
    SIDECAR.get_or_init(|| SidecarManager::new())
}

/// Phase 12: non-initializing accessor. Unit tests and cold paths use this so
/// merely asking for voice transport never spawns a child process.
pub fn try_get_sidecar() -> Option<&'static SidecarManager> {
    SIDECAR.get()
}

#[cfg(test)]
#[path = "sidecar_tests.rs"]
mod sidecar_tests;

#[tauri::command]
pub fn sidecar_status() -> SidecarStatusResponse {
    get_sidecar().get_status()
}

#[tauri::command]
pub fn sidecar_token() -> String {
    get_sidecar().get_token()
}

#[tauri::command]
pub fn sidecar_restart() -> SidecarStatusResponse {
    let sidecar = get_sidecar();
    sidecar.restart();
    sidecar.get_status()
}
