include!("../build-resources.rs");

fn main() {
    embed_resources(
        "wakeshot.exe.manifest",
        "Wakeshot command line",
        "wakeshot.exe",
    );
}
