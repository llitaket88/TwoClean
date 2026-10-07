use crate::platform::{check_updates, format_size};
use crate::{models::*, platform};
use gpui_kit::component::alert::Alert;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::checkbox::Checkbox;
use gpui_kit::component::empty::{Empty, EmptyHeader, EmptyMedia, EmptyMediaVariant, EmptyTitle};
use gpui_kit::component::popover::Popover;
use gpui_kit::component::separator::Separator;
use gpui_kit::component::spinner::Spinner;
use gpui_kit::component::status_bar::StatusBar;
use gpui_kit::component::{ActiveTheme, Icon, Sizable, Theme, ThemeMode, TitleBar, h_flex, v_flex};
use gpui_kit::component::{Disableable, IconName};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

pub struct TwoCleanApp {
    cache_entries: Vec<CacheEntry>,
    cache_loading: bool,
    infobases: Vec<InfoBase>,
    infobases_loading: bool,
    operation_in_progress: bool,
    status_message: Option<String>,
    error_message: Option<String>,
    version: &'static str,
    new_available_version: Option<&'static str>,
}

impl TwoCleanApp {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let version = env!("CARGO_PKG_VERSION");
        let new_available_version = check_updates(version);
        let mut app = Self {
            cache_entries: vec![],
            cache_loading: false,
            infobases: vec![],
            infobases_loading: false,
            operation_in_progress: false,
            status_message: None,
            error_message: None,
            version,
            new_available_version,
        };
        app.load_all_data(cx);
        app
    }

    fn load_all_data(&mut self, cx: &mut Context<Self>) {
        self.load_infobases_and_cache(cx);
    }

    fn load_infobases_and_cache(&mut self, cx: &mut Context<Self>) {
        self.cache_loading = true;
        self.infobases_loading = true;
        cx.notify();
        cx.spawn(async move |this, cx| {
            let infobases_result = cx
                .background_executor()
                .spawn(async move { platform::get_info_bases() })
                .await;
            let cache_result = cx
                .background_executor()
                .spawn(async move { platform::get_cache_entries() })
                .await;

            this.update(cx, |app, cx| {
                let infobases = match infobases_result {
                    Ok(v) => v,
                    Err(e) => {
                        app.error_message = Some(e.to_string());
                        vec![]
                    }
                };
                let mut cache = match cache_result {
                    Ok(v) => v,
                    Err(e) => {
                        app.error_message = Some(e.to_string());
                        vec![]
                    }
                };

                for entry in &mut cache {
                    if let Some(ib) = infobases
                        .iter()
                        .find(|ib| ib.uuid.to_lowercase() == entry.uuid.to_lowercase())
                    {
                        entry.display_name = ib.name.clone();
                        entry.connection = ib.connection.clone();
                        entry.selected = false;
                    } else {
                        entry.display_name = "<База не найдена>".to_string();
                        entry.selected = true;
                    }
                }

                app.infobases = infobases;
                app.cache_entries = cache;
                app.infobases_loading = false;
                app.cache_loading = false;
                app.status_message = Some("Кэш загружен".to_string());
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn start_delete_cache(&mut self, cx: &mut Context<Self>) {
        if platform::has_running_1c_processes() {
            self.error_message = Some(
                "Обнаружены запущенные процессы 1С. Пожалуйста, закройте все базы и повторите попытку.".to_string(),
            );
            cx.notify();
            return;
        }

        self.operation_in_progress = true;
        self.error_message = None;
        cx.notify();

        let paths: Vec<String> = self
            .cache_entries
            .iter()
            .filter(|e| e.selected)
            .map(|e| e.path.clone())
            .collect();
        let count = paths.len();

        cx.spawn(async move |this, cx| {
            for path in &paths {
                let p = path.clone();
                let _ = cx
                    .background_executor()
                    .spawn(async move { platform::delete_cache_entry(&p) })
                    .await;
            }

            this.update(cx, |app, cx| {
                app.cache_entries.retain(|e| !paths.contains(&e.path));
                app.operation_in_progress = false;
                app.status_message = Some(format!("Удалено {} записей кэша", count));
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn render_title_bar(&self, cx: &Context<Self>) -> impl IntoElement {
        TitleBar::new().child(
            div()
                .pl_3()
                .text_sm()
                .text_color(cx.theme().muted_foreground)
                .child("TwoClean - Очистка кэша 1С"),
        )
    }

    fn render_update_notification(&self) -> impl IntoElement {
        let update = match self.new_available_version {
            Some(version) => {
                let message = format!("Доступна новая версия {}", version);
                let button_label = "Скачать";
                let button_url = "https://github.com/llitaket88/TwoClean/releases/latest";
                (message, button_label, button_url)
            }
            None => (
                "Вы используете последнюю версию".to_string(),
                "О программе",
                "https://github.com/llitaket88/TwoClean",
            ),
        };
        v_flex().gap_2().child(update.0).child(
            Button::new("update-link")
                .secondary()
                .small()
                .label(update.1)
                .on_click(|_, _, cx| {
                    cx.open_url(update.2);
                }),
        )
    }

    fn render_status_bar(&self, cx: &Context<Self>) -> impl IntoElement {
        StatusBar::new()
            .left(div().text_sm().map(|this| {
                if let Some(err) = &self.error_message {
                    this.text_color(cx.theme().danger).child(err.clone())
                } else if let Some(msg) = &self.status_message {
                    this.text_color(cx.theme().muted_foreground)
                        .child(msg.clone())
                } else {
                    this
                }
            }))
            .right("")
            .right(Separator::vertical())
            .right(
                Button::new("theme-mod-switcher")
                    .ghost()
                    .xsmall()
                    .text_color(cx.theme().muted_foreground)
                    .icon(IconName::Moon)
                    .tooltip("Переключить тему")
                    .when(cx.theme().is_dark(), |el| el.icon(IconName::Sun))
                    .on_click(cx.listener(|_, _, window, cx| {
                        let mode = match cx.theme().is_dark() {
                            true => ThemeMode::Light,
                            false => ThemeMode::Dark,
                        };
                        Theme::change(mode, Some(window), cx);
                    })),
            )
            .right(Separator::vertical())
            .right(
                Popover::new("anchored")
                    .anchor(Anchor::BottomRight)
                    .offset(px(8.))
                    .arrow(true)
                    .trigger(
                        Button::new("version")
                            .ghost()
                            .xsmall()
                            .icon(IconName::Github)
                            .tooltip("О программе")
                            .text_color(cx.theme().muted_foreground)
                            .label(format!("v{}", self.version)),
                    )
                    .child(self.render_update_notification()),
            )
    }

    fn render_empty_state(message: impl Into<String>, cx: &Context<Self>) -> impl IntoElement {
        Empty::new().header(
            EmptyHeader::new()
                .media(
                    EmptyMedia::new()
                        .with_variant(EmptyMediaVariant::Icon)
                        .child(Icon::new(IconName::Folder)),
                )
                .title(
                    EmptyTitle::new()
                        .text_color(cx.theme().muted_foreground)
                        .child(message.into()),
                ),
        )
    }

    fn render_cache_tab(&self, cx: &Context<Self>) -> impl IntoElement {
        let total_size: u64 = self.cache_entries.iter().map(|e| e.size).sum();
        let sel_count = self.cache_entries.iter().filter(|e| e.selected).count();
        let sel_size: u64 = self
            .cache_entries
            .iter()
            .filter(|e| e.selected)
            .map(|e| e.size)
            .sum();

        v_flex()
            .size_full()
            .child(
                h_flex()
                    .px_3()
                    .py_2()
                    .gap_2()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .items_center()
                    .child(
                        Button::new("cache-select-all")
                            .label("Выбрать все")
                            .small()
                            .ghost()
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.cache_entries
                                    .iter_mut()
                                    .for_each(|e| e.selected = true);
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("cache-deselect-all")
                            .label("Снять выбор")
                            .small()
                            .ghost()
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.cache_entries
                                    .iter_mut()
                                    .for_each(|e| e.selected = false);
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("cache-select-orphaned")
                            .label("Выбрать осиротевшие")
                            .small()
                            .ghost()
                            .tooltip("Неиспользуемый кэш ранее присутствующих в списке баз")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.cache_entries.iter_mut().for_each(|e| {
                                    e.selected = e.display_name == "<База не найдена>";
                                });
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("cache-reload")
                            .label("Обновить список")
                            .small()
                            .ghost()
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.load_infobases_and_cache(cx);
                            })),
                    )
                    .child(div().flex_1())
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(format!(
                                "Выбрано: {} ({}) / Всего: {}",
                                sel_count,
                                format_size(sel_size),
                                format_size(total_size),
                            )),
                    )
                    .child(
                        Button::new("cache-delete")
                            .label("Очистить кэш")
                            .small()
                            .primary()
                            .disabled(sel_count == 0 || self.operation_in_progress)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.start_delete_cache(cx);
                            })),
                    ),
            )
            .map(|this| {
                if self.cache_loading {
                    this.child(
                        div()
                            .flex_1()
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(Spinner::new()),
                    )
                } else if self.cache_entries.is_empty() {
                    this.child(Self::render_empty_state("Кэш метаданных 1С пуст", cx))
                } else {
                    this.child(
                        div()
                            .id("cache-list")
                            .flex_1()
                            .overflow_y_scroll()
                            .child(
                                div().p_3().child(
                                    Alert::new(
                                        "running-alert",
                                        "Для корректной очистки кэша необходимо закрыть все базы данных. Перед очисткой убедитесь, что 1С не запущена.",
                                    )
                                    .text_color(cx.theme().muted_foreground),
                                ),
                            )
                            .child(
                                h_flex()
                                    .px_3()
                                    .py_1()
                                    .gap_3()
                                    .border_b_1()
                                    .border_color(cx.theme().border)
                                    .text_sm()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(div().w_5())
                                    .child(div().w_1_2().child("Наименование информационной базы"))
                                    .child(div().w_1_2().child("Строка подключения"))
                                    .child(div().w_24().child("Размер кэша")),
                            )
                            .children(self.cache_entries.iter().enumerate().map(|(i, e)| {
                                let display_name = e.display_name.clone();
                                let connection = e.connection.clone();
                                let size_str = format_size(e.size);
                                h_flex()
                                    .px_3()
                                    .py_2()
                                    .gap_3()
                                    .border_b_1()
                                    .border_color(cx.theme().border)
                                    .items_center()
                                    .child(
                                        Checkbox::new(format!("cache-cb-{}", i))
                                            .checked(e.selected)
                                            .on_click(cx.listener(
                                                move |this, checked: &bool, _, cx| {
                                                    if i < this.cache_entries.len() {
                                                        this.cache_entries[i].selected = *checked;
                                                        cx.notify();
                                                    }
                                                },
                                            )),
                                    )
                                    .child(div().w_1_2().child(display_name))
                                    .child(div().w_1_2().child(connection))
                                    .child(div().w_24().child(size_str))
                            })),
                    )
                }
            })
    }
}

impl Render for TwoCleanApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .bg(cx.theme().background)
            .child(self.render_title_bar(cx))
            .child(
                div()
                    .flex_1()
                    .overflow_hidden()
                    .child(self.render_cache_tab(cx)),
            )
            .child(self.render_status_bar(cx))
    }
}
