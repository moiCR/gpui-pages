use gpui_pages::{
    Page,
    gpui::{Context, IntoElement, Window, div, prelude::*},
};

pub struct HomePage;

impl Page for HomePage {
    fn new(_: &mut Context<Self>) -> Self {
        Self
    }

    fn options() -> gpui_pages::PageOptions {
        gpui_pages::PageOptions::default().title("Home")
    }

    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().text_xl().child("Home")
    }
}
