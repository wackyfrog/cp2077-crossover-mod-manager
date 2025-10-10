fn main() {
    // Add custom build configuration for macOS to register NXM protocol
    #[cfg(target_os = "macos")]
    {
        println!("cargo:rustc-env=MACOSX_DEPLOYMENT_TARGET=10.15");

        // Add URL scheme configuration for macOS
        println!("cargo:rustc-env=TAURI_CONFIG_URL_SCHEMES=nxm");
    }

    tauri_build::build()
}
