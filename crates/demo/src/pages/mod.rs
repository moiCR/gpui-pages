use gpui_pages::{
    Page,
    gpui::{Context, IntoElement, Window, div, prelude::*},
};

#[derive(Default)]
pub struct HomePage;

impl Page for HomePage {
    fn options() -> gpui_pages::PageOptions {
        gpui_pages::PageOptions::default().title("Home")
    }

    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().text_xl().child("Home")
    }
}
