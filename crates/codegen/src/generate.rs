use crate::route::{DiscoveryError, RouteNode};
use std::{
    collections::BTreeMap,
    error::Error,
    fmt,
    path::{Path, PathBuf},
};

const INDEX: &str = "__gpui_pages_index";

#[derive(Debug)]
pub enum GenerateError {
    Discover(DiscoveryError),
    CrateRoot(std::io::Error),
    NonUtf8Path(PathBuf),
    IndexCollision(PathBuf),
    MissingModule(PathBuf),
}

impl fmt::Display for GenerateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Discover(error) => error.fmt(f),
            Self::CrateRoot(error) => write!(f, "cannot resolve the crate root: {error}"),
            Self::NonUtf8Path(path) => write!(f, "cannot emit a Rust path for {}", path.display()),
            Self::MissingModule(path) => write!(f, "no generated module for {}", path.display()),
            Self::IndexCollision(path) => write!(
                f,
                "{} conflicts with the reserved module name {INDEX}",
                path.display()
            ),
        }
    }
}

impl Error for GenerateError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Discover(error) => Some(error),
            Self::CrateRoot(error) => Some(error),
            _ => None,
        }
    }
}

pub fn generate(crate_root: impl AsRef<Path>) -> Result<String, GenerateError> {
    let absolute_root = std::path::absolute(crate_root).map_err(GenerateError::CrateRoot)?;
    let root = RouteNode::discover(absolute_root).map_err(GenerateError::Discover)?;
    let mut modules = BTreeMap::new();
    let mut output = String::from("pub mod pages {\n");
    emit_node(&root, "pages", &mut modules, &mut output)?;
    output.push_str("}\n\npub struct GeneratedRoute {\n    pub path: &'static str,\n    pub page: &'static str,\n    pub layouts: &'static [&'static str],\n}\n\npub const ROUTES: &[GeneratedRoute] = &[\n");

    for entry in root.route_table() {
        let page = module_for(&modules, &entry.page.source)?;
        output.push_str(&format!(
            "    GeneratedRoute {{ path: {:?}, page: {page:?}, layouts: &[",
            entry.page.path
        ));
        for layout in entry.layouts {
            let layout = module_for(&modules, &layout.source)?;
            output.push_str(&format!("{layout:?},"));
        }
        output.push_str("] },\n");
    }
    output.push_str("];\n\npub const PAGE_ROUTES: &[::gpui_pages::Route] = &[\n");
    for entry in root.route_table() {
        let page = rust_module(module_for(&modules, &entry.page.source)?);
        output.push_str(&format!(
            "    ::gpui_pages::Route {{ path: {:?}, mount: {page}::mount, layouts: &[",
            entry.page.path
        ));
        for layout in entry.layouts {
            let id = module_for(&modules, &layout.source)?;
            let module = rust_module(id);
            output.push_str(&format!(
                "::gpui_pages::Layout {{ id: {id:?}, mount: {module}::mount }},"
            ));
        }
        output.push_str("] },\n");
    }
    output.push_str("];\n");
    Ok(output)
}

fn rust_module(module: &str) -> String {
    module
        .split("::")
        .map(|segment| format!("r#{segment}"))
        .collect::<Vec<_>>()
        .join("::")
}

fn module_for<'a>(
    modules: &'a BTreeMap<&Path, String>,
    source: &Path,
) -> Result<&'a str, GenerateError> {
    modules
        .get(source)
        .map(String::as_str)
        .ok_or_else(|| GenerateError::MissingModule(source.to_owned()))
}

fn emit_node<'a>(
    node: &'a RouteNode,
    module: &str,
    modules: &mut BTreeMap<&'a Path, String>,
    output: &mut String,
) -> Result<(), GenerateError> {
    if let Some(page) = &node.page {
        if node.children.contains_key(INDEX) {
            return Err(GenerateError::IndexCollision(page.source.clone()));
        }
        emit_file(INDEX, &page.source, module, modules, output)?;
    }
    if let Some(layout) = &node.layout {
        emit_file("layout", &layout.source, module, modules, output)?;
    }
    for (segment, child) in &node.children {
        if let Some(page) = &child.page
            && child.children.is_empty()
            && child.layout.is_none()
            && page.source.file_name().is_some_and(|name| name != "mod.rs")
        {
            emit_file(segment, &page.source, module, modules, output)?;
        } else {
            output.push_str(&format!("pub mod r#{segment} {{\n"));
            emit_node(child, &format!("{module}::{segment}"), modules, output)?;
            output.push_str("}\n");
        }
    }
    Ok(())
}

fn emit_file<'a>(
    name: &str,
    source: &'a Path,
    parent: &str,
    modules: &mut BTreeMap<&'a Path, String>,
    output: &mut String,
) -> Result<(), GenerateError> {
    let path = source
        .to_str()
        .ok_or_else(|| GenerateError::NonUtf8Path(source.to_owned()))?;
    output.push_str(&format!("#[path = {path:?}]\npub mod r#{name};\n"));
    modules.insert(source, format!("{parent}::{name}"));
    Ok(())
}
