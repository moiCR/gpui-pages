pub const TITLE: &str = "Home";
pub fn mount(cx: &mut gpui_pages::gpui::App) -> gpui_pages::gpui::AnyView {
    crate::mount_title(TITLE, cx)
}
