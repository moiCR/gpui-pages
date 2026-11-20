use gpui_pages::{
    ApplicationExt, PageManager,
    gpui::{
        AnyView, App, Context, Entity, IntoElement, Render, Window, WindowOptions, div, prelude::*,
    },
};

include!(concat!(env!("OUT_DIR"), "/pages.rs"));

fn main() {
    gpui_platform::application().use_pages().run(|cx| {
        let manager = match PageManager::new(PAGE_ROUTES, "/dashboard/analytics", cx) {
            Ok(manager) => cx.new(|_| manager),
            Err(error) => {
                eprintln!("{error}");
                return;
            }
        };
        if let Err(error) = cx.open_window(WindowOptions::default(), |_, cx| {
            cx.new(|_| Demo { manager })
        }) {
            eprintln!("{error}");
            return;
        }
        cx.activate(true);
    });
}

struct Demo {
    manager: Entity<PageManager>,
}

impl Render for Demo {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .p_4()
            .child(
                div().flex().gap_4().children(
                    [
                        ("/dashboard/analytics", "Analytics"),
                        ("/dashboard/users", "Users"),
                    ]
                    .map(|(route, label)| {
                        div()
                            .id(route)
                            .cursor_pointer()
                            .child(label)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.manager.update(cx, |manager, cx| {
                                    if let Err(error) = manager.navigate(route, cx) {
                                        eprintln!("{error}");
                                    }
                                });
                            }))
                    }),
                ),
            )
            .child(self.manager.clone())
    }
}

struct Title(&'static str);

impl Render for Title {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().text_xl().child(self.0)
    }
}

fn mount_title(title: &'static str, cx: &mut App) -> AnyView {
    cx.new(|_| Title(title)).into()
}

#[cfg(test)]
mod tests {
    use super::{ROUTES, pages};

    #[test]
    fn compiles_page_modules_and_inherits_layouts() {
        assert_eq!(pages::dashboard::analytics::TITLE, "Analytics");
        let routes: Vec<_> = ROUTES
            .iter()
            .map(|route| (route.path, route.page, route.layouts))
            .collect();
        let root = &["pages::layout"][..];
        let dashboard = &["pages::layout", "pages::dashboard::layout"][..];
        assert_eq!(
            routes,
            [
                ("/", "pages::__gpui_pages_index", root),
                (
                    "/dashboard",
                    "pages::dashboard::__gpui_pages_index",
                    dashboard
                ),
                (
                    "/dashboard/analytics",
                    "pages::dashboard::analytics",
                    dashboard
                ),
                ("/dashboard/users", "pages::dashboard::users", dashboard),
                ("/settings", "pages::settings", root),
                ("/type", "pages::type", root),
            ]
        );
        assert_eq!(pages::r#type::TITLE, "Type");
    }
}
