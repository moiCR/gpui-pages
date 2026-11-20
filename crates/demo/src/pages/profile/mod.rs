use gpui_pages::prelude::*;

#[derive(Default)]
pub struct ProfilePage;

impl Page for ProfilePage {
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
