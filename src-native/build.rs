use std::{env, fs, path::PathBuf, process::Command};

fn main() {
    println!("cargo:rerun-if-changed=../src-tauri/icons/icon.ico");
    println!("cargo:rerun-if-changed=Cargo.toml");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    assert_eq!(
        env::var("CARGO_CFG_TARGET_ENV").as_deref(),
        Ok("msvc"),
        "Windows native packages require the MSVC Windows SDK resource compiler"
    );
    let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let icon = manifest
        .join("../src-tauri/icons/icon.ico")
        .canonicalize()
        .unwrap();
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let script = out.join("neon-hud.rc");
    let resource = out.join("neon-hud.res");
    // No SDK includes are required for an ICON resource.
    let icon_path = icon
        .to_str()
        .unwrap()
        .trim_start_matches(r"\\?\")
        .replace('\\', "/");
    fs::write(&script, format!("1 ICON \"{icon_path}\"\n")).unwrap();

    let mut compilers: Vec<PathBuf> = env::var_os("PATH")
        .map(|path| {
            env::split_paths(&path)
                .map(|dir| dir.join("rc.exe"))
                .collect()
        })
        .unwrap_or_default();
    if let Some(program_files) = env::var_os("ProgramFiles(x86)") {
        let sdk = PathBuf::from(program_files).join("Windows Kits/10/bin");
        let mut versions: Vec<_> = fs::read_dir(sdk)
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.is_dir())
            .collect();
        versions.sort();
        compilers.extend(versions.into_iter().rev().map(|dir| dir.join("x64/rc.exe")));
    }
    let compiler = compilers
        .into_iter()
        .find(|path| path.is_file())
        .expect("Install the Windows 10/11 SDK to embed the native program icon (rc.exe missing)");
    let result = Command::new(compiler)
        .arg("/nologo")
        .arg("/fo")
        .arg(&resource)
        .arg(&script)
        .status()
        .expect("Could not run the Windows SDK resource compiler");
    assert!(result.success(), "Could not compile the native app icon");
    println!(
        "cargo:rustc-link-arg-bin=neon-hud-native={}",
        resource.display()
    );
}
