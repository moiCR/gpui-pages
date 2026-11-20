use std::{env, error::Error, fs, path::PathBuf};

fn main() -> Result<(), Box<dyn Error>> {
    println!("cargo:rerun-if-changed=src/pages");
    let crate_root = env::var_os("CARGO_MANIFEST_DIR").ok_or("missing CARGO_MANIFEST_DIR")?;
    let out_dir = env::var_os("OUT_DIR").ok_or("missing OUT_DIR")?;
    let output = PathBuf::from(out_dir).join("pages.rs");
    let generated = pages_codegen::generate(crate_root).map_err(|error| error.to_string())?;
    fs::write(&output, generated)
        .map_err(|error| format!("cannot write {}: {error}", output.display()))?;
    Ok(())
}
