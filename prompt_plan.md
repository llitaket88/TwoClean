# prompt_plan.md — Пошаговый план реализации TwoClear

Этот файл содержит серию промптов для кодогенерирующей LLM.
Каждый шаг строится на предыдущем. Выполняй последовательно, проверяй `cargo build` после каждого шага.

---

## Фаза 1: Основа проекта

### Шаг 1.1 — Cargo.toml и структура директорий

```text
Настрой Cargo.toml для проекта TwoClear (Windows-утилита на Rust 2024 edition, GPUI + gpui-component).

Зависимости в [dependencies]:
  gpui          = { git = "https://github.com/zed-industries/zed" }
  gpui_platform = { git = "https://github.com/zed-industries/zed", features = ["font-kit", "runtime_shaders"] }
  gpui-component        = { git = "https://github.com/longbridge/gpui-component", package = "gpui-component" }
  gpui-component-assets = { git = "https://github.com/longbridge/gpui-component", package = "gpui-component-assets" }
  anyhow   = "1"
  smol     = "2"
  winreg   = "0.10"
  walkdir  = "2"

Зависимости в [target.'cfg(target_os = "windows")'.dependencies]:
  windows = { version = "0.58", features = [
    "Win32_Graphics_Direct3D",
    "Win32_Graphics_Direct3D11",
    "Win32_Graphics_Dxgi",
  ] }

Создай пустые файлы-заглушки (с минимальным валидным содержимым):
  src/main.rs              — fn main() {}
  src/app.rs               — пусто
  src/models.rs            — пусто
  src/platform/mod.rs      — pub mod utils; pub mod versions; pub mod cache; pub mod infobases;
  src/platform/utils.rs    — пусто
  src/platform/versions.rs — пусто
  src/platform/cache.rs    — пусто
  src/platform/infobases.rs — пусто

Убедись, что `cargo build` проходит без ошибок.
```

### Шаг 1.2 — Модели данных (src/models.rs)

```text
Реализуй src/models.rs. Все структуры с #[derive(Debug, Clone)].

pub struct InstalledVersion {
    pub name: String,
    pub version: String,
    pub version_int: u64,     // для сортировки: части версии с padding конкатенированы в u64
    pub uuid: String,         // имя раздела реестра (используется в msiexec /x{uuid})
    pub location: String,     // путь установки
    pub install_date: String, // "yyyy-MM-dd"
    pub size: u64,            // размер директории в байтах
    pub selected: bool,
}

pub struct CacheEntry {
    pub path: String,
    pub uuid: String,           // имя директории (GUID)
    pub display_name: String,   // совпадение с InfoBase.name по UUID, иначе "<База не найдена>"
    pub size: u64,
    pub selected: bool,         // true по умолчанию если нет совпадения с InfoBase
}

pub struct InfoBase {
    pub name: String,
    pub uuid: String,
    pub version: Option<String>,
    pub connection: String,
    pub is_file_base: bool,   // true если Connect начинается с "File="
    pub size: u64,            // размер директории для файловых баз, 0 для серверных
    pub selected: bool,
}
```

---

## Фаза 2: Платформенный слой

### Шаг 2.1 — Утилиты (src/platform/utils.rs)

```text
Реализуй src/platform/utils.rs:

1. pub fn get_dir_size(path: &std::path::Path) -> u64
   Использует walkdir::WalkDir. Считает сумму размеров всех файлов рекурсивно.
   Не следует символическим ссылкам (follow_links = false).
   Игнорирует любые ошибки доступа через filter_map(|e| e.ok()).

2. pub fn format_size(bytes: u64) -> String
   >= 1 073 741 824  ->  "{:.1} ГБ"
   >=     1 048 576  ->  "{:.1} МБ"
   >=         1 024  ->  "{:.1} КБ"
   иначе             ->  "{} Б"

3. pub fn parse_install_date(s: &str) -> String
   Принимает "yyyyMMdd", возвращает "yyyy-MM-dd".
   Если длина != 8 или формат неверный — вернуть исходную строку.
```

### Шаг 2.2 — Версии платформы (src/platform/versions.rs)

