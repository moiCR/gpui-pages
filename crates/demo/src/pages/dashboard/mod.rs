pub const TITLE: &str = "Dashboard";
pub fn mount(cx: &mut gpui_pages::gpui::App) -> gpui_pages::gpui::AnyView {
    crate::mount_title(TITLE, cx)
}
