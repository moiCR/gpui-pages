use crate::layout::NodeLayout;
#[cfg(test)]
mod tests;

use std::{
    collections::{BTreeMap, HashMap},
    fmt, fs, io,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone)]
pub struct Route {
    pub path: String,
    pub source: PathBuf,
}

#[derive(Debug)]
pub struct RouteEntry<'a> {
    pub page: &'a Route,
    pub layouts: Vec<&'a NodeLayout>,
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
        root.scan(&crate_root.as_ref().join(crate::VIEWS_DIR), "/")?;
        root.validate_unique_routes()?;
        Ok(root)
    }

    fn validate_unique_routes(&self) -> Result<(), DiscoveryError> {
        let mut seen: HashMap<String, &Route> = HashMap::new();
        for entry in self.route_table() {
            let normalized = normalize_route(&entry.page.path);
            if let Some(first) = seen.insert(normalized, entry.page) {
                if first.path == entry.page.path {
                    return Err(DiscoveryError::DuplicateRoute {
                        path: entry.page.path.clone(),
                        first: first.source.clone(),
                        second: entry.page.source.clone(),
                    });
                }
                return Err(DiscoveryError::AmbiguousParameters {
                    first: first.source.clone(),
                    second: entry.page.source.clone(),
                });
            }
        }
        Ok(())
    }

    pub fn route_table(&self) -> Vec<RouteEntry<'_>> {
        let mut routes = Vec::new();
        self.collect_routes(&mut Vec::new(), &mut routes);
        routes
    }

    fn collect_routes<'a>(
        &'a self,
        layouts: &mut Vec<&'a NodeLayout>,
        routes: &mut Vec<RouteEntry<'a>>,
    ) {
        let inherited_count = layouts.len();
        layouts.extend(self.layout.as_ref());

        if let Some(page) = &self.page {
            routes.push(RouteEntry {
                page,
                layouts: layouts.clone(),
            });
        }

        for child in self.children.values() {
            child.collect_routes(layouts, routes);
        }

        layouts.truncate(inherited_count);
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
        let mut dynamic_directory = None;

        for entry in entries {
            let path = entry.path();
            let kind = entry.file_type().map_err(|source| DiscoveryError::Read {
                path: path.clone(),
                source,
            })?;

            if kind.is_dir() {
                let segment = route_segment(&path, path.file_name())?;
                let child_route = if is_route_group(segment) {
                    route.to_owned()
                } else if let Some(parameter) = segment.strip_prefix('_') {
                    if !parameter.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_') {
                        return Err(DiscoveryError::InvalidName(path));
                    }
                    if route
                        .split('/')
                        .any(|part| part.strip_prefix(':') == Some(parameter))
                    {
                        return Err(DiscoveryError::RepeatedParameter {
                            parameter: parameter.to_owned(),
                            path,
                        });
                    }
                    if let Some(first) = dynamic_directory.replace(path.clone()) {
                        return Err(DiscoveryError::AmbiguousParameters {
                            first,
                            second: path,
                        });
                    }
                    join_route(route, &format!(":{parameter}"))
                } else {
                    join_route(route, segment)
                };
                self.children
                    .entry(segment.to_owned())
                    .or_default()
                    .scan(&path, &child_route)?;
            } else if kind.is_file() {
                match path.file_name().and_then(|name| name.to_str()) {
                    Some("mod.rs") => {
                        self.page = Some(Route {
                            path: route.to_owned(),
                            source: path,
                        })
                    }
                    Some("layout.rs") => self.layout = Some(NodeLayout { source: path }),
                    _ => {}
                }
            }
        }
        Ok(())
    }
}

pub(crate) fn group_name(segment: &str) -> Option<&str> {
    segment.strip_prefix('(')?.strip_suffix(')')
}

fn is_route_group(segment: &str) -> bool {
    group_name(segment).is_some()
}

fn is_valid_segment_ident(name: &str) -> bool {
    let Some((&first, rest)) = name.as_bytes().split_first() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == b'_')
        && rest
            .iter()
            .all(|&byte| byte.is_ascii_alphanumeric() || byte == b'_')
        && !matches!(
            name,
            "_" | "self" | "Self" | "super" | "crate" | "mod" | "layout"
        )
}

fn route_segment<'a>(
    path: &Path,
    name: Option<&'a std::ffi::OsStr>,
) -> Result<&'a str, DiscoveryError> {
    if let Some(name) = name.and_then(|name| name.to_str()) {
        if let Some(group) = group_name(name)
            && is_valid_segment_ident(group)
        {
            return Ok(name);
        }
        if is_valid_segment_ident(name) {
            return Ok(name);
        }
    }
    Err(DiscoveryError::InvalidName(path.to_owned()))
}

fn normalize_route(route: &str) -> String {
    route
        .split('/')
        .map(|segment| {
            if segment.starts_with(':') {
                ":"
            } else {
                segment
            }
        })
        .collect::<Vec<_>>()
        .join("/")
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
    RepeatedParameter {
        path: PathBuf,
        parameter: String,
    },
    AmbiguousParameters {
        first: PathBuf,
        second: PathBuf,
    },
    DuplicateRoute {
        path: String,
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
                "invalid route directory name at {}: use an ASCII identifier compatible with a Rust module",
                path.display()
            ),
            Self::RepeatedParameter { path, parameter } => {
                write!(
                    f,
                    "{} repeats the route parameter {parameter}",
                    path.display()
                )
            }
            Self::AmbiguousParameters { first, second } => write!(
                f,
                "ambiguous dynamic route directories {} and {}",
                first.display(),
                second.display()
            ),
            Self::DuplicateRoute {
                path,
                first,
                second,
            } => write!(
                f,
                "duplicate route {path} defined by {} and {}",
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
