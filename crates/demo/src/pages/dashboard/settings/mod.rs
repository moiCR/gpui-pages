use gpui_pages::{
    Page,
    gpui::{Context, IntoElement, Window, div, prelude::*},
};

#[derive(Default)]
pub struct SettingsPage;

impl Page for SettingsPage {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().text_xl().child("Settings")
    }
}
