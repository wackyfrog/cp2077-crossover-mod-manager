fn main() {
    // Always re-run build.rs so timestamp updates every build
    println!("cargo:rerun-if-changed=NONEXISTENT_FILE");

    // Build timestamp for identifying builds
    let now = chrono::Local::now();
    println!("cargo:rustc-env=BUILD_TIMESTAMP={}", now.format("%m%d·%H%M"));

    // Add custom build configuration for macOS to register NXM protocol
    #[cfg(target_os = "macos")]
    {
        println!("cargo:rustc-env=MACOSX_DEPLOYMENT_TARGET=10.15");

        // Add URL scheme configuration for macOS
        println!("cargo:rustc-env=TAURI_CONFIG_URL_SCHEMES=nxm");
    }

    tauri_build::build()
}
