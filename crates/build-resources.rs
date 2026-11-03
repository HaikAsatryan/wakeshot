// The Windows resources of both exes: manifest, app icon and version info. Pulled into each `build.rs` with
// `include!`, so the two exes never drift apart and no build crate is needed. It sits as a plain file in `crates/`
// because a folder there without a `Cargo.toml` breaks the workspace's `crates/*` member glob.
//
// Owner: contracts card (the icon file itself is the tray track's, `p5-icon-art`).

use std::{env, fs, path::Path, path::PathBuf};

/// Embeds `manifest` (in the crate folder), the app icon and a version resource that always matches `Cargo.toml`.
fn embed_resources(manifest: &str, description: &str, file_name: &str) {
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    let root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let icon = root.join("..").join("..").join("assets").join("wakeshot.ico");

    let mut rc = format!("1 24 \"{}\"\n", rc_path(&root.join(manifest)));
    rc += &format!("1 ICON \"{}\"\n", rc_path(&icon));
    rc += &version_info(description, file_name);

    let rc_file = out.join("app.rc");
    fs::write(&rc_file, rc).unwrap();
    // Required: without the manifest's supportedOS, Windows gives the exe the compatibility behavior of an older
    // Windows; the tray also needs its per-monitor v2 DPI setting.
    embed_resource::compile(&rc_file, embed_resource::NONE)
        .manifest_required()
        .unwrap();

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=../build-resources.rs");
    println!("cargo:rerun-if-changed={manifest}");
    println!("cargo:rerun-if-changed={}", icon.display());
}

fn rc_path(p: &Path) -> String {
    p.display().to_string().replace('\\', "\\\\")
}

fn version_info(description: &str, file_name: &str) -> String {
    let part = |name: &str| env::var(name).unwrap();
    let (major, minor, patch) = (
        part("CARGO_PKG_VERSION_MAJOR"),
        part("CARGO_PKG_VERSION_MINOR"),
        part("CARGO_PKG_VERSION_PATCH"),
    );
    let version = part("CARGO_PKG_VERSION");
    format!(
        r#"
1 VERSIONINFO
FILEVERSION {major},{minor},{patch},0
PRODUCTVERSION {major},{minor},{patch},0
FILEOS 0x40004
FILETYPE 0x1
BEGIN
  BLOCK "StringFileInfo"
  BEGIN
    BLOCK "040904b0"
    BEGIN
      VALUE "CompanyName", "Haik Asatryan"
      VALUE "FileDescription", "{description}"
      VALUE "FileVersion", "{version}"
      VALUE "OriginalFilename", "{file_name}"
      VALUE "ProductName", "Wakeshot"
      VALUE "ProductVersion", "{version}"
    END
  END
  BLOCK "VarFileInfo"
  BEGIN
    VALUE "Translation", 0x409, 1200
  END
END
"#
    )
}
