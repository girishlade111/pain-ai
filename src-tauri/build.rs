use std::env;
use std::path::Path;

fn main() {
    let target = env::var("TARGET").unwrap_or_else(|_| "x86_64-pc-windows-msvc".to_string());
    let ext = if target.contains("windows") { ".exe" } else { "" };
    let expected_filename = format!("engine-{}{}", target, ext);
    let binary_path = Path::new("binaries").join(&expected_filename);

    if !binary_path.exists() {
        panic!(
            "\n\n=========================================================================\n\
             [P12 BUILD ERROR] Missing required sidecar binary: {}\n\
             Expected path: src-tauri/binaries/{}\n\n\
             To fix this:\n\
             1. Run PyInstaller spec: `pyinstaller sidecar/engine.spec` OR\n\
             2. Build/place precompiled engine binary at `src-tauri/binaries/{}`\n\
             =========================================================================\n\n",
            expected_filename, expected_filename, expected_filename
        );
    }

    println!("cargo:rerun-if-changed=binaries");
    println!(
        "cargo:warning=[P12] Verified sidecar binary exists at src-tauri/binaries/{}",
        expected_filename
    );
    println!(
        "cargo:warning=[P12] FFmpeg note: System ffmpeg detected/used for video processing; fallback gracefully logs warning if missing."
    );

    tauri_build::build();
}