```text
Реализуй src/platform/versions.rs.

use winreg::{RegKey, enums::*};
use crate::{models::InstalledVersion, platform::utils::{get_dir_size, parse_install_date}};

pub fn get_installed_versions() -> anyhow::Result<Vec<InstalledVersion>>

Алгоритм:
1. Открыть HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall
   с флагами KEY_READ | KEY_WOW64_32KEY
2. На 64-bit OS добавить ещё KEY_READ | KEY_WOW64_64KEY
3. Для каждого подраздела прочитать значения (пропускать при ошибке через continue):
   - DisplayName    : String  (пропустить если пустое)
   - Publisher      : String  (пропустить если не "1С-Софт" / "1C-Soft" / "1C" / "1С")
   - DisplayVersion : String  (default "0.0.0.0")
   - InstallLocation: String  (пропустить если пустое)
   - InstallDate    : String  (default "00010101")
4. version_int: split('.'), 4 части, каждую parse::<u64>(),
   итог = v[0]*10^10 + v[1]*10^8 + v[2]*10^5 + v[3]
5. size = get_dir_size(Path::new(&location))
6. install_date = parse_install_date(&date_str)
7. Дедупликация: пропустить если UUID уже есть в результирующем векторе
8. В конце: sort_by_key(|v| v.version_int)

pub fn uninstall_version(uuid: &str) -> anyhow::Result<bool>
  std::process::Command::new("msiexec.exe")
    .args(["/x", &format!("{}", uuid), "/q"])   // формат: /x{uuid}
    .status()
    -> Ok(status.success())
```

### Шаг 2.3 — Кэш метаданных (src/platform/cache.rs)

```text
Реализуй src/platform/cache.rs.

use crate::{models::CacheEntry, platform::utils::get_dir_size};

pub fn get_cache_entries() -> anyhow::Result<Vec<CacheEntry>>

Алгоритм:
1. local_app_data = std::env::var("LOCALAPPDATA")?
2. base_path = PathBuf::from(local_app_data).join("1C")
3. Если base_path не существует — вернуть Ok(vec![])
4. Перебрать записи в base_path, взять только директории чьё имя начинается с "1cv8"
5. Внутри каждой такой директории перебрать поддиректории.
   UUID-директория: имя полностью совпадает с regex
   [0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}
   (используй once_cell::sync::Lazy или просто Regex::new в теле функции)
6. Для каждой UUID-директории:
   CacheEntry {
     path: полный путь (to_string_lossy().to_string()),
     uuid: имя директории,
     display_name: "<База не найдена>".to_string(), // временно
     size: get_dir_size(&dir_path),
     selected: false, // временно
   }

pub fn delete_cache_entry(path: &str) -> anyhow::Result<()>
  std::fs::remove_dir_all(path).map_err(Into::into)

Примечание: поля display_name и selected будут скорректированы в app.rs
после загрузки infobases, путём сопоставления по UUID.
```

### Шаг 2.4 — Информационные базы (src/platform/infobases.rs)

