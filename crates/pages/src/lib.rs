pub use gpui;
use gpui::{
    AnyView, App, AppContext, Application, Context, Entity, Global, IntoElement, ParentElement,
    Render, SharedString, Subscription, Window, WindowOptions, div,
};
pub use pages_macros::include_pages;
mod transition;
use transition::Transition;
pub use transition::{PageAnimation, PageOptions, PagesConfig};

struct PagesEnabled;

impl Global for PagesEnabled {}

pub trait ApplicationExt {
    fn use_pages(self) -> PagesApplication;
}

impl ApplicationExt for Application {
    fn use_pages(self) -> PagesApplication {
        PagesApplication {
            application: self,
            config: PagesConfig::default(),
        }
    }
}

pub struct PagesApplication {
    application: Application,
    config: PagesConfig,
}

impl PagesApplication {
    pub fn configure(mut self, config: PagesConfig) -> Self {
        self.config = config;
        self
    }

    pub fn routes(self, routes: &'static [Route]) -> RoutedApplication {
        RoutedApplication {
            application: self,
            routes,
            window_options: WindowOptions::default(),
        }
    }

    pub fn run_with(self, launch: impl FnOnce(&mut App) + 'static) {
        self.application.run(move |cx| {
            cx.set_global(PagesEnabled);
            launch(cx);
        });
    }
}

pub struct RoutedApplication {
    application: PagesApplication,
    routes: &'static [Route],
    window_options: WindowOptions,
}

impl RoutedApplication {
    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.window_options.titlebar.get_or_insert_default().title = Some(title.into());
        self
    }

    pub fn run(self) {
        let config = self.application.config;
        self.application.run_with(move |cx| {
            let manager = match PageManager::with_config(self.routes, "/", config, cx) {
                Ok(manager) => cx.new(|_| manager),
                Err(error) => {
                    eprintln!("{error}");
                    cx.quit();
                    return;
                }
            };
            let mut window_options = self.window_options;
            let default_title = window_options
                .titlebar
                .as_ref()
                .and_then(|bar| bar.title.clone());
            if let Some(title) = manager.read(cx).current_title() {
                window_options.titlebar.get_or_insert_default().title = Some(title.into());
            }
            match cx.open_window(window_options, |window, cx| {
                cx.new(|cx| {
                    let title_subscription =
                        cx.observe_in(&manager, window, move |_, manager, window, cx| {
                            let title = manager
                                .read(cx)
                                .current_title()
                                .or(default_title.as_deref())
                                .unwrap_or_default();
                            window.set_window_title(title);
                        });
                    PagesRoot {
                        manager,
                        _title_subscription: title_subscription,
                    }
                })
            }) {
                Ok(_) => cx.activate(true),
                Err(error) => {
                    eprintln!("{error}");
                    cx.quit();
                }
            }
        });
    }
}

struct PagesRoot {
    manager: Entity<PageManager>,
    _title_subscription: Subscription,
}

impl Render for PagesRoot {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().child(self.manager.clone())
    }
}

pub fn router(window: &Window, cx: &App) -> Option<Entity<PageManager>> {
    let root = window.root::<PagesRoot>().flatten()?;
    Some(root.read(cx).manager.clone())
}

pub trait PageLayout: Render {
    fn new(child: AnyView) -> Self;
    fn set_child(&mut self, child: AnyView);
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement;
}

pub trait Page: Render + Default {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement;

    fn options() -> PageOptions {
        PageOptions::default()
    }
}

pub fn mount_page<P: Page>(cx: &mut App) -> AnyView {
    cx.new(|_| P::default()).into()
}

pub fn mount_layout<L: PageLayout>(child: AnyView, cx: &mut App) -> MountedLayout {
    MountedLayout::new(L::new(child), cx)
}

pub mod prelude {
    pub use crate::{
        ApplicationExt, Page, PageAnimation, PageLayout, PageOptions, PagesConfig, include_pages,
        router,
    };
    pub use gpui::{AnyView, Context, IntoElement, Window, div, prelude::*};
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
    pub options: fn() -> PageOptions,
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
    page: AnyView,
    config: PagesConfig,
    transition: Option<Entity<Transition>>,
    history: Vec<&'static Route>,
    history_index: usize,
}

