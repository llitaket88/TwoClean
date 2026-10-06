#![windows_subsystem = "windows"]

mod app;
mod models;
mod platform;

use gpui_kit::component::{Root, Theme, TitleBar};
use gpui_kit::*;

fn main() {
    let Some(_instance) = platform::SingleInstance::new("TwoClean_UniqueMutex") else {
        platform::show_already_running_message();
        return;
    };

    application().with_assets(assets::Assets).run(|cx| {
        init(cx);

        let window_options = WindowOptions {
            titlebar: Some(TitleBar::title_bar_options()),
            window_bounds: Some(WindowBounds::centered(size(px(1024.), px(640.)), cx)),
            window_min_size: Some(size(px(1024.), px(640.))),
            ..Default::default()
        };

        open_window(window_options, cx, |window, cx| {
            Theme::sync_system_appearance(Some(window), cx);
            window.set_window_title("TwoClean - Очистка кэша 1С");
            window.activate_window();

            let view = cx.new(|cx| app::TwoCleanApp::new(window, cx));
            cx.new(|cx| Root::new(view, window, cx))
        })
        .expect("Failed to open window");
    });
}
