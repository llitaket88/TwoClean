use gpui::*;
use gpui_component::{ActiveTheme, Sizable, TitleBar, h_flex, v_flex};
use gpui_component::tab::{Tab, TabBar};
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::Disableable;
use gpui_component::checkbox::Checkbox;
use gpui_component::spinner::Spinner;
use gpui::prelude::FluentBuilder as _;
use crate::{models::*, platform};
use crate::platform::format_size;

pub struct TwoClearApp {
    active_tab: usize,
    versions: Vec<InstalledVersion>,
    versions_loading: bool,
    cache_entries: Vec<CacheEntry>,
    cache_loading: bool,
    infobases: Vec<InfoBase>,
    infobases_loading: bool,
    operation_in_progress: bool,
    status_message: Option<String>,
    error_message: Option<String>,
}

impl TwoClearApp {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut app = Self {
            active_tab: 0,
            versions: vec![],
            versions_loading: false,
            cache_entries: vec![],
            cache_loading: false,
            infobases: vec![],
            infobases_loading: false,
            operation_in_progress: false,
            status_message: None,
            error_message: None,
        };
        app.load_all_data(cx);
        app
    }

    fn load_all_data(&mut self, cx: &mut Context<Self>) {
        self.load_versions(cx);
        self.load_infobases_and_cache(cx);
    }

    fn load_versions(&mut self, cx: &mut Context<Self>) {
        self.versions_loading = true;
        self.error_message = None;
        cx.notify();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { platform::get_installed_versions() })
                .await;
            this.update(cx, |app, cx| {
                match result {
                    Ok(v) => app.versions = v,
                    Err(e) => app.error_message = Some(e.to_string()),
                }
                app.versions_loading = false;
                cx.notify();
            })
            .ok();
        })
        .detach();
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
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn start_uninstall(&mut self, cx: &mut Context<Self>) {
        self.operation_in_progress = true;
        cx.notify();

        let to_delete: Vec<String> = self
            .versions
            .iter()
            .filter(|v| v.selected)
            .map(|v| v.uuid.clone())
            .collect();
        let total = to_delete.len();

        cx.spawn(async move |this, cx| {
            for (i, uuid) in to_delete.iter().enumerate() {
                this.update(cx, |app, cx| {
                    app.status_message = Some(format!("Удаление {} из {}...", i + 1, total));
                    cx.notify();
                })
                .ok();

                let uuid_clone = uuid.clone();
                let result = cx
                    .background_executor()
                    .spawn(async move { platform::uninstall_version(&uuid_clone) })
                    .await;

                let uuid_for_retain = uuid.clone();
                this.update(cx, |app, cx| {
                    match result {
                        Ok(true) => app.versions.retain(|v| v.uuid != uuid_for_retain),
                        Ok(false) => {
                            app.error_message =
                                Some(format!("Не удалось удалить {}", uuid_for_retain))
                        }
                        Err(e) => app.error_message = Some(e.to_string()),
                    }
                    cx.notify();
                })
                .ok();
            }

            this.update(cx, |app, cx| {
                app.operation_in_progress = false;
                app.status_message = Some(format!("Готово. Обработано версий: {}", total));
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn start_delete_cache(&mut self, cx: &mut Context<Self>) {
        self.operation_in_progress = true;
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

    fn start_delete_infobases(&mut self, cx: &mut Context<Self>) {
        self.operation_in_progress = true;
        cx.notify();

        let names: Vec<String> = self
            .infobases
            .iter()
            .filter(|ib| ib.selected)
            .map(|ib| ib.name.clone())
            .collect();
        let count = names.len();

        cx.spawn(async move |this, cx| {
            let names_clone = names.clone();
            let result = cx
                .background_executor()
                .spawn(async move { platform::delete_info_bases(&names_clone) })
                .await;

            this.update(cx, |app, cx| {
                match result {
                    Ok(_) => {
                        app.infobases.retain(|ib| !names.contains(&ib.name));
                        app.status_message = Some(format!("Удалено {} баз из списка", count));
                    }
                    Err(e) => app.error_message = Some(e.to_string()),
                }
                app.operation_in_progress = false;
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn render_title_bar(&self, cx: &Context<Self>) -> impl IntoElement {
        TitleBar::new().child(
            TabBar::new("main-tabs")
                .selected_index(self.active_tab)
                .on_click(cx.listener(|this, ix: &usize, _window, cx| {
                    this.active_tab = *ix;
                    cx.notify();
                }))
                .child(Tab::new().label("Версии платформы"))
                .child(Tab::new().label("Кэш метаданных"))
                .child(Tab::new().label("Информационные базы")),
        )
    }

    fn render_status_bar(&self, cx: &Context<Self>) -> impl IntoElement {
        h_flex()
            .h_7()
            .px_3()
            .gap_2()
            .items_center()
            .border_t_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().tab_bar)
            .child(
                div()
                    .flex_1()
                    .text_sm()
                    .map(|this| {
                        if let Some(err) = &self.error_message {
                            this.text_color(cx.theme().danger).child(err.clone())
                        } else if let Some(msg) = &self.status_message {
                            this.text_color(cx.theme().muted_foreground).child(msg.clone())
                        } else {
                            this
                        }
                    }),
            )
    }

    fn render_empty_state(message: impl Into<String>, cx: &Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .text_color(cx.theme().muted_foreground)
                    .child(message.into()),
            )
    }

    fn render_versions_tab(&self, cx: &Context<Self>) -> impl IntoElement {
        let total_size: u64 = self.versions.iter().map(|v| v.size).sum();
        let sel_count = self.versions.iter().filter(|v| v.selected).count();
        let sel_size: u64 = self
            .versions
            .iter()
            .filter(|v| v.selected)
            .map(|v| v.size)
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
                        Button::new("ver-select-all")
                            .label("Выбрать все")
                            .small()
                            .ghost()
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.versions.iter_mut().for_each(|v| v.selected = true);
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("ver-deselect-all")
                            .label("Снять выбор")
                            .small()
                            .ghost()
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.versions.iter_mut().for_each(|v| v.selected = false);
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("ver-reload")
                            .label("Обновить")
                            .small()
                            .ghost()
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.load_versions(cx);
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
                        Button::new("ver-delete")
                            .label("Удалить выбранные")
                            .small()
                            .primary()
                            .disabled(sel_count == 0 || self.operation_in_progress)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.start_uninstall(cx);
                            })),
                    ),
            )
            .map(|this| {
                if self.versions_loading {
                    this.child(
                        div()
                            .flex_1()
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(Spinner::new()),
                    )
                } else if self.versions.is_empty() {
                    this.child(Self::render_empty_state(
                        "Установленные версии 1С не найдены",
                        cx,
                    ))
                } else {
                    this.child(
                        div()
                            .id("ver-list")
                            .flex_1()
                            .overflow_y_scroll()
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
                                    .child(div().flex_1().child("Название"))
                                    .child(div().w(px(112.)).child("Версия"))
                                    .child(div().w(px(112.)).child("Дата установки"))
                                    .child(div().w_20().child("Размер")),
                            )
                            .children(self.versions.iter().enumerate().map(|(i, v)| {
                                let name = v.name.clone();
                                let version = v.version.clone();
                                let date = v.install_date.clone();
                                let size_str = format_size(v.size);
                                h_flex()
                                    .px_3()
                                    .py_2()
                                    .gap_3()
                                    .border_b_1()
                                    .border_color(cx.theme().border)
                                    .items_center()
                                    .child(
                                        Checkbox::new(format!("ver-cb-{}", i))
                                            .checked(v.selected)
                                            .on_click(cx.listener(
                                                move |this, checked: &bool, _, cx| {
                                                    if i < this.versions.len() {
                                                        this.versions[i].selected = *checked;
                                                        cx.notify();
                                                    }
                                                },
                                            )),
                                    )
                                    .child(div().flex_1().child(name))
                                    .child(div().w(px(112.)).text_sm().child(version))
                                    .child(
                                        div()
                                            .w(px(112.))
                                            .text_sm()
                                            .text_color(cx.theme().muted_foreground)
                                            .child(date),
                                    )
                                    .child(div().w_20().text_sm().child(size_str))
                            })),
                    )
                }
            })
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
                                this.cache_entries.iter_mut().for_each(|e| e.selected = true);
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
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.cache_entries.iter_mut().for_each(|e| {
                                    e.selected = e.display_name == "<База не найдена>";
                                });
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("cache-reload")
                            .label("Обновить")
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
                            .label("Удалить кэш")
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
                                h_flex()
                                    .px_3()
                                    .py_1()
                                    .gap_3()
                                    .border_b_1()
                                    .border_color(cx.theme().border)
                                    .text_sm()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(div().w_5())
                                    .child(div().flex_1().child("База"))
                                    .child(div().w_64().child("UUID"))
                                    .child(div().w_20().child("Размер")),
                            )
                            .children(self.cache_entries.iter().enumerate().map(|(i, e)| {
                                let display_name = e.display_name.clone();
                                let uuid = e.uuid.clone();
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
                                    .child(div().flex_1().child(display_name))
                                    .child(
                                        div()
                                            .w_64()
                                            .text_xs()
                                            .text_color(cx.theme().muted_foreground)
                                            .overflow_hidden()
                                            .child(uuid),
                                    )
                                    .child(div().w_20().text_sm().child(size_str))
                            })),
                    )
                }
            })
    }

    fn render_infobases_tab(&self, cx: &Context<Self>) -> impl IntoElement {
        let total_size: u64 = self.infobases.iter().map(|ib| ib.size).sum();
        let sel_count = self.infobases.iter().filter(|ib| ib.selected).count();
        let sel_size: u64 = self
            .infobases
            .iter()
            .filter(|ib| ib.selected)
            .map(|ib| ib.size)
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
                        Button::new("ib-select-all")
                            .label("Выбрать все")
                            .small()
                            .ghost()
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.infobases.iter_mut().for_each(|ib| ib.selected = true);
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("ib-deselect-all")
                            .label("Снять выбор")
                            .small()
                            .ghost()
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.infobases.iter_mut().for_each(|ib| ib.selected = false);
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("ib-reload")
                            .label("Обновить")
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
                        Button::new("ib-delete")
                            .label("Удалить из списка")
                            .small()
                            .primary()
                            .disabled(sel_count == 0 || self.operation_in_progress)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.start_delete_infobases(cx);
                            })),
                    ),
            )
            .map(|this| {
                if self.infobases_loading {
                    this.child(
                        div()
                            .flex_1()
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(Spinner::new()),
                    )
                } else if self.infobases.is_empty() {
                    this.child(Self::render_empty_state(
                        "Список информационных баз пуст",
                        cx,
                    ))
                } else {
                    this.child(
                        div()
                            .id("ib-list")
                            .flex_1()
                            .overflow_y_scroll()
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
                                    .child(div().flex_1().child("Название"))
                                    .child(div().w_24().child("Тип"))
                                    .child(div().w_24().child("Версия"))
                                    .child(div().w_48().child("Подключение"))
                                    .child(div().w_20().child("Размер")),
                            )
                            .children(self.infobases.iter().enumerate().map(|(i, ib)| {
                                let name = ib.name.clone();
                                let type_str = if ib.is_file_base {
                                    "Файловая"
                                } else {
                                    "Серверная"
                                }
                                .to_string();
                                let ver_str = ib.version.clone().unwrap_or_default();
                                let conn_str = ib.connection.clone();
                                let size_str = if ib.size == 0 {
                                    "—".to_string()
                                } else {
                                    format_size(ib.size)
                                };
                                h_flex()
                                    .px_3()
                                    .py_2()
                                    .gap_3()
                                    .border_b_1()
                                    .border_color(cx.theme().border)
                                    .items_center()
                                    .child(
                                        Checkbox::new(format!("ib-cb-{}", i))
                                            .checked(ib.selected)
                                            .on_click(cx.listener(
                                                move |this, checked: &bool, _, cx| {
                                                    if i < this.infobases.len() {
                                                        this.infobases[i].selected = *checked;
                                                        cx.notify();
                                                    }
                                                },
                                            )),
                                    )
                                    .child(div().flex_1().child(name))
                                    .child(div().w_24().text_sm().child(type_str))
                                    .child(
                                        div()
                                            .w_24()
                                            .text_sm()
                                            .text_color(cx.theme().muted_foreground)
                                            .child(ver_str),
                                    )
                                    .child(
                                        div()
                                            .w_48()
                                            .text_xs()
                                            .text_color(cx.theme().muted_foreground)
                                            .overflow_hidden()
                                            .child(conn_str),
                                    )
                                    .child(div().w_20().text_sm().child(size_str))
                            })),
                    )
                }
            })
    }
}

impl Render for TwoClearApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
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
            .child(self.render_status_bar(cx))
    }
}
