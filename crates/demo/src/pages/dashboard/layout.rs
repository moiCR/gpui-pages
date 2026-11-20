pub const TITLE: &str = "Dashboard layout";

use gpui_pages::{
    PageLayout,
    gpui::{AnyView, Context, IntoElement, Window, div, prelude::*},
};

pub(crate) struct DashboardLayout {
    child: AnyView,
    count: usize,
}

impl PageLayout for DashboardLayout {
    fn new(child: AnyView) -> Self {
        Self { child, count: 0 }
    }

    fn set_child(&mut self, child: AnyView) {
        self.child = child;
    }
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .mt_4()
            .child(TITLE)
            .child(
                div()
                    .id("layout-counter")
                    .cursor_pointer()
                    .child(format!("Layout counter: {}", self.count))
                    .on_click(cx.listener(|layout, _, _, cx| {
                        layout.count += 1;
                        cx.notify();
                    })),
            )
            .child(self.child.clone())
    }
}
