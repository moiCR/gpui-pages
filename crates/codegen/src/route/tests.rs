use super::{DiscoveryError, RouteNode};
use crate::{GenerateError, generate};
use std::{
    fs, io,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

static NEXT_FIXTURE: AtomicUsize = AtomicUsize::new(0);

#[test]
fn discovers_dynamic_directories_and_inherits_their_layouts()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = Fixture::new()?;
    let page = fixture.add("users/_id/posts/_post/mod.rs")?;
    let layout = fixture.add("users/_id/layout.rs")?;
    let root = RouteNode::discover(&fixture.0)?;
    let table = root.route_table();
    assert_eq!(table.len(), 1);
    assert_eq!(table[0].page.path, "/users/:id/posts/:post");
    assert_eq!(table[0].page.source, page);
    assert_eq!(table[0].layouts[0].source, layout);
    Ok(())
}

#[test]
fn reports_ambiguous_and_repeated_parameters() -> Result<(), Box<dyn std::error::Error>> {
    let ambiguous = Fixture::new()?;
    let first = ambiguous.add("users/_id/mod.rs")?;
    let second = ambiguous.add("users/_name/mod.rs")?;
    assert!(matches!(RouteNode::discover(&ambiguous.0),
        Err(DiscoveryError::AmbiguousParameters { first: left, second: right })
            if Some(left.as_path()) == first.parent() && Some(right.as_path()) == second.parent()));

    let repeated = Fixture::new()?;
    let page = repeated.add("users/_id/posts/_id/mod.rs")?;
    assert!(matches!(RouteNode::discover(&repeated.0),
        Err(DiscoveryError::RepeatedParameter { path, parameter })
            if Some(path.as_path()) == page.parent() && parameter == "id"));
    Ok(())
}

#[test]
fn generates_reproducible_rust() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = Fixture::new()?;
    fs::write(
        fixture.add("mod.rs")?,
        "pub struct Home; impl gpui_pages::Page for Home {}",
    )?;
    fs::write(
        fixture.add("type/mod.rs")?,
        "pub(crate) struct ArbitraryName; impl Page for ArbitraryName {}",
    )?;
    let generated = generate(&fixture.0)?;
    assert_eq!(generated, generate(&fixture.0)?);
    assert!(generated.contains("pub mod r#type {"));
    assert!(generated.contains("r#pages::r#type::ArbitraryName"));
    Ok(())
}

#[test]
fn rejects_missing_ambiguous_and_unsupported_page_implementations()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = Fixture::new()?;
    let page = fixture.add("mod.rs")?;
    for (source, message) in [
        ("pub struct Home;", "expected one top-level"),
        (
            "pub struct A; pub struct B; impl Page for A {} impl Page for B {}",
            "multiple",
        ),
        ("struct Home; impl Page for Home {}", "non-generic pub"),
        (
            "pub struct Home<T>(T); impl<T> Page for Home<T> {}",
            "non-generic pub",
        ),
    ] {
        fs::write(&page, source)?;
        assert!(
            matches!(generate(&fixture.0), Err(GenerateError::InvalidComponent { path, reason })
            if path == page && reason.contains(message))
        );
    }
    Ok(())
}

#[test]
fn rejects_index_module_collisions() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = Fixture::new()?;
    let index = fixture.add("mod.rs")?;
    fixture.add("__gpui_pages_index/mod.rs")?;
    assert!(
        matches!(generate(&fixture.0), Err(GenerateError::IndexCollision(path)) if path == index)
    );
    Ok(())
}

#[test]
fn rejects_collisions_between_page_types_and_child_modules()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = Fixture::new()?;
    let page = fixture.add("mod.rs")?;
    fs::write(&page, "pub struct Dashboard; impl Page for Dashboard {}")?;
    fixture.add("Dashboard/mod.rs")?;
    assert!(
        matches!(generate(&fixture.0), Err(GenerateError::InvalidComponent { path, reason })
        if path == page && reason.contains("conflicts with a generated module"))
    );
    Ok(())
}

