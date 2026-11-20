use super::{DiscoveryError, RouteNode};
use std::{
    fs, io,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

static NEXT_FIXTURE: AtomicUsize = AtomicUsize::new(0);

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
    let users = fixture.add("dashboard/users.rs")?;
    let settings = fixture.add("settings.rs")?;
    let root_page = fixture.add("mod.rs")?;
    let dashboard_page = fixture.add("dashboard/mod.rs")?;
    let analytics = fixture.add("dashboard/analytics.rs")?;
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
    fixture.add("dashboard/analytics.rs")?;
    fixture.add("dashboard/reports/monthly.rs")?;
    fixture.add("dashboard/users.rs")?;
    fixture.add("settings.rs")?;

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
fn reports_both_sources_of_a_duplicate_route() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = Fixture::new()?;
    let first = fixture.add("dashboard/mod.rs")?;
    let second = fixture.add("dashboard.rs")?;
    assert!(
        matches!(RouteNode::discover(&fixture.0), Err(DiscoveryError::Conflict {
            route, first: actual_first, second: actual_second,
        }) if route == "/dashboard" && actual_first == first && actual_second == second)
    );
    Ok(())
}

#[test]
fn reports_the_path_of_an_invalid_page_name() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = Fixture::new()?;
    let invalid = fixture.add("bad-name.rs")?;
    assert!(
        matches!(RouteNode::discover(&fixture.0), Err(DiscoveryError::InvalidName(path)) if path == invalid)
    );
    Ok(())
}
