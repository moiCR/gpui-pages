use gpui_pages::{
    PageLayout,
    gpui::{AnyView, Context, IntoElement, Window, div, prelude::*},
    router,
};

pub(crate) struct RootLayout(AnyView);

impl PageLayout for RootLayout {
    fn new(child: AnyView, _: &mut Context<Self>) -> Self {
        Self(child)
    }

    fn set_child(&mut self, child: AnyView) {
        self.0 = child;
    }
    
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .p_4()
            .child(
                div().flex().gap_4().children(
                    [
                        ("/", "Home"),
                        ("/dashboard", "Dashboard"),
                        ("/dashboard/juanito", "Juanito"),
                        ("/dashboard/settings", "Settings"),
                        ("/dashboard/42", "Item 42"),
                        ("/dashboard/73", "Item 73"),
                        ("/profile", "Profile"),
                    ]
                    .map(|(path, label)| {
                        div().id(path).cursor_pointer().child(label).on_click(
                            move |_, window, cx| {
                                if let Some(manager) = router(window, cx) {
                                    manager.update(cx, |manager, cx| {
                                        if let Err(error) = manager.navigate(path, cx) {
                                            eprintln!("{error}");
                                        }
                                    });
                                }
                            },
                        )
                    }),
                ),
            )
            .child("Root")
            .child(self.0.clone())
    }
}
