# gpui-pages

Directory-based routing for GPUI (`gpui-pre`), inspired by Next.js App Router. Automatically discover pages, nested layouts, dynamic segments, and route groups at compile time without manual route tables.

## Features

- **File-based routing**: `src/pages/**/mod.rs` defines routes with deterministic compile-time discovery and validation.
- **Persistent nested layouts**: `layout.rs` files wrap descendant routes and preserve GPUI entity state across sibling navigations.
- **Route groups**: `(group)/...` directories share layouts without adding URL segments.
- **Dynamic segments & query params**: `_id` directories match `:id` parameters; `?key=value` query strings are parsed out of the box.
- **Navigation, history & in-memory payloads**: `navigate`, `navigate_with`, `replace`, `back`, `forward`, `can_back`, and `can_forward`.
- **Page transitions & options**: per-page titles, `without_root_layout()`, and `Fade`/`Slide`/`Scale` transitions respecting `reduce_motion`.
- **Custom window support**: works standalone or embedded inside custom window wrappers such as `gpui-kit`.

## Version Compatibility

| Dependency | Version |
| --- | --- |
| `gpui-pre` | `0.3.12` |
| `gpui-kit` | `0.10.0` |

## Installation

Add `gpui-pages` and `gpui-pre-platform` to your dependencies. Enable only the `codegen` feature under `[build-dependencies]` so your build script generates routes without compiling the GPUI runtime.

```toml
[dependencies]
gpui-pages = "0.1"
gpui_platform = { package = "gpui-pre-platform", version = "=0.3.12" }

[build-dependencies]
gpui-pages = { version = "0.1", default-features = false, features = ["codegen"] }
```

## Quick Start

### `build.rs`

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    gpui_pages::build()
}
```

### `src/main.rs`

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

### `src/pages/mod.rs`

```rust
use gpui_pages::prelude::*;

pub struct HomePage;

impl Page for HomePage {
    fn new(_: &mut Context<Self>) -> Self {
        Self
    }

    fn options() -> PageOptions {
        PageOptions::default().title("Home")
    }

    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().child("Home")
    }
}
```

The build script watches `src/pages` and generates `src/pages_modules.rs` with ordinary module declarations so `rust-analyzer` can index all route files. Run `cargo check` once after cloning to generate it (you may ignore `src/pages_modules.rs` in Git). The route registry and generated `Render` implementations are emitted into `OUT_DIR` and included via `include_pages!()`.

## Routing Conventions

| File | Route |
| --- | --- |
| `src/pages/mod.rs` | `/` |
| `src/pages/dashboard/mod.rs` | `/dashboard` |
| `src/pages/dashboard/_id/mod.rs` | `/dashboard/:id` |
| `src/pages/(auth)/login/mod.rs` | `/login` |
| `src/pages/profile/mod.rs` | `/profile` |

- Only `mod.rs` defines a route. Other `.rs` files inside a page directory can be declared as ordinary auxiliary modules (e.g., `mod helpers;`).
- Directories without `mod.rs` can still contain descendant routes and a `layout.rs`.
- Each `mod.rs` must contain one top-level `impl Page` for a local, non-generic `pub` or `pub(crate)` struct.
- Directories wrapped in parentheses such as `(auth)` are **route groups**: they apply their `layout.rs` to descendants without adding a URL segment. Duplicate or ambiguous routes across groups are rejected at compile time.

## Layouts

Place a `layout.rs` file in any directory under `src/pages/` to wrap that directory's page and all nested routes. Shared ancestor layouts stay mounted and retain their state when navigating between sibling routes.

```rust
use gpui_pages::prelude::*;

pub(crate) struct RootLayout(AnyView);

impl PageLayout for RootLayout {
    fn new(child: AnyView, _: &mut Context<Self>) -> Self {
        Self(child)
    }

    fn set_child(&mut self, child: AnyView) {
        self.0 = child;
    }

    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(self.0.clone())
    }
}
```

## Navigation & History

Access the current window's router with `router(window, cx)` and update the returned `Entity<PageManager>`:

```rust
if let Some(manager) = router(window, cx) {
    manager.update(cx, |manager, cx| {
        let _ = manager.navigate("/dashboard/42?tab=overview", cx);
    });
}
```

Available methods on `PageManager`:

- `navigate(path, cx)` / `replace(path, cx)`
- `navigate_with(path, data, cx)` — passes an in-memory payload delivered once to `Page::on_navigate`:
  ```rust
  fn on_navigate(&mut self, data: NavigationData, _: &mut Context<Self>) {
      if let Some(data) = data.take::<RegisterData>() {
          self.email = data.email;
      }
  }
  ```
- `back(cx)` / `forward(cx)` / `can_back()` / `can_forward()`
- `current_path()` / `current_title()`

Unknown routes return `Err(NavigationError::UnknownRoute(_))` and leave the active route unchanged.

## Dynamic & Query Parameters

- A directory prefixed with `_` (for example `src/pages/users/_id/mod.rs`) defines a dynamic parameter `:id`. Static segments take precedence over dynamic segments at the same depth.
- Read route parameters with `manager.read(cx).param("id")`.
- Read query parameters with `manager.read(cx).query("tab")` or the raw query string with `manager.read(cx).query_string()`.
- Values are returned as written in the path without percent decoding.

## Page Options & Transitions

Override `Page::options()` to configure per-page behavior:

```rust
fn options() -> PageOptions {
    PageOptions::default()
        .title("Profile")
        .on_open(PageAnimation::SlideFade)
        .on_close(PageAnimation::Fade)
        .without_root_layout()
}
```

Set application-wide transition defaults with `.configure(PagesConfig::new().default_animation(...))` before `.routes(PAGE_ROUTES)`. Transitions automatically honor the system's `reduce_motion` preference.

## Custom Window Setup

When opening windows manually (for example with `gpui-kit`, custom `WindowOptions`, or system tray integration), use `run_with` together with `PageManager::new` / `PageManager::with_config` and `PageManager::attach`:

```rust
app.use_pages().run_with(|cx| {
    let manager = match PageManager::new(PAGE_ROUTES, "/", cx) {
        Ok(manager) => cx.new(|_| manager),
        Err(error) => {
            eprintln!("{error}");
            cx.quit();
            return;
        }
    };

    let _ = cx.open_window(WindowOptions::default(), move |window, cx| {
        PageManager::attach(&manager, window, cx);
        manager
    });
});
```

`PageManager::attach` registers the router for `router(window, cx)` and keeps the window title synchronized with `PageOptions::title`. You can also use `PageManager::mount` or `PageManager::mount_with_config` directly inside a window constructor.

## Example

Run `cargo run -p demo` to explore nested layouts, persistent layout state, dynamic routes, and page transitions.
