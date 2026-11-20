use crate::{PAGE_ROUTES, ROUTES, pages};
use gpui_pages::{
    ApplicationExt, Layout, NavigationData, NavigationError, Page, PageManager, PageOptions, Route,
    gpui::{App, AppContext, Context, IntoElement, Render, Window, div},
    mount_layout, mount_page,
};
use std::cell::{Cell, RefCell};

thread_local! {
    static ROOT_MOUNTS: Cell<usize> = const { Cell::new(0) };
    static DASHBOARD_MOUNTS: Cell<usize> = const { Cell::new(0) };
    static REGISTER_MOUNTS: Cell<usize> = const { Cell::new(0) };
    static REGISTER_DATA: RefCell<Vec<RegisterData>> = const { RefCell::new(Vec::new()) };
}

const TRACKED_LAYOUTS: &[Layout] = &[
    Layout {
        id: "root",
        is_root: true,
        mount: |child, cx| {
            ROOT_MOUNTS.with(|count| count.set(count.get() + 1));
            mount_layout::<pages::layout::RootLayout>(child, cx)
        },
    },
    Layout {
        id: "dashboard",
        is_root: false,
        mount: |child, cx| {
            DASHBOARD_MOUNTS.with(|count| count.set(count.get() + 1));
            mount_layout::<pages::dashboard::layout::DashboardLayout>(child, cx)
        },
    },
];

#[test]
fn reuses_dashboard_layout_between_sibling_routes() {
    const ROUTES: &[Route] = &[
        Route {
            path: "/juanito",
            mount: mount_page::<pages::dashboard::juanito::JuanitoPage>,
            options: PageOptions::default,
            layouts: TRACKED_LAYOUTS,
        },
        Route {
            path: "/settings",
            mount: mount_page::<pages::dashboard::settings::SettingsPage>,
            options: PageOptions::default,
            layouts: TRACKED_LAYOUTS,
        },
        Route {
            path: "/bare",
            mount: mount_page::<pages::dashboard::settings::SettingsPage>,
            options: || PageOptions::default().without_root_layout(),
            layouts: TRACKED_LAYOUTS,
        },
        Route {
            path: "/:id",
            mount: mount_page::<pages::dashboard::_id::DashboardItem>,
            options: PageOptions::default,
            layouts: TRACKED_LAYOUTS,
        },
    ];
    gpui_platform::headless().use_pages().run_with(|cx| {
        cx.quit();
        verifies_one_shot_navigation_data(cx);
        DASHBOARD_MOUNTS.with(|count| count.set(0));
        let manager = PageManager::new(ROUTES, "/juanito", cx).expect("mount Juanito");
        cx.new(|_| manager).update(cx, |manager, cx| {
            assert!(!manager.can_back());
            assert!(!manager.can_forward());
            manager
                .navigate("/settings", cx)
                .expect("navigate to Settings");
            assert!(manager.can_back());
            assert!(!manager.can_forward());
            assert!(matches!(
                manager.navigate("/missing/extra", cx),
                Err(NavigationError::UnknownRoute(_))
            ));
            assert_eq!(manager.current_path(), "/settings");
            assert!(manager.back(cx));
            assert!(!manager.can_back());
            assert!(manager.can_forward());
            assert_eq!(manager.current_path(), "/juanito");
            assert!(manager.forward(cx));
            assert!(manager.can_back());
            assert!(!manager.can_forward());
            assert_eq!(manager.current_path(), "/settings");
            manager.replace("/juanito", cx).expect("replace Settings");
            assert_eq!(manager.current_path(), "/juanito");
            assert!(manager.back(cx));
            assert!(manager.forward(cx));
            assert_eq!(manager.current_path(), "/juanito");
            assert!(!manager.forward(cx));
            manager
                .navigate("/42?tab=details&flag", cx)
                .expect("open item 42");
            assert_eq!(manager.current_path(), "/42");
            assert_eq!(manager.param("id"), Some("42"));
            assert_eq!(manager.query("tab"), Some("details"));
            assert_eq!(manager.query("flag"), Some(""));
            assert_eq!(manager.query("missing"), None);
            assert_eq!(manager.query_string(), Some("tab=details&flag"));
            manager
                .navigate("/42?tab=logs", cx)
                .expect("update query on same path");
            assert_eq!(manager.query("tab"), Some("logs"));
            assert!(manager.back(cx));
            assert_eq!(manager.query("tab"), Some("details"));
            manager.navigate("/73", cx).expect("open item 73");
            assert_eq!(manager.current_path(), "/73");
            assert_eq!(manager.param("id"), Some("73"));
            assert_eq!(manager.query_string(), None);
            assert!(manager.back(cx));
            assert_eq!(manager.current_path(), "/42");
            assert_eq!(manager.param("id"), Some("42"));
            assert!(manager.forward(cx));
            assert_eq!(manager.param("id"), Some("73"));
            manager.replace("/91", cx).expect("replace item");
            assert_eq!(manager.param("id"), Some("91"));
            manager
                .navigate("/settings", cx)
                .expect("prefer static route");
            assert_eq!(manager.param("id"), None);
        });
        DASHBOARD_MOUNTS.with(|count| assert_eq!(count.get(), 1));
        ROOT_MOUNTS.with(|count| assert_eq!(count.get(), 1));
        let bare = PageManager::new(ROUTES, "/bare", cx).expect("mount without root");
        ROOT_MOUNTS.with(|count| assert_eq!(count.get(), 1));
        DASHBOARD_MOUNTS.with(|count| assert_eq!(count.get(), 2));
        cx.new(|_| bare).update(cx, |manager, cx| {
            manager.navigate("/settings", cx).expect("restore root");
            assert!(manager.back(cx));
        });
        ROOT_MOUNTS.with(|count| assert_eq!(count.get(), 2));
        DASHBOARD_MOUNTS.with(|count| assert_eq!(count.get(), 4));
        let manager = PageManager::new(PAGE_ROUTES, "/", cx).expect("mount the generated index");
        cx.new(|_| manager).update(cx, |manager, cx| {
            assert_eq!(manager.current_title(), Some("Home"));
            manager
                .navigate("/profile", cx)
                .expect("start opening Profile");
            assert_eq!(manager.current_title(), Some("Profile"));
            manager
                .navigate("/dashboard/settings", cx)
                .expect("interrupt the Profile transition");
            assert_eq!(manager.current_path(), "/dashboard/settings");
            assert_eq!(manager.current_title(), None);
            assert!(manager.back(cx));
            assert_eq!(manager.current_title(), Some("Profile"));
        });
    });
}

