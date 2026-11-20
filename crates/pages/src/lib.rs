pub use gpui;
use gpui::{AnyView, App, Application, Context, Global, IntoElement, Render, Window, div};

struct PagesEnabled;

impl Global for PagesEnabled {}

pub trait ApplicationExt {
    fn use_pages(self) -> Self;
}

impl ApplicationExt for Application {
    fn use_pages(self) -> Self {
        self.update(|cx| cx.set_global(PagesEnabled));
        self
    }
}

pub trait PageLayout: Render {
    fn set_child(&mut self, child: AnyView);
}

type UpdateChild = Box<dyn Fn(AnyView, &mut App)>;

pub struct MountedLayout {
    view: AnyView,
    update_child: UpdateChild,
}

impl MountedLayout {
    pub fn new<L: PageLayout>(layout: L, cx: &mut App) -> Self {
        let entity = cx.new(|_| layout);
        Self {
            view: entity.clone().into(),
            update_child: Box::new(move |child, cx| {
                entity.update(cx, |layout, cx| {
                    layout.set_child(child);
                    cx.notify();
                });
            }),
        }
    }
}

pub struct Layout {
    pub id: &'static str,
    pub mount: fn(AnyView, &mut App) -> MountedLayout,
}

pub struct Route {
    pub path: &'static str,
    pub mount: fn(&mut App) -> AnyView,
    pub layouts: &'static [Layout],
}

#[derive(Debug)]
pub enum NavigationError {
    Disabled,
    UnknownRoute(String),
}

impl std::fmt::Display for NavigationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Disabled => f.write_str("call Application::use_pages() before mounting pages"),
            Self::UnknownRoute(path) => write!(f, "unknown page route {path}"),
        }
    }
}

impl std::error::Error for NavigationError {}

pub struct PageManager {
    routes: &'static [Route],
    active: &'static Route,
    layouts: Vec<MountedLayout>,
    view: AnyView,
}

impl PageManager {
    pub fn new(
        routes: &'static [Route],
        path: &str,
        cx: &mut App,
    ) -> Result<Self, NavigationError> {
        if cx.try_global::<PagesEnabled>().is_none() {
            return Err(NavigationError::Disabled);
        }
        let active = find_route(routes, path)?;
        let (view, layouts) = mount_branch(active, 0, cx);
        Ok(Self {
            routes,
            active,
            layouts,
            view,
        })
    }

    pub fn navigate(&mut self, path: &str, cx: &mut Context<Self>) -> Result<(), NavigationError> {
        let route = find_route(self.routes, path)?;
        if self.active.path == route.path {
            return Ok(());
        }
        let shared = self
            .active
            .layouts
            .iter()
            .zip(route.layouts)
            .take_while(|(old, new)| old.id == new.id)
            .count();
        self.layouts.truncate(shared);
        let (view, layouts) = mount_branch(route, shared, cx);
        if let Some(layout) = self.layouts.last() {
            (layout.update_child)(view, cx);
        } else {
            self.view = view;
        }
        self.layouts.extend(layouts);
        self.active = route;
        cx.notify();
        Ok(())
    }
}

impl Render for PageManager {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().child(self.view.clone())
    }
}

fn find_route(routes: &'static [Route], path: &str) -> Result<&'static Route, NavigationError> {
    routes
        .iter()
        .find(|route| route.path == path)
        .ok_or_else(|| NavigationError::UnknownRoute(path.to_owned()))
}

fn mount_branch(route: &Route, shared: usize, cx: &mut App) -> (AnyView, Vec<MountedLayout>) {
    let mut view = (route.mount)(cx);
    let mut layouts = Vec::new();
    for layout in route.layouts[shared..].iter().rev() {
        let mounted = (layout.mount)(view, cx);
        view = mounted.view.clone();
        layouts.push(mounted);
    }
    layouts.reverse();
    (view, layouts)
}