#[test]
fn generates_updated_routes_as_directories_and_layouts_change()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = Fixture::new()?;
    fs::write(
        fixture.add("mod.rs")?,
        "pub struct Home; impl Page for Home {}",
    )?;
    assert!(!generate(&fixture.0)?.contains("path: \"/account\""));

    let page = fixture.add("account/mod.rs")?;
    fs::write(&page, "pub struct Account; impl Page for Account {}")?;
    let layout = fixture.add("account/layout.rs")?;
    fs::write(&layout, "pub struct Shell; impl PageLayout for Shell {}")?;
    let mut watched = Vec::new();
    let generated = crate::generate::generate_tracking(&fixture.0, |path| {
        watched.push(path.to_owned());
    })?;
    assert!(watched.contains(&page));
    assert!(watched.contains(&layout));
    assert!(generated.modules.contains("#[path = \"pages/account\"]"));
    assert!(generated.modules.contains("#[path = \"mod.rs\"]"));
    assert!(
        !generated
            .modules
            .contains(&fixture.0.to_string_lossy().into_owned())
    );
    assert!(!generated.registry.contains("pub mod"));
    assert!(generated.registry.contains("path: \"/account\""));
    assert!(generated.registry.contains("pages::account::layout"));

    fs::remove_file(layout)?;
    assert!(!generate(&fixture.0)?.contains("pages::account::layout"));
    let pages = fixture.0.join("src/pages");
    fs::rename(pages.join("account"), pages.join("profile"))?;
    let generated = generate(&fixture.0)?;
    assert!(!generated.contains("path: \"/account\""));
    assert!(generated.contains("path: \"/profile\""));
    fs::remove_file(pages.join("profile/mod.rs"))?;
    fs::remove_dir(pages.join("profile"))?;
    assert!(!generate(&fixture.0)?.contains("path: \"/profile\""));
    Ok(())
}

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let sequence = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "pages-codegen-{}-{timestamp}-{sequence}",
            std::process::id()
        ));
        fs::create_dir(&root)?;
        Ok(Self(root))
    }

    fn add(&self, name: &str) -> io::Result<PathBuf> {
        let path = self.0.join("src/pages").join(name);
        fs::create_dir_all(path.parent().ok_or(io::ErrorKind::InvalidInput)?)?;
        fs::write(&path, "")?;
        Ok(path)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn discovers_pages_and_nested_layouts_in_sorted_order() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = Fixture::new()?;
    let users = fixture.add("dashboard/users/mod.rs")?;
    let settings = fixture.add("settings/mod.rs")?;
    let root_page = fixture.add("mod.rs")?;
    let dashboard_page = fixture.add("dashboard/mod.rs")?;
    let analytics = fixture.add("dashboard/analytics/mod.rs")?;
    let root_layout = fixture.add("layout.rs")?;
    let dashboard_layout = fixture.add("dashboard/layout.rs")?;
    fixture.add("notes.txt")?;

    let root = RouteNode::discover(&fixture.0)?;
    let dashboard = &root.children["dashboard"];
    assert_eq!(
        root.children.keys().collect::<Vec<_>>(),
        ["dashboard", "settings"]
    );
    assert_eq!(
        dashboard.children.keys().collect::<Vec<_>>(),
        ["analytics", "users"]
    );
    for (node, route, source) in [
        (&root, "/", root_page),
        (&root.children["settings"], "/settings", settings),
        (dashboard, "/dashboard", dashboard_page),
        (
            &dashboard.children["analytics"],
            "/dashboard/analytics",
            analytics,
        ),
        (&dashboard.children["users"], "/dashboard/users", users),
    ] {
        assert_eq!(
            node.page
                .as_ref()
                .map(|page| (page.path.as_str(), &page.source)),
            Some((route, &source))
        );
    }
    assert_eq!(
        root.layout.as_ref().map(|layout| &layout.source),
        Some(&root_layout)
    );
    assert_eq!(
        dashboard.layout.as_ref().map(|layout| &layout.source),
        Some(&dashboard_layout)
    );
    Ok(())
}

#[test]
fn resolves_layouts_for_each_page_without_leaking_across_branches()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = Fixture::new()?;
    let root_layout = fixture.add("layout.rs")?;
    let dashboard_layout = fixture.add("dashboard/layout.rs")?;
    let reports_layout = fixture.add("dashboard/reports/layout.rs")?;
    fixture.add("mod.rs")?;
    fixture.add("dashboard/mod.rs")?;
    fixture.add("dashboard/analytics/mod.rs")?;
    fixture.add("dashboard/reports/monthly/mod.rs")?;
    fixture.add("dashboard/users/mod.rs")?;
    fixture.add("settings/mod.rs")?;

    let root = RouteNode::discover(&fixture.0)?;
    let table = root.route_table();
    let routes: Vec<_> = table
        .iter()
        .map(|entry| {
            (
                entry.page.path.as_str(),
                entry
                    .layouts
                    .iter()
                    .map(|layout| &layout.source)
                    .collect::<Vec<_>>(),
            )
        })
        .collect();
    assert_eq!(
        routes,
        [
            ("/", vec![&root_layout]),
            ("/dashboard", vec![&root_layout, &dashboard_layout]),
            (
                "/dashboard/analytics",
                vec![&root_layout, &dashboard_layout]
            ),
            (
                "/dashboard/reports/monthly",
                vec![&root_layout, &dashboard_layout, &reports_layout]
            ),
            ("/dashboard/users", vec![&root_layout, &dashboard_layout]),
            ("/settings", vec![&root_layout]),
        ]
    );
    Ok(())
}

#[test]
fn ignores_standalone_pages_and_auxiliary_files() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = Fixture::new()?;
    let dashboard = fixture.add("dashboard/mod.rs")?;
    fs::write(
        &dashboard,
        "pub struct Dashboard; impl Page for Dashboard {}",
    )?;
    fixture.add("dashboard.rs")?;
    fixture.add("dashboard/components.rs")?;
    fixture.add("dashboard/helpers.rs")?;
    fixture.add("dashboard/state.rs")?;
    fixture.add("standalone.rs")?;
    let root = RouteNode::discover(&fixture.0)?;
    let table = root.route_table();
    assert_eq!(table.len(), 1);
    assert_eq!(table[0].page.path, "/dashboard");
    assert_eq!(table[0].page.source, dashboard);
    assert!(!generate(&fixture.0)?.contains("standalone.rs"));
    Ok(())
}

#[test]
fn reports_the_path_of_an_invalid_page_name() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = Fixture::new()?;
    let invalid = fixture.add("bad-name/mod.rs")?;
    assert!(
        matches!(RouteNode::discover(&fixture.0), Err(DiscoveryError::InvalidName(path)) if Some(path.as_path()) == invalid.parent())
    );
    Ok(())
}
