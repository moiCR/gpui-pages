use crate::route::{DiscoveryError, RouteNode};
use std::{
    collections::BTreeMap,
    error::Error,
    fmt, fs,
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
    InvalidComponent { path: PathBuf, reason: String },
}

impl fmt::Display for GenerateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Discover(error) => error.fmt(f),
            Self::CrateRoot(error) => write!(f, "cannot resolve the crate root: {error}"),
            Self::NonUtf8Path(path) => write!(f, "cannot emit a Rust path for {}", path.display()),
            Self::MissingModule(path) => write!(f, "no generated module for {}", path.display()),
            Self::InvalidComponent { path, reason } => write!(f, "{}: {reason}", path.display()),
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
    generate_tracking(crate_root, |_| {})
}

pub(crate) fn generate_tracking(
    crate_root: impl AsRef<Path>,
    mut track_source: impl FnMut(&Path),
) -> Result<String, GenerateError> {
    let absolute_root = std::path::absolute(crate_root).map_err(GenerateError::CrateRoot)?;
    let root = RouteNode::discover(absolute_root).map_err(GenerateError::Discover)?;
    let mut modules = BTreeMap::new();
    let mut output = String::from("pub mod pages {\n");
    emit_node(&root, "pages", &mut modules, &mut output)?;
    for source in modules.keys() {
        track_source(source);
    }
    output.push_str("}\n\npub struct GeneratedRoute {\n    pub path: &'static str,\n    pub page: &'static str,\n    pub layouts: &'static [&'static str],\n}\n\npub const ROUTES: &[GeneratedRoute] = &[\n");

    let routes = root.route_table();
    for entry in &routes {
        let page = module_for(&modules, &entry.page.source)?;
        output.push_str(&format!(
            "    GeneratedRoute {{ path: {:?}, page: {page:?}, layouts: &[",
            entry.page.path
        ));
        for layout in &entry.layouts {
            let layout = module_for(&modules, &layout.source)?;
            output.push_str(&format!("{layout:?},"));
        }
        output.push_str("] },\n");
    }
    output.push_str("];\n\npub const PAGE_ROUTES: &[::gpui_pages::Route] = &[\n");
    for entry in &routes {
        let page = rust_module(module_for(&modules, &entry.page.source)?);
        let page_type = component_for(&modules, &entry.page.source)?;
        output.push_str(&format!(
            "    ::gpui_pages::Route {{ path: {:?}, mount: ::gpui_pages::mount_page::<{page}::{page_type}>, options: <{page}::{page_type} as ::gpui_pages::Page>::options, layouts: &[",
            entry.page.path
        ));
        for layout in &entry.layouts {
            let id = module_for(&modules, &layout.source)?;
            let is_root = root
                .layout
                .as_ref()
                .is_some_and(|root| root.source == layout.source);
            let module = rust_module(id);
            let layout_type = component_for(&modules, &layout.source)?;
            output.push_str(&format!(
                "::gpui_pages::Layout {{ id: {id:?}, is_root: {is_root}, mount: ::gpui_pages::mount_layout::<{module}::{layout_type}> }},"
            ));
        }
        output.push_str("] },\n");
    }
    output.push_str("];\n");
    for entry in modules.values() {
        let module = rust_module(&entry.name);
        let component = &entry.component;
        let trait_name = entry.trait_name;
        output.push_str(&format!(
            "\nimpl ::gpui_pages::gpui::Render for {module}::{component} {{\n    fn render(&mut self, window: &mut ::gpui_pages::gpui::Window, cx: &mut ::gpui_pages::gpui::Context<Self>) -> impl ::gpui_pages::gpui::IntoElement {{\n        <Self as ::gpui_pages::{trait_name}>::render(self, window, cx)\n    }}\n}}\n"
        ));
    }
    Ok(output)
}