```text
Реализуй src/platform/infobases.rs.

use crate::{models::InfoBase, platform::utils::get_dir_size};

pub fn get_infobases_path() -> anyhow::Result<std::path::PathBuf>
  appdata = std::env::var("APPDATA")?
  path = PathBuf::from(appdata).join("1C").join("1CEStart").join("ibases.v8i")
  if !path.exists() -> anyhow::bail!("Файл не найден: {:?}", path)
  Ok(path)

pub fn get_info_bases() -> anyhow::Result<Vec<InfoBase>>
  Читает файл get_infobases_path()? и парсит INI-подобный формат:

  Формат файла:
    [Имя базы]
    Connect=File="C:\path";
    ID={GUID}
    DefaultVersion=8.3.20

  Алгоритм парсинга:
    Итерировать по строкам. Если строка начинается с '[' и заканчивается на ']':
      сохранить текущую секцию, начать новую HashMap<String,String>.
    Если строка содержит '=': split на ключ/значение (только первый '='),
      key.trim().to_lowercase() -> value.trim().to_string()
    После обхода всех строк: обработать последнюю секцию.
    Из каждой секции собирать InfoBase только если присутствует ключ "connect".

  is_file_base = connection.to_ascii_lowercase().starts_with("file=")
  uuid  = keys["id"] (trim фигурных скобок если нужно)
  version = keys["defaultversion"].or(keys["version"])
  size: если is_file_base и путь существует -> get_dir_size, иначе 0

fn extract_file_path(connection: &str) -> Option<String>
  // Формат: File="C:\path"; или File="C:\path"
  if connection.to_ascii_lowercase().starts_with("file=") {
    let s = connection[5..]         // убираем "File="
      .trim_start_matches('"')
      .trim_end_matches(';')
      .trim_end_matches('"');
    if !s.is_empty() { Some(s.to_string()) } else { None }
  } else { None }

pub fn delete_info_bases(names: &[String]) -> anyhow::Result<()>
  1. path = get_infobases_path()?
  2. Создать бэкап:
     timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()
     backup = path.parent().unwrap().join(format!("ibases.v8i_backup_{}", timestamp))
     std::fs::copy(&path, &backup)?
  3. Прочитать файл: content = std::fs::read_to_string(&path)?
  4. Для каждого name в names:
     Если в соответствующей секции connect является файловой базой
     И путь НЕ начинается с "\\\\" (UNC):
       -> std::fs::remove_dir_all(path) (игнорировать ошибки через .ok())
  5. Переписать файл без удалённых секций:
     Итерировать по строкам, отслеживать текущую секцию,
     пропускать строки секций из names.
     std::fs::write(&path, new_content.as_bytes())?
```

### Шаг 2.5 — Реэкспорт (src/platform/mod.rs)

```text
Реализуй src/platform/mod.rs:

pub mod utils;
pub mod versions;
pub mod cache;
pub mod infobases;

pub use utils::{format_size, get_dir_size};
pub use versions::{get_installed_versions, uninstall_version};
pub use cache::{get_cache_entries, delete_cache_entry};
pub use infobases::{get_info_bases, delete_info_bases};
```

---

## Фаза 3: UI

### Шаг 3.1 — Точка входа (src/main.rs)

```text
Реализуй src/main.rs.

use gpui::*;
use gpui_component::{Root, TitleBar};

fn main() {
    gpui_platform::application()
        .with_assets(gpui_component_assets::Assets)
        .run(|cx| {
            gpui_component::init(cx);  // ОБЯЗАТЕЛЬНО первым

            let window_options = WindowOptions {
                titlebar: Some(TitleBar::title_bar_options()),
                window_bounds: Some(WindowBounds::centered(size(px(900.), px(640.)), cx)),
                ..Default::default()
            };

            cx.spawn(async move |cx| {
                cx.open_window(window_options, |window, cx| {
                    window.set_window_title("TwoClear — Очистка 1С");
                    let view = cx.new(|cx| app::TwoClearApp::new(window, cx));
                    cx.new(|cx| Root::new(view, window, cx))  // Root ОБЯЗАТЕЛЕН
                })
                .expect("Failed to open window");
            })
            .detach();
        });
}
```

### Шаг 3.2 — Структура приложения (src/app.rs)

```text
Реализуй src/app.rs.

Импорты:
  use gpui::*;
  use gpui_component::{ActiveTheme, v_flex, h_flex, TitleBar};
  use gpui_component::tab::{Tab, TabBar};
  use gpui_component::button::Button;
  use gpui_component::checkbox::Checkbox;
  use gpui_component::spinner::Spinner;
  use gpui::prelude::FluentBuilder as _;
  use crate::{models::*, platform};

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

    fn load_versions(&mut self, cx: &mut Context<Self>) { /* async load */ }
    fn load_infobases_and_cache(&mut self, cx: &mut Context<Self>) { /* async load, затем сопоставление UUID */ }
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
```

### Шаг 3.3 — Загрузка данных (дополнение к app.rs)

