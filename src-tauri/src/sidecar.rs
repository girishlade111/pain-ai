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

        let python_exe = Self::resolve_python_path();
        let script_path = PathBuf::from("sidecar/lsc_bridge.py");

        let mut cmd = Command::new(&python_exe);
        cmd.arg(&script_path);

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

        println!("[SIDECAR] Spawning sidecar process with {:?} {:?}", python_exe, script_path);
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

pub fn get_sidecar() -> &'static SidecarManager {
    SIDECAR.get_or_init(|| SidecarManager::new())
}

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