fn component_type(path: &Path, trait_name: &str) -> Result<(String, &'static str), GenerateError> {
    use syn::{Item, PathArguments, Type, Visibility};
    let invalid = |reason| GenerateError::InvalidComponent {
        path: path.to_owned(),
        reason,
    };
    let source = fs::read_to_string(path).map_err(|error| invalid(error.to_string()))?;
    let file = syn::parse_file(&source).map_err(|error| invalid(error.to_string()))?;
    let mut implementations = file.items.iter().filter_map(|item| {
        let Item::Impl(implementation) = item else {
            return None;
        };
        let (_, trait_path, _) = implementation.trait_.as_ref()?;
        trait_path
            .segments
            .last()
            .filter(|segment| segment.ident == trait_name)?;
        Some(implementation)
    });
    let Some(implementation) = implementations.next() else {
        return Err(invalid(format!(
            "expected one top-level impl {trait_name} for a local struct"
        )));
    };
    if implementations.next().is_some() {
        return Err(invalid(format!("multiple impl {trait_name} declarations")));
    }
    if let Type::Path(target) = implementation.self_ty.as_ref()
        && target.qself.is_none()
        && target.path.leading_colon.is_none()
        && target.path.segments.len() == 1
        && let Some(segment) = target.path.segments.first()
        && matches!(segment.arguments, PathArguments::None)
        && implementation.generics.params.is_empty()
        && implementation.generics.where_clause.is_none()
        && let Some(Item::Struct(component)) = file.items.iter().find(
            |item| matches!(item, Item::Struct(component) if component.ident == segment.ident),
        )
        && component.generics.params.is_empty()
        && matches!(
            &component.vis,
            Visibility::Public(_) | Visibility::Restricted(_)
        )
        && !matches!(&component.vis, Visibility::Restricted(vis) if !vis.path.is_ident("crate"))
    {
        let visibility = if matches!(component.vis, Visibility::Public(_)) {
            "pub"
        } else {
            "pub(crate)"
        };
        return Ok((segment.ident.to_string(), visibility));
    }
    Err(invalid(format!(
        "impl {trait_name} must target a non-generic pub or pub(crate) struct declared in this file"
    )))
}

fn rust_module(module: &str) -> String {
    module
        .split("::")
        .map(|segment| format!("r#{segment}"))
        .collect::<Vec<_>>()
        .join("::")
}

struct Module {
    name: String,
    component: String,
    trait_name: &'static str,
}

fn module_for<'a>(
    modules: &'a BTreeMap<&Path, Module>,
    source: &Path,
) -> Result<&'a str, GenerateError> {
    modules
        .get(source)
        .map(|module| module.name.as_str())
        .ok_or_else(|| GenerateError::MissingModule(source.to_owned()))
}

fn component_for<'a>(
    modules: &'a BTreeMap<&Path, Module>,
    source: &Path,
) -> Result<&'a str, GenerateError> {
    modules
        .get(source)
        .map(|module| module.component.as_str())
        .ok_or_else(|| GenerateError::MissingModule(source.to_owned()))
}

fn emit_node<'a>(
    node: &'a RouteNode,
    module: &str,
    modules: &mut BTreeMap<&'a Path, Module>,
    output: &mut String,
) -> Result<(), GenerateError> {
    if let Some(page) = &node.page {
        if node.children.contains_key(INDEX) {
            return Err(GenerateError::IndexCollision(page.source.clone()));
        }
        emit_file(INDEX, &page.source, module, modules, output)?;
        let component = component_for(modules, &page.source)?;
        let name = component.trim_start_matches("r#");
        if name == INDEX
            || name == "layout" && node.layout.is_some()
            || node.children.contains_key(name)
        {
            return Err(GenerateError::InvalidComponent {
                path: page.source.clone(),
                reason: format!(
                    "page type {component} conflicts with a generated module in {module}"
                ),
            });
        }
    }
    if let Some(layout) = &node.layout {
        emit_file("layout", &layout.source, module, modules, output)?;
    }
    for (segment, child) in &node.children {
        output.push_str(&format!("pub mod r#{segment} {{\n"));
        emit_node(child, &format!("{module}::{segment}"), modules, output)?;
        output.push_str("}\n");
    }
    Ok(())
}

fn emit_file<'a>(
    name: &str,
    source: &'a Path,
    parent: &str,
    modules: &mut BTreeMap<&'a Path, Module>,
    output: &mut String,
) -> Result<(), GenerateError> {
    let path = source
        .to_str()
        .ok_or_else(|| GenerateError::NonUtf8Path(source.to_owned()))?;
    let is_page = name == INDEX;
    let trait_name = if is_page { "Page" } else { "PageLayout" };
    let (component, visibility) = component_type(source, trait_name)?;
    let module = if is_page {
        output.push_str(&format!(
            "#[path = {path:?}]\nmod {name};\n{visibility} use self::{name}::{component};\n"
        ));
        parent.to_owned()
    } else {
        output.push_str(&format!("#[path = {path:?}]\npub mod r#{name};\n"));
        format!("{parent}::{name}")
    };
    modules.insert(
        source,
        Module {
            name: module,
            component,
            trait_name,
        },
    );
    Ok(())
}