```text
Реализуй методы загрузки данных в TwoClearApp.

fn load_versions(&mut self, cx: &mut Context<Self>) {
    self.versions_loading = true;
    self.error_message = None;
    cx.notify();
    cx.spawn(async move |this, mut cx| {
        let result = cx
            .background_executor()
            .spawn(async move { platform::get_installed_versions() })
            .await;
        this.update(&mut cx, |app, cx| {
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
    cx.spawn(async move |this, mut cx| {
        let infobases_result = cx
            .background_executor()
            .spawn(async move { platform::get_info_bases() })
            .await;
        let cache_result = cx
            .background_executor()
            .spawn(async move { platform::get_cache_entries() })
            .await;

        this.update(&mut cx, |app, cx| {
            let infobases = match infobases_result {
                Ok(v) => v,
                Err(e) => { app.error_message = Some(e.to_string()); vec![] }
            };
            let mut cache = match cache_result {
                Ok(v) => v,
                Err(e) => { app.error_message = Some(e.to_string()); vec![] }
            };

            // Сопоставление UUID кэша с InfoBase
            for entry in &mut cache {
                if let Some(ib) = infobases.iter().find(|ib| ib.uuid == entry.uuid) {
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
```

### Шаг 3.4 — TitleBar и вкладки (дополнение к app.rs)

```text
Добавь в TwoClearApp:

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
                        this.text_color(cx.theme().destructive).child(err.clone())
                    } else if let Some(msg) = &self.status_message {
                        this.text_color(cx.theme().muted_foreground).child(msg.clone())
                    } else {
                        this
                    }
                }),
        )
}
```

### Шаг 3.5 — Вкладка «Версии платформы» (дополнение к app.rs)

```text
Добавь в TwoClearApp рендер вкладки версий.

fn render_versions_tab(&self, cx: &Context<Self>) -> impl IntoElement {
    let total_size: u64 = self.versions.iter().map(|v| v.size).sum();
    let sel_count = self.versions.iter().filter(|v| v.selected).count();
    let sel_size: u64 = self.versions.iter().filter(|v| v.selected).map(|v| v.size).sum();

    v_flex()
        .size_full()
        .child(/* toolbar */)
        .child(/* список или spinner */)
}

Тулбар (h_flex, px_3, py_2, gap_2, border_b_1, items_center):
  - Button "Выбрать все" .small().ghost() -> iter_mut().for_each(|v| v.selected = true)
  - Button "Снять выбор" .small().ghost() -> iter_mut().for_each(|v| v.selected = false)
  - Button "Обновить"    .small().ghost() -> load_versions(cx)
  - div().flex_1()  // spacer
  - div().text_sm().text_color(muted) -> "Выбрано: N (X МБ) / Всего: Y МБ"
  - Button "Удалить выбранные" .small().primary()
      .disabled(sel_count == 0 || self.operation_in_progress)
      -> start_uninstall(cx)

Список (div().flex_1().overflow_y_scroll()):
  Заголовок-строка: h_flex, px_3, py_1, border_b_1, text_sm, muted_foreground
    "Название" (flex_1) | "Версия" (w_28) | "Дата" (w_28) | "Размер" (w_20, text_right)

  Строки (enumerate):
    h_flex().px_3().py_2().gap_3().border_b_1().items_center()
      .child(Checkbox::new(format!("ver-{}", i)).checked(v.selected)
          .on_click(cx.listener(move |this, checked, _, cx| {
              this.versions[i].selected = *checked;
              cx.notify();
          })))
      .child(div().flex_1().child(v.name.clone()))
      .child(div().w_28().text_sm().child(v.version.clone()))
      .child(div().w_28().text_sm().text_color(muted).child(v.install_date.clone()))
      .child(div().w_20().text_sm().text_right().child(format_size(v.size)))

Если versions_loading — показать spinner по центру вместо списка.
Если versions пустой и не loading — показать "Установленные версии 1С не найдены" по центру.
```

### Шаг 3.6 — Вкладка «Кэш метаданных» (дополнение к app.rs)

