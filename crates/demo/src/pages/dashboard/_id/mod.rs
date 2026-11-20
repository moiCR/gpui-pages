use gpui_pages::prelude::*;

#[derive(Default)]
pub struct DashboardItem;

impl Page for DashboardItem {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let id = router(window, cx)
            .and_then(|manager| manager.read(cx).param("id").map(str::to_owned))
            .unwrap_or_default();
        div().text_xl().child(format!("Dashboard item {id}"))
    }
}
