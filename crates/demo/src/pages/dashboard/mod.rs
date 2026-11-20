use gpui_pages::{
    Page,
    gpui::{Context, IntoElement, Window, div, prelude::*},
};

pub struct DashboardPage;

impl Page for DashboardPage {
    fn new(_: &mut Context<Self>) -> Self {
        Self
    }

    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().text_xl().child("Dashboard")
    }
}
