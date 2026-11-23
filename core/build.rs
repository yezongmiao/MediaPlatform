use std::path::Path;

fn visit(dir: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    println!("cargo:rerun-if-changed={}", dir.display());
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            visit(&path);
        } else {
            println!("cargo:rerun-if-changed={}", path.display());
        }
    }
}

fn main() {
    // The UI is embedded with include_dir!. Cargo does not reliably track files
    // inside that directory, so explicitly register every dist file. Without
    // this, a server rebuilt after only a frontend build can keep a stale UI.
    let dist = Path::new(env!("CARGO_MANIFEST_DIR")).join("../dist");
    visit(&dist);
}
