use crate::layout::NodeLayout;
use std::{
    collections::BTreeMap,
    fmt, fs, io,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone)]
pub struct Route {
    pub path: String,
    pub source: PathBuf,
}

#[derive(Debug, Default)]
pub struct RouteNode {
    pub page: Option<Route>,
    pub layout: Option<NodeLayout>,
    pub children: BTreeMap<String, RouteNode>,
}

impl RouteNode {
    pub fn discover(crate_root: impl AsRef<Path>) -> Result<Self, DiscoveryError> {
        let mut root = Self::default();
        root.scan(&crate_root.as_ref().join("src/pages"), "/")?;
        Ok(root)
    }

    fn scan(&mut self, directory: &Path, route: &str) -> Result<(), DiscoveryError> {
        let directory_error = |source| DiscoveryError::Read {
            path: directory.to_owned(),
            source,
        };
        let mut entries = fs::read_dir(directory)
            .map_err(directory_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(directory_error)?;
        entries.sort_by_cached_key(fs::DirEntry::file_name);

        for entry in entries {
            let path = entry.path();
            let kind = entry.file_type().map_err(|source| DiscoveryError::Read {
                path: path.clone(),
                source,
            })?;

            if kind.is_dir() {
                let segment = route_segment(&path, path.file_name())?;
                let child_route = join_route(route, segment);
                self.children
                    .entry(segment.to_owned())
                    .or_default()
                    .scan(&path, &child_route)?;
            } else if kind.is_file() && path.extension().is_some_and(|ext| ext == "rs") {
                match path.file_stem().and_then(|stem| stem.to_str()) {
                    Some("mod") => self.set_page(route.to_owned(), path)?,
                    Some("layout") => self.layout = Some(NodeLayout { source: path }),
                    _ => {
                        let segment = route_segment(&path, path.file_stem())?;
                        let child_route = join_route(route, segment);
                        self.children
                            .entry(segment.to_owned())
                            .or_default()
                            .set_page(child_route, path)?;
                    }
                }
            }
        }
        Ok(())
    }

    fn set_page(&mut self, route: String, source: PathBuf) -> Result<(), DiscoveryError> {
        if let Some(existing) = &self.page {
            return Err(DiscoveryError::Conflict {
                route,
                first: existing.source.clone(),
                second: source,
            });
        }
        self.page = Some(Route {
            path: route,
            source,
        });
        Ok(())
    }
}

fn route_segment<'a>(
    path: &Path,
    name: Option<&'a std::ffi::OsStr>,
) -> Result<&'a str, DiscoveryError> {
    if let Some(name) = name.and_then(|name| name.to_str())
        && let Some((&first, rest)) = name.as_bytes().split_first()
        && (first.is_ascii_alphabetic() || first == b'_')
        && rest
            .iter()
            .all(|&byte| byte.is_ascii_alphanumeric() || byte == b'_')
        && !matches!(
            name,
            "_" | "self" | "Self" | "super" | "crate" | "mod" | "layout"
        )
    {
        return Ok(name);
    }
    Err(DiscoveryError::InvalidName(path.to_owned()))
}

fn join_route(parent: &str, segment: &str) -> String {
    format!("{}/{segment}", parent.trim_end_matches('/'))
}

#[derive(Debug)]
pub enum DiscoveryError {
    Read {
        path: PathBuf,
        source: io::Error,
    },
    InvalidName(PathBuf),
    Conflict {
        route: String,
        first: PathBuf,
        second: PathBuf,
    },
}

impl fmt::Display for DiscoveryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read { path, source } => write!(f, "cannot scan {}: {source}", path.display()),
            Self::InvalidName(path) => write!(
                f,
                "invalid page or directory name at {}: use an ASCII identifier compatible with a Rust module",
                path.display()
            ),
            Self::Conflict {
                route,
                first,
                second,
            } => write!(
                f,
                "route {route} is defined by both {} and {}",
                first.display(),
                second.display()
            ),
        }
    }
}

impl std::error::Error for DiscoveryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Read { source, .. } => Some(source),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
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
    fn discovers_pages_and_nested_layouts_in_sorted_order() -> Result<(), Box<dyn std::error::Error>>
    {
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
}
