use gpui_pages::{
    Page,
    gpui::{Context, IntoElement, Window, div, prelude::*},
};

pub struct SettingsPage;

impl Page for SettingsPage {
    fn new(_: &mut Context<Self>) -> Self {
        Self
    }

    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().text_xl().child("Settings")
    }
}
