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
```

The build script watches `src/pages` and generates `src/pages_modules.rs` with ordinary module declarations so rust-analyzer can index route files. Run `cargo check` once after cloning to create it. You can ignore the generated file in Git. Cargo updates it when the route tree changes. The route registry and generated `Render` implementations go to Cargo's output directory and are included by the macro. The framework opens a GPUI window at `/`.

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

Use `manager.navigate_with("/register", RegisterData { email, password }, cx)` to pass a value to a page in memory. The destination receives it at creation through `Page::on_navigate`:

```rust
fn on_navigate(&mut self, data: NavigationData, _: &mut Context<Self>) {
    if let Some(data) = data.take::<RegisterData>() {
        self.email = data.email;
        self.password = data.password;
    }
}
```

Define `RegisterData` in a module shared by both pages. The payload is excluded from the URL and history. Direct visits and back or forward navigation create the page without a payload. Sending another payload to the current URL creates a new instance without adding a history entry.

A directory prefixed with an underscore defines a parameter. For example, `src/pages/users/_id/mod.rs` matches `/users/42`. In a page, use `router(window, cx)` and read `manager.read(cx).param("id")` to get the current value. Static segments have priority over parameter segments at the same depth. Parameter values are returned as written in the path, without percent decoding. A directory can have one dynamic child, and each parameter name can appear once in a route.

Pages can override `Page::options()` with `PageOptions::default().on_open(PageAnimation::SlideFade).on_close(PageAnimation::Fade)`. Use `.configure(PagesConfig::new().default_animation(...))` before `.routes(...)` to set app defaults. Scale effects adjust GPUI rem-based sizing within the transitioning view. Fixed-pixel dimensions keep their size.

Use `PageOptions::default().title("Profile")` to set a page's window title. Pages without a title use the application title.

Use `PageOptions::default().without_root_layout()` to omit `src/pages/layout.rs` for a page. Nested layouts still apply.

Run `cargo run -p demo` to visit the example. Increment the Dashboard layout counter and switch between Juanito and Settings to see the shared layout retain its state. Open Profile to see a transition.
