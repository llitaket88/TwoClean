mod app;
mod models;
mod platform;

use gpui_kit::component::{Root, TitleBar};
use gpui_kit::*;

fn main() {
    application().with_assets(assets::Assets).run(|cx| {
        init(cx);

        let window_options = WindowOptions {
            titlebar: Some(TitleBar::title_bar_options()),
            window_bounds: Some(WindowBounds::centered(size(px(900.), px(640.)), cx)),
            ..Default::default()
        };

        open_window(window_options, cx, |window, cx| {
            window.set_window_title("TwoClear — Очистка 1С");
            let view = cx.new(|cx| app::TwoClearApp::new(window, cx));
            cx.new(|cx| Root::new(view, window, cx))
        })
        .expect("Failed to open window");
    });
}
