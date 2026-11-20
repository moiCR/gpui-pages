use gpui_pages::prelude::*;
#[cfg(test)]
mod tests;

#[rustfmt::skip]
#[path = "pages_modules.rs"]
pub mod pages;

include_pages!();

fn main() {
    gpui_platform::application()
        .use_pages()
        .routes(PAGE_ROUTES)
        .title("Pages Demo")
        .run();
}
