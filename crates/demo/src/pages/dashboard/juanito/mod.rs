use gpui_pages::{
    Page,
    gpui::{Context, IntoElement, Window, div, prelude::*},
};

mod helpers;

#[derive(Default)]
pub struct JuanitoPage;

impl Page for JuanitoPage {
    fn options() -> gpui_pages::PageOptions {
        gpui_pages::PageOptions::default().title(helpers::TITLE)
    }

    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().text_xl().child(helpers::TITLE)
    }
}
