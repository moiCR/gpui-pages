use gpui_pages::prelude::*;

pub struct ProfilePage;

impl Page for ProfilePage {
    fn new(_: &mut Context<Self>) -> Self {
        Self
    }

    fn options() -> PageOptions {
        PageOptions::default()
            .title("Profile")
            .on_open(PageAnimation::SlideFade)
            .on_close(PageAnimation::Fade)
            .without_root_layout()
    }

    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().text_xl().child("Profile")
    }
}
