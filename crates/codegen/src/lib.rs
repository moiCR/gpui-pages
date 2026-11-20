mod generate;
pub mod layout;
pub mod route;

pub use generate::{GenerateError, generate};

const VIEWS_DIR: &str = "src/pages";

pub fn build() -> Result<(), Box<dyn std::error::Error>> {
    use std::{env, fs, path::PathBuf};
    println!("cargo:rerun-if-changed={VIEWS_DIR}");
    let crate_root = env::var_os("CARGO_MANIFEST_DIR").ok_or("missing CARGO_MANIFEST_DIR")?;
    let out_dir = env::var_os("OUT_DIR").ok_or("missing OUT_DIR")?;
    let output = PathBuf::from(out_dir).join("pages.rs");
    let generated = generate::generate_tracking(crate_root, |source| {
        println!("cargo:rerun-if-changed={}", source.display());
    })
    .map_err(|error| error.to_string())?;
    fs::write(&output, generated)
        .map_err(|error| format!("cannot write {}: {error}", output.display()))?;
    Ok(())
}