#[derive(Debug, PartialEq, Eq)]
struct RegisterData {
    email: String,
    password: String,
}

struct RegisterPage;

impl Page for RegisterPage {
    fn new(_: &mut Context<Self>) -> Self {
        REGISTER_MOUNTS.with(|count| count.set(count.get() + 1));
        Self
    }

    fn on_navigate(&mut self, data: NavigationData, _: &mut Context<Self>) {
        if let Some(data) = data.take::<RegisterData>() {
            REGISTER_DATA.with_borrow_mut(|received| received.push(data));
        }
    }

    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

impl Render for RegisterPage {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        Page::render(self, window, cx)
    }
}

fn verifies_one_shot_navigation_data(cx: &mut App) {
    const ROUTES: &[Route] = &[
        Route {
            path: "/",
            mount: mount_page::<pages::HomePage>,
            options: PageOptions::default,
            layouts: &[],
        },
        Route {
            path: "/register",
            mount: mount_page::<RegisterPage>,
            options: PageOptions::default,
            layouts: &[],
        },
    ];
    let data = || RegisterData {
        email: "demo@example.test".into(),
        password: "example-only".into(),
    };
    let manager = PageManager::new(ROUTES, "/", cx).expect("mount Home");
    cx.new(|_| manager).update(cx, |manager, cx| {
        assert!(matches!(
            manager.navigate_with("/missing", data(), cx),
            Err(NavigationError::UnknownRoute(_))
        ));
        manager
            .navigate_with("/register", data(), cx)
            .expect("deliver payload");
        assert_eq!(manager.current_path(), "/register");
        manager
            .navigate_with("/register", data(), cx)
            .expect("deliver a fresh payload");
        assert!(manager.back(cx));
        assert_eq!(manager.current_path(), "/");
        assert!(manager.forward(cx));
        assert_eq!(manager.current_path(), "/register");
    });
    REGISTER_MOUNTS.with(|count| assert_eq!(count.get(), 3));
    REGISTER_DATA.with_borrow(|received| assert_eq!(received.as_slice(), [data(), data()]));
}

#[test]
fn compiles_page_modules_and_inherits_layouts() {
    let _ = pages::dashboard::juanito::JuanitoPage;
    let routes: Vec<_> = ROUTES
        .iter()
        .map(|route| (route.path, route.page, route.layouts))
        .collect();
    let root = &["pages::layout"][..];
    let dashboard = &["pages::layout", "pages::dashboard::layout"][..];
    assert_eq!(
        routes,
        [
            ("/", "pages", root),
            ("/dashboard", "pages::dashboard", dashboard),
            ("/dashboard/:id", "pages::dashboard::_id", dashboard),
            ("/dashboard/juanito", "pages::dashboard::juanito", dashboard),
            (
                "/dashboard/settings",
                "pages::dashboard::settings",
                dashboard
            ),
            ("/profile", "pages::profile", root),
        ]
    );
}
