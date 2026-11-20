pub const TITLE: &str = "Root";

use gpui_pages::{
    MountedLayout, PageLayout,
    gpui::{AnyView, App, Context, IntoElement, Render, Window, div, prelude::*},
};

struct RootLayout(AnyView);

impl PageLayout for RootLayout {
    fn set_child(&mut self, child: AnyView) {
        self.0 = child;
    }
}

impl Render for RootLayout {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().mt_4().child(TITLE).child(self.0.clone())
    }
}

pub fn mount(child: AnyView, cx: &mut App) -> MountedLayout {
    MountedLayout::new(RootLayout(child), cx)
}