```text
Добавь render_cache_tab аналогично шагу 3.5.

Тулбар (дополнительно):
  - Button "Выбрать осиротевшие" .small().ghost()
    -> cache_entries.iter_mut()
         .for_each(|e| e.selected = e.display_name == "<База не найдена>")

Строки:
  Checkbox + display_name (flex_1) + uuid (w_64, text_xs, muted, truncate) + size (w_20, text_right)

Кнопка действия: "Удалить кэш" -> start_delete_cache(cx)

fn start_delete_cache(&mut self, cx: &mut Context<Self>)
  operation_in_progress = true; cx.notify()
  Собрать Vec<String> путей выбранных записей
  cx.spawn(async |this, cx| {
    для каждого пути: cx.background_executor().spawn(|| delete_cache_entry(&path)).await
    this.update(cx, |app, cx| {
      app.cache_entries.retain(|e| !paths.contains(&e.path));
      app.operation_in_progress = false;
      app.status_message = Some(format!("Удалено {} записей кэша", count));
      cx.notify();
    }).ok()
  }).detach()

Пустое состояние: "Кэш метаданных 1С пуст"
```

### Шаг 3.7 — Вкладка «Информационные базы» (дополнение к app.rs)

```text
Добавь render_infobases_tab аналогично шагу 3.5.

Строки:
  Checkbox
  + name (flex_1)
  + тип: div().w_24().text_sm() -> если is_file_base "Файловая" иначе "Серверная"
  + version (w_24, text_sm, muted)
  + краткий путь/сервер из connection (w_48, text_xs, muted, truncate)
  + size (w_20, text_sm, text_right) — "—" если 0

Кнопка действия: "Удалить из списка" -> start_delete_infobases(cx)

fn start_delete_infobases(&mut self, cx: &mut Context<Self>)
  operation_in_progress = true; cx.notify()
  Собрать Vec<String> имён выбранных InfoBase
  cx.spawn(async |this, cx| {
    let result = cx.background_executor()
      .spawn(async move { platform::delete_info_bases(&names) }).await;
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
    }).ok()
  }).detach()

Пустое состояние: "Список информационных баз пуст"
Если ibases.v8i не найден — показать ошибку из error_message.
```

---

## Фаза 4: Операции

### Шаг 4.1 — Удаление версий платформы с прогрессом

```text
Реализуй fn start_uninstall(&mut self, cx: &mut Context<Self>) в TwoClearApp:

self.operation_in_progress = true;
cx.notify();

let to_delete: Vec<String> = self.versions
    .iter()
    .filter(|v| v.selected)
    .map(|v| v.uuid.clone())
    .collect();
let total = to_delete.len();

cx.spawn(async move |this, mut cx| {
    for (i, uuid) in to_delete.iter().enumerate() {
        this.update(&mut cx, |app, cx| {
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
        this.update(&mut cx, |app, cx| {
            match result {
                Ok(true) => app.versions.retain(|v| v.uuid != uuid_for_retain),
                Ok(false) => {
                    app.error_message = Some(format!("Не удалось удалить {}", uuid_for_retain))
                }
                Err(e) => app.error_message = Some(e.to_string()),
            }
            cx.notify();
        })
        .ok();
    }

    this.update(&mut cx, |app, cx| {
        app.operation_in_progress = false;
        app.status_message = Some(format!("Готово. Обработано версий: {}", total));
        cx.notify();
    })
    .ok();
})
.detach();
```

---

## Фаза 5: Полировка

### Шаг 5.1 — Вспомогательный render_empty_state

```text
Добавь в TwoClearApp:

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

Используй его во всех трёх вкладках вместо повторяющегося кода.
```

### Шаг 5.2 — Финальная проверка

```text
Выполни следующие проверки и исправь все найденные проблемы:

1. cargo clippy -- --deny warnings   → нулевые предупреждения
2. cargo fmt                         → код отформатирован
3. Все пользовательские строки на русском языке
4. Нет unwrap() в производственном коде (только в тестах)
5. operation_in_progress сбрасывается в false при любом исходе операции (Ok и Err)
6. Кнопки деструктивных операций: disabled(sel_count == 0 || self.operation_in_progress)
7. Бэкап ibases.v8i создаётся до любых изменений файла
8. UNC-пути (начинающиеся с \\) не удаляются физически
9. cargo build проходит без ошибок
```

---

## Порядок выполнения

```
1.1 → 1.2
→ 2.1 → 2.2 → 2.3 → 2.4 → 2.5
→ 3.1 → 3.2 → 3.3 → 3.4 → 3.5 → 3.6 → 3.7
→ 4.1
→ 5.1 → 5.2
```

**Правило:** после каждого шага `cargo build` должен проходить без ошибок компиляции.