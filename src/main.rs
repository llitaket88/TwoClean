mod app;
mod models;
mod platform;

use gpui::*;
use gpui_component::{Root, TitleBar};

fn main() {
    gpui_platform::application()
        .with_assets(gpui_component_assets::Assets)
        .run(|cx| {
            gpui_component::init(cx);

            let window_options = WindowOptions {
                titlebar: Some(TitleBar::title_bar_options()),
                window_bounds: Some(WindowBounds::centered(size(px(900.), px(640.)), cx)),
                ..Default::default()
            };

            cx.spawn(async move |cx| {
                cx.open_window(window_options, |window, cx| {
                    window.set_window_title("TwoClear — Очистка 1С");
                    let view = cx.new(|cx| app::TwoClearApp::new(window, cx));
                    cx.new(|cx| Root::new(view, window, cx))
                })
                .expect("Failed to open window");
            })
            .detach();
        });
}