impl PageManager {
    pub fn new(
        routes: &'static [Route],
        path: &str,
        cx: &mut App,
    ) -> Result<Self, NavigationError> {
        Self::with_config(routes, path, PagesConfig::default(), cx)
    }

    fn with_config(
        routes: &'static [Route],
        path: &str,
        config: PagesConfig,
        cx: &mut App,
    ) -> Result<Self, NavigationError> {
        if cx.try_global::<PagesEnabled>().is_none() {
            return Err(NavigationError::Disabled);
        }
        let active = find_route(routes, path)?;
        let page = (active.mount)(cx);
        let transition = (!cx.reduce_motion())
            .then(|| (active.options)().effect(config, true))
            .flatten()
            .map(|effect| Transition::mount(page.clone(), Some(effect), None, cx));
        let content = transition
            .clone()
            .map_or_else(|| page.clone(), AnyView::from);
        let (view, layouts) = mount_layouts(active, 0, content, cx);
        Ok(Self {
            routes,
            active,
            layouts,
            view,
            page,
            config,
            transition,
            history: vec![active],
            history_index: 0,
        })
    }

    pub fn navigate(&mut self, path: &str, cx: &mut Context<Self>) -> Result<(), NavigationError> {
        let route = find_route(self.routes, path)?;
        if self.active.path == route.path {
            return Ok(());
        }
        self.history.truncate(self.history_index + 1);
        self.history.push(route);
        self.history_index += 1;
        self.activate(route, cx);
        Ok(())
    }

    pub fn replace(&mut self, path: &str, cx: &mut Context<Self>) -> Result<(), NavigationError> {
        let route = find_route(self.routes, path)?;
        self.history[self.history_index] = route;
        self.activate(route, cx);
        Ok(())
    }

    pub fn current_path(&self) -> &'static str {
        self.active.path
    }

    pub fn current_title(&self) -> Option<&'static str> {
        (self.active.options)().title
    }

    pub fn back(&mut self, cx: &mut Context<Self>) -> bool {
        self.history_index
            .checked_sub(1)
            .is_some_and(|index| self.go_to(index, cx))
    }

    pub fn forward(&mut self, cx: &mut Context<Self>) -> bool {
        self.go_to(self.history_index + 1, cx)
    }

    fn go_to(&mut self, index: usize, cx: &mut Context<Self>) -> bool {
        let Some(&route) = self.history.get(index) else {
            return false;
        };
        self.history_index = index;
        self.activate(route, cx);
        true
    }

    fn activate(&mut self, route: &'static Route, cx: &mut Context<Self>) {
        if self.active.path == route.path {
            return;
        }
        let shared = self
            .active
            .layouts
            .iter()
            .zip(route.layouts)
            .take_while(|(old, new)| old.id == new.id)
            .count();
        if let Some(transition) = self.transition.take() {
            transition.update(cx, Transition::finish);
        }
        let outgoing = self
            .layouts
            .get(shared)
            .map_or_else(|| self.page.clone(), |layout| layout.view.clone());
        self.layouts.truncate(shared);
        let page = (route.mount)(cx);
        let (mut view, layouts) = mount_layouts(route, shared, page.clone(), cx);
        if !cx.reduce_motion() {
            let opening = (route.options)().effect(self.config, true);
            let closing = (self.active.options)().effect(self.config, false);
            if opening.is_some() || closing.is_some() {
                let transition =
                    Transition::mount(view, opening, closing.map(|effect| (outgoing, effect)), cx);
                view = transition.clone().into();
                self.transition = Some(transition);
            }
        }
        if let Some(layout) = self.layouts.last() {
            (layout.update_child)(view, cx);
        } else {
            self.view = view;
        }
        self.layouts.extend(layouts);
        self.active = route;
        self.page = page;
        cx.notify();
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

fn mount_layouts(
    route: &Route,
    shared: usize,
    mut view: AnyView,
    cx: &mut App,
) -> (AnyView, Vec<MountedLayout>) {
    let mut layouts = Vec::new();
    for layout in route.layouts[shared..].iter().rev() {
        let mounted = (layout.mount)(view, cx);
        view = mounted.view.clone();
        layouts.push(mounted);
    }
    layouts.reverse();
    (view, layouts)
}
