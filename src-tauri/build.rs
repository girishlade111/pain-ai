use std::env;
use std::path::Path;

fn main() {
    let target = env::var("TARGET").unwrap_or_else(|_| "x86_64-pc-windows-msvc".to_string());
    let ext = if target.contains("windows") { ".exe" } else { "" };
    let exe_name = format!("engine{}", ext);
    // Canonical PyInstaller one-dir staging (bundled as $RESOURCE/engine/
    // via tauri.conf.json `"binaries/engine/": "engine"`). Same path on
    // Windows and Linux: the engine is always built on the host platform.
    let one_dir_exe = Path::new("binaries").join("engine").join(&exe_name);
    // Legacy triple staging from earlier P12 drafts (accepted for dev, but
    // NOT bundled — re-stage to the canonical dir before `tauri build`).
    let legacy_dir_exe = Path::new("binaries")
        .join(format!("engine-{}", target))
        .join(&exe_name);
    let legacy_file = Path::new("binaries").join(format!("engine-{}{}", target, ext));

    // Fail-closed release gates. The bundler itself refuses a missing
    // `binaries/engine/` under the release overlay (`tauri.release.conf.json`
    // maps it to $RESOURCE/engine/), but a half-staged dir (present without
    // the exe) would bundle silently — always an error state, even for dev
    // (dev needs no staging dir at all). LSC_REQUIRE_ENGINE=1 (set by CI for
    // `npm run tauri:release`) additionally forbids the stub/dev fallbacks.
    let require_engine = env::var("LSC_REQUIRE_ENGINE")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    let one_dir = Path::new("binaries").join("engine");

    if one_dir.exists() && !one_dir_exe.exists() {
        panic!(
            "\n\n=========================================================================\n\
             [P12 BUILD ERROR] Corrupt engine staging: src-tauri/binaries/engine/ exists\n\
             but {} is missing (interrupted PyInstaller run?).\n\
             Rebuild: `pyinstaller sidecar/engine.spec --distpath src-tauri/binaries --name engine`\n\
             or delete the directory for dev-only builds.\n\
             =========================================================================\n\n",
            exe_name
        );
    }

    if one_dir_exe.exists() {
        println!("cargo:rerun-if-changed=binaries");
        println!(
            "cargo:warning=[P12] Verified packaged sidecar one-dir at src-tauri/binaries/engine/ (bundled as $RESOURCE/engine/)"
        );
    } else if require_engine {
        panic!(
            "\n\n=========================================================================\n\
             [P12 RELEASE ERROR] LSC_REQUIRE_ENGINE=1 but no packaged engine staged.\n\
             Stage it first: `pyinstaller sidecar/engine.spec --distpath src-tauri/binaries --name engine`\n\
             (canonical output src-tauri/binaries/engine/; keep workpath OUTSIDE OneDrive trees)\n\
             then `npm run tauri:release`. The installer must never ship the dev stub.\n\
             =========================================================================\n\n"
        );
    } else if legacy_dir_exe.exists() {
        println!("cargo:rerun-if-changed=binaries");
        println!(
            "cargo:warning=[P12] Legacy triple one-dir at src-tauri/binaries/engine-{}/ is NOT bundled. Re-stage canonical: `pyinstaller sidecar/engine.spec --distpath src-tauri/binaries --name engine`",
            target
        );
    } else if legacy_file.exists() {
        // Dev tolerance: the tracked stub (or a legacy one-file build) keeps
        // `cargo check/test` green. The Rust host never executes it without
        // passing --lsc-version validation (see sidecar::find_bundled_engine).
        // NOTE: `tauri build` will fail at the bundle step until the real
        // one-dir is staged — the installer must never ship the stub.
        println!("cargo:rerun-if-changed=binaries");
        println!(
            "cargo:warning=[P12] No packaged one-dir engine; dev fallback applies. Stage it with: `pyinstaller sidecar/engine.spec --distpath src-tauri/binaries --name engine` (keep workpath OUTSIDE OneDrive trees)"
        );
    } else {
        panic!(
            "\n\n=========================================================================\n\
             [P12 BUILD ERROR] Missing sidecar engine.\n\
             Expected canonical one-dir: src-tauri/binaries/engine/{} (from `pyinstaller sidecar/engine.spec --distpath src-tauri/binaries --name engine`)\n\
             or dev fallback: src-tauri/binaries/engine-{}{}\n\
             =========================================================================\n\n",
            exe_name, target, ext
        );
    }
    println!(
        "cargo:warning=[P12] FFmpeg note: System ffmpeg detected/used for video processing; fallback gracefully logs warning if missing."
    );

    tauri_build::build();
}
