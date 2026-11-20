pub const TITLE: &str = "Dashboard layout";

use gpui_pages::{
    MountedLayout, PageLayout,
    gpui::{AnyView, App, Context, IntoElement, Render, Window, div, prelude::*},
};

struct DashboardLayout {
    child: AnyView,
    count: usize,
}

impl PageLayout for DashboardLayout {
    fn set_child(&mut self, child: AnyView) {
        self.child = child;
    }
}

impl Render for DashboardLayout {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div().mt_4().child(TITLE).child(
            div().id("layout-counter").cursor_pointer()
                .child(format!("Layout counter: {}", self.count))
                .on_click(cx.listener(|layout, _, _, cx| {
                    layout.count += 1;
                    cx.notify();
                })),
        ).child(self.child.clone())
    }
}

pub fn mount(child: AnyView, cx: &mut App) -> MountedLayout {
    MountedLayout::new(DashboardLayout { child, count: 0 }, cx)
}
