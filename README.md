# gpui-pages
Directory-based routing for GPUI. Automatically discover and register pages without manual route configuration.

Add `gpui-pages` and the matching `gpui-pre-platform` backend to the app's dependencies, and add `pages-codegen` to its build dependencies. For a complete workspace example, see `crates/demo/Cargo.toml`.

`build.rs`

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    pages_codegen::build()
}
```

`src/main.rs`

```rust
use gpui_pages::prelude::*;

include_pages!();

fn main() {
    gpui_platform::application()
        .use_pages()
        .routes(PAGE_ROUTES)
        .title("Pages Demo")
        .run();
}
```

The build script watches `src/pages` and writes the registry to Cargo's output directory. The macro includes that registry once. The framework opens a GPUI window at `/`.

`src/pages/mod.rs`

```rust
use gpui_pages::prelude::*;

#[derive(Default)]
pub struct HomePage;

impl Page for HomePage {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().child("Home")
    }
}
```

| Directory index | Route |
| --- | --- |
| `src/pages/mod.rs` | `/` |
| `src/pages/dashboard/mod.rs` | `/dashboard` |
| `src/pages/dashboard/juanito/mod.rs` | `/dashboard/juanito` |
| `src/pages/dashboard/settings/mod.rs` | `/dashboard/settings` |
| `src/pages/profile/mod.rs` | `/profile` |

Every route comes from a directory containing `mod.rs`. Directories without an index can contain nested routes and inherited layouts. Declare auxiliary files such as `helpers.rs` with ordinary Rust module declarations inside the page. The generator declares the route directories and exposes page types at paths such as `pages::dashboard::juanito::JuanitoPage`.

Each `mod.rs` contains one top-level `impl Page` for a local, non-generic `pub` or `pub(crate)` struct. Qualified trait paths ending in `Page` are accepted. A `layout.rs` has the same visibility convention and implements `PageLayout`, with methods for constructing it with its child, replacing that child, and rendering.

Get the window's router with `router(window, cx)`. Update the returned entity to call `navigate`, `replace`, `back`, or `forward`. An unknown route returns an error and leaves the current route in place.

Pages can override `Page::options()` with `PageOptions::default().on_open(PageAnimation::SlideFade).on_close(PageAnimation::Fade)`. Use `.configure(PagesConfig::new().default_animation(...))` before `.routes(...)` to set app defaults. Scale effects adjust GPUI rem-based sizing within the transitioning view. Fixed-pixel dimensions keep their size.

Use `PageOptions::default().title("Profile")` to set a page's window title. Pages without a title use the application title.

Use `PageOptions::default().without_root_layout()` to omit `src/pages/layout.rs` for a page. Nested layouts still apply.

Run `cargo run -p demo` to visit the example. Increment the Dashboard layout counter and switch between Juanito and Settings to see the shared layout retain its state. Open Profile to see a transition.
