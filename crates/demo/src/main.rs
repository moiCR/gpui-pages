use gpui_pages::prelude::*;

include_pages!();

fn main() {
    gpui_platform::application()
        .use_pages()
        .routes(PAGE_ROUTES)
        .run();
}

#[cfg(test)]
mod tests {
    use super::{PAGE_ROUTES, ROUTES, pages};
    use gpui_pages::{
        ApplicationExt, Layout, NavigationError, PageManager, PageOptions, Route, gpui::AppContext,
        mount_layout, mount_page,
    };
    use std::cell::Cell;

    thread_local! {
        static DASHBOARD_MOUNTS: Cell<usize> = const { Cell::new(0) };
    }

    const TRACKED_LAYOUTS: &[Layout] = &[
        Layout {
            id: "root",
            mount: mount_layout::<pages::layout::RootLayout>,
        },
        Layout {
            id: "dashboard",
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
        ];
        gpui_platform::headless().use_pages().run_with(|cx| {
            cx.quit();
            DASHBOARD_MOUNTS.with(|count| count.set(0));
            let manager = PageManager::new(ROUTES, "/juanito", cx).expect("mount Juanito");
            cx.new(|_| manager).update(cx, |manager, cx| {
                manager
                    .navigate("/settings", cx)
                    .expect("navigate to Settings");
                assert!(matches!(
                    manager.navigate("/missing", cx),
                    Err(NavigationError::UnknownRoute(_))
                ));
                assert_eq!(manager.current_path(), "/settings");
                assert!(manager.back(cx));
                assert_eq!(manager.current_path(), "/juanito");
                assert!(manager.forward(cx));
                assert_eq!(manager.current_path(), "/settings");
                manager.replace("/juanito", cx).expect("replace Settings");
                assert_eq!(manager.current_path(), "/juanito");
                assert!(manager.back(cx));
                assert!(manager.forward(cx));
                assert_eq!(manager.current_path(), "/juanito");
                assert!(!manager.forward(cx));
            });
            DASHBOARD_MOUNTS.with(|count| assert_eq!(count.get(), 1));
            let manager =
                PageManager::new(PAGE_ROUTES, "/", cx).expect("mount the generated index");
            cx.new(|_| manager).update(cx, |manager, cx| {
                manager
                    .navigate("/profile", cx)
                    .expect("start opening Profile");
                manager
                    .navigate("/dashboard/settings", cx)
                    .expect("interrupt the Profile transition");
                assert_eq!(manager.current_path(), "/dashboard/settings");
            });
        });
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
}
