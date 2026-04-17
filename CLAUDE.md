# CLAUDE.md — TwoClear Project Guide

This file provides guidance to Claude and other AI assistants when working on the TwoClear project.

## Project Overview

**TwoClear** is a Windows utility for cleaning up 1C:Enterprise 8 platform artifacts, written in Rust using GPUI and gpui-component. It is a spiritual port of [OneCleaner](https://github.com/vbondarevsky/OneCleaner) (C# / WPF).

The app has three tabs:
1. **Версии платформы** — lists installed 1C platform versions (from Windows Registry), allows uninstalling selected versions via `msiexec /x`.
2. **Кэш метаданных** — lists metadata cache directories in `%LOCALAPPDATA%\1C\1cv8*\{UUID}`, allows deleting selected entries.
3. **Информационные базы** — lists info bases from `%APPDATA%\1C\1CEStart\ibases.v8i`, allows removing entries (and optionally deleting the file database directory).

## Project Structure

```
TwoClear/
├── CLAUDE.md
├── spec.md
├── prompt_plan.md
├── todo.md
├── Cargo.toml
└── src/
    ├── main.rs            # Entry point: Application setup, window, Root
    ├── app.rs             # TwoClearApp struct: state + Render impl
    ├── models.rs          # Data models: InstalledVersion, CacheEntry, InfoBase
    └── platform/
        ├── mod.rs         # Re-exports, pub use
        ├── versions.rs    # Read Windows Registry (installed 1C versions)
        ├── cache.rs       # Read %LOCALAPPDATA%\1C cache directories
        ├── infobases.rs   # Parse ibases.v8i, delete entries
        └── utils.rs       # get_dir_size(), format_size()
```

## Key Commands

```bash
# Build
cargo build

# Run
cargo run

# Check
cargo clippy -- --deny warnings

# Format
cargo fmt
```

## Dependencies

```toml
[dependencies]
gpui = { git = "https://github.com/zed-industries/zed" }
gpui_platform = { git = "https://github.com/zed-industries/zed", features = ["font-kit", "runtime_shaders"] }
gpui-component = { git = "https://github.com/longbridge/gpui-component", package = "gpui-component" }
gpui-component-assets = { git = "https://github.com/longbridge/gpui-component", package = "gpui-component-assets" }
anyhow = "1"
smol = "2"
winreg = "0.10"
walkdir = "2"

[target.'cfg(target_os = "windows")'.dependencies]
windows = { version = "0.58", features = [
    "Win32_Graphics_Direct3D",
    "Win32_Graphics_Direct3D11",
    "Win32_Graphics_Dxgi",
] }
```

## GPUI Core Patterns

### Application Entry Point

```rust
use gpui::Application;
use gpui_component::{Root, TitleBar};
use gpui_platform;

fn main() {
    gpui_platform::application()
        .with_assets(gpui_component_assets::Assets)
        .run(|cx| {
            gpui_component::init(cx);  // MUST be called first

            let window_options = WindowOptions {
                titlebar: Some(TitleBar::title_bar_options()),
                window_bounds: Some(WindowBounds::centered(size(px(900.), px(640.)), cx)),
                ..Default::default()
            };

            cx.spawn(async move |cx| {
                cx.open_window(window_options, |window, cx| {
                    window.set_window_title("TwoClear");
                    let view = cx.new(|cx| TwoClearApp::new(window, cx));
                    // First view MUST be wrapped in Root
                    cx.new(|cx| Root::new(view, window, cx))
                })
                .expect("Failed to open window");
            })
            .detach();
        });
}
```

### State Management (Entity / Context)

```rust
pub struct TwoClearApp {
    active_tab: usize,
    versions: Vec<InstalledVersion>,
    versions_loading: bool,
    // ...
}

impl TwoClearApp {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut app = Self { /* ... */ };
        app.load_all_data(cx);
        app
    }

    fn load_versions(&mut self, cx: &mut Context<Self>) {
        self.versions_loading = true;
        cx.notify();

        cx.spawn(async move |this, mut cx| {
            let result = cx
                .background_executor()
                .spawn(async move { platform::get_installed_versions() })
                .await;

            this.update(&mut cx, |app, cx| {
                match result {
                    Ok(v) => app.versions = v,
                    Err(e) => app.error = Some(e.to_string()),
                }
                app.versions_loading = false;
                cx.notify();
            })
            .ok();
        })
        .detach();
    }
}
```

### Render Implementation

```rust
impl Render for TwoClearApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .bg(cx.theme().background)
            .child(self.render_title_bar(cx))
            .child(
                div()
                    .flex_1()
                    .overflow_hidden()
                    .map(|this| match self.active_tab {
                        0 => this.child(self.render_versions_tab(cx)),
                        1 => this.child(self.render_cache_tab(cx)),
                        2 => this.child(self.render_infobases_tab(cx)),
                        _ => this,
                    }),
            )
    }
}
```

## gpui-component Patterns

### Imports

```rust
use gpui_component::{
    ActiveTheme, Root, Sizable, TitleBar,
    button::Button,
    checkbox::Checkbox,
    tab::{Tab, TabBar},
    spinner::Spinner,
    v_flex, h_flex,
};
use gpui::prelude::FluentBuilder as _;
```

### TabBar (for title bar)

```rust
TitleBar::new()
    .child(
        TabBar::new("main-tabs")
            .selected_index(self.active_tab)
            .on_click(cx.listener(|this, ix: &usize, _window, cx| {
                this.active_tab = *ix;
                cx.notify();
            }))
            .child(Tab::new().label("Версии"))
            .child(Tab::new().label("Кэш"))
            .child(Tab::new().label("Базы"))
    )
```

### Button

```rust
use gpui_component::button::Button;

// Primary button
Button::new("id").label("Текст").primary()
    .on_click(cx.listener(|this, _event, _window, cx| {
        // handle
    }))

// Ghost button
Button::new("id").label("Текст").ghost()

// Small button
Button::new("id").label("Текст").small()

// Disabled
Button::new("id").label("Текст").disabled(condition)
```

### Checkbox

**Important**: `on_click` receives `&bool` — the NEW checked state (after toggle), not a click event.

```rust
use gpui_component::checkbox::Checkbox;

Checkbox::new(format!("item-{}", i))  // unique id per item
    .checked(item.selected)
    .on_click(cx.listener(move |this, checked: &bool, _window, cx| {
        this.items[i].selected = *checked;  // i: usize is Copy
        cx.notify();
    }))
```

### Scrollable List with Checkboxes

```rust
fn render_list(&self, cx: &Context<Self>) -> impl IntoElement {
    div()
        .flex_1()
        .overflow_y_scroll()
        .children(
            self.items.iter().enumerate().map(|(i, item)| {
                h_flex()
                    .px_3()
                    .py_2()
                    .gap_3()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .items_center()
                    .child(
                        Checkbox::new(format!("cb-{}", i))
                            .checked(item.selected)
                            .on_click(cx.listener(move |this, checked, _, cx| {
                                this.items[i].selected = *checked;
                                cx.notify();
                            })),
                    )
                    .child(div().flex_1().child(item.name.clone()))
                    .child(
                        div()
                            .w_20()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(format_size(item.size)),
                    )
            }),
        )
}
```

### Toolbar Pattern

```rust
fn render_toolbar(&self, cx: &Context<Self>) -> impl IntoElement {
    let selected_count = self.items.iter().filter(|i| i.selected).count();
    let selected_size: u64 = self.items.iter().filter(|i| i.selected).map(|i| i.size).sum();

    h_flex()
        .px_3()
        .py_2()
        .gap_2()
        .border_b_1()
        .border_color(cx.theme().border)
        .items_center()
        .child(
            Button::new("select-all")
                .label("Выбрать все")
                .small()
                .ghost()
                .on_click(cx.listener(|this, _, _, cx| {
                    this.items.iter_mut().for_each(|i| i.selected = true);
                    cx.notify();
                })),
        )
        .child(
            Button::new("deselect-all")
                .label("Снять выбор")
                .small()
                .ghost()
                .on_click(cx.listener(|this, _, _, cx| {
                    this.items.iter_mut().for_each(|i| i.selected = false);
                    cx.notify();
                })),
        )
        .child(div().flex_1())  // spacer
        .child(
            div()
                .text_sm()
                .text_color(cx.theme().muted_foreground)
                .child(format!(
                    "Выбрано: {} ({}) / Всего: {}",
                    selected_count,
                    format_size(selected_size),
                    format_size(total_size),
                )),
        )
        .child(
            Button::new("action-btn")
                .label("Удалить")
                .small()
                .primary()
                .disabled(selected_count == 0 || self.operation_in_progress)
                .on_click(cx.listener(|this, _, _, cx| {
                    this.start_delete(cx);
                })),
        )
}
```

### Spinner (loading state)

```rust
use gpui_component::spinner::Spinner;

div()
    .when(self.loading, |this| {
        this.child(
            div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .child(Spinner::new("loading")),
        )
    })
    .when(!self.loading, |this| {
        this.children(rendered_items)
    })
```

### Conditional rendering helpers

```rust
use gpui::prelude::FluentBuilder as _;

// .when(condition, |this| this.child(...))
div().when(self.loading, |d| d.child(spinner))

// .when_some(option, |this, value| this.child(...))
div().when_some(self.error.as_ref(), |d, err| {
    d.child(div().text_color(cx.theme().destructive).child(err.clone()))
})
```

### Theme Colors

```rust
cx.theme().background
cx.theme().foreground
cx.theme().muted_foreground
cx.theme().border
cx.theme().primary
cx.theme().destructive         // red, for errors/danger
cx.theme().card                // card background
cx.theme().tab_bar             // tab bar background
```

## Platform Layer

### Windows Registry — Installed Versions

1C platform versions are registered in:
- `HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall` (32-bit view)
- `HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall` (64-bit view, on 64-bit OS)

Criteria for 1C entry: `Publisher` must be one of: `"1С-Софт"`, `"1C-Soft"`, `"1C"`, `"1С"`.

Registry values per entry:
- `DisplayName` — display name
- `Publisher` — vendor
- `DisplayVersion` — version string, e.g. `"8.3.20.1710"`
- `InstallLocation` — install directory path
- `InstallDate` — date string in `yyyyMMdd` format

**Uninstall command:** `msiexec.exe /x{UUID} /q`

```rust
use winreg::{RegKey, enums::*};

let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
let key32 = hklm.open_subkey_with_flags(UNINSTALL_KEY, KEY_READ | KEY_WOW64_32KEY)?;
let key64 = hklm.open_subkey_with_flags(UNINSTALL_KEY, KEY_READ | KEY_WOW64_64KEY)?;
```

### Metadata Cache

Path pattern: `%LOCALAPPDATA%\1C\1cv8*\{UUID}`

- Parent dirs match glob `1cv8*`
- UUID dirs match regex: `[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}`
- UUID should be matched against InfoBase UUIDs from ibases.v8i (to show the base name)
- Cache entries where no matching base is found: `selected = true` by default (orphaned cache)

### ibases.v8i Format

Path: `%APPDATA%\1C\1CEStart\ibases.v8i`

This is an INI-like file:

```ini
[Имя базы]
Connect=File="C:\1c\base";
ID={GUID}
DefaultVersion=8.3.20

[Серверная база]
Connect=Srvr="server";Ref="basename";
ID={GUID}
Version=8.3.19
```

Key parsing rules:
- Section name = InfoBase name
- `Connect` value starting with `File=` → file database; extract path: strip `File="` prefix and `";` suffix
- `ID` value = UUID (match with cache entries)
- `DefaultVersion` or `Version` = preferred 1C version
- **Always create a backup** before modifying: `ibases.v8i_backup_{timestamp}`

**File path extraction from `Connect` value:**
```rust
fn extract_file_path(connection: &str) -> Option<String> {
    if connection.to_ascii_lowercase().starts_with("file=") {
        let path = connection[5..]
            .trim_start_matches('"')
            .trim_end_matches(';')
            .trim_end_matches('"');
        if !path.is_empty() { Some(path.to_string()) } else { None }
    } else {
        None
    }
}
```

**When deleting an InfoBase entry:**
1. Create backup of `ibases.v8i`
2. If it’s a file database AND the path is local (not UNC `\\...`): delete the directory
3. Remove the `[SectionName]` block from the file
4. Write the modified file back

### Directory Size Calculation

```rust
use walkdir::WalkDir;

pub fn get_dir_size(path: &std::path::Path) -> u64 {
    WalkDir::new(path)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter_map(|e| e.metadata().ok())
        .filter(|m| m.is_file())
        .map(|m| m.len())
        .sum()
}
```

### Size Formatting

```rust
pub fn format_size(bytes: u64) -> String {
    const GB: u64 = 1024 * 1024 * 1024;
    const MB: u64 = 1024 * 1024;
    const KB: u64 = 1024;
    if bytes >= GB {
        format!("{:.1} ГБ", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} МБ", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} КБ", bytes as f64 / KB as f64)
    } else {
        format!("{} Б", bytes)
    }
}
```

## Data Models

```rust
// src/models.rs

#[derive(Debug, Clone)]
pub struct InstalledVersion {
    pub name: String,
    pub version: String,
    pub version_int: u64,      // for sorting: padded version parts concatenated
    pub uuid: String,          // registry subkey name (used for msiexec /x)
    pub location: String,      // install directory
    pub install_date: String,  // formatted as "yyyy-MM-dd"
    pub size: u64,             // directory size in bytes
    pub selected: bool,
}

#[derive(Debug, Clone)]
pub struct CacheEntry {
    pub path: String,
    pub uuid: String,           // directory name (GUID)
    pub display_name: String,   // from InfoBase match, or "<База не найдена>"
    pub size: u64,
    pub selected: bool,         // true by default if no matching InfoBase
}

#[derive(Debug, Clone)]
pub struct InfoBase {
    pub name: String,
    pub uuid: String,
    pub version: Option<String>,
    pub connection: String,
    pub is_file_base: bool,     // connection starts with "File="
    pub size: u64,              // 0 for server bases
    pub selected: bool,
}
```

## Code Style

- **Language**: All user-facing strings in Russian
- **Error handling**: Use `anyhow::Result` in platform layer, display errors in UI status area
- **No `unwrap()`** in production code, use `?` or `.ok()` with fallbacks
- **Async**: Use `cx.spawn()` for background work, `cx.background_executor().spawn()` for blocking I/O
- **Naming**: snake_case for everything, descriptive names
- **No clippy warnings**: code must pass `cargo clippy -- --deny warnings`
- **Imports**: prefer explicit imports over glob imports, except `gpui::*` and `gpui_component::*`

## Important Constraints

1. **Root is mandatory**: Every window’s first view must be `cx.new(|cx| Root::new(view, window, cx))`
2. **`gpui_component::init(cx)` must be called before any component usage**
3. **`cx.notify()`** must be called after any state mutation to trigger re-render
4. **Checkbox `on_click`** receives `&bool` = NEW checked state, not a click event
5. **Destructive operations** (uninstall, delete) must be guarded: disabled if nothing selected, and `operation_in_progress` flag prevents double-execution
6. **ibases.v8i backup** must be created before any modifications
7. **UNC paths** (`\\server\share`) must NOT be deleted even for file databases
8. **Directory size** calculation can be slow for large dirs — run in `background_executor`
