include!("../build-resources.rs");

fn main() {
    embed_resources(
        "wakeshot-tray.exe.manifest",
        "Wakeshot",
        "wakeshot-tray.exe",
    );
}
