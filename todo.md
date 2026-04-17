# todo.md — Чеклист реализации TwoClear

## Фаза 1: Основа проекта

### 1.1 Cargo.toml и структура
- [x] Обновить `Cargo.toml` с зависимостями: gpui, gpui_platform, gpui-component, gpui-component-assets, anyhow, smol, winreg, walkdir, windows
- [x] Создать `src/app.rs` (заглушка)
- [x] Создать `src/models.rs` (заглушка)
- [x] Создать `src/platform/mod.rs`
- [x] Создать `src/platform/utils.rs` (заглушка)
- [x] Создать `src/platform/versions.rs` (заглушка)
- [x] Создать `src/platform/cache.rs` (заглушка)
- [x] Создать `src/platform/infobases.rs` (заглушка)
- [x] `cargo build` проходит без ошибок

### 1.2 Модели данных
- [x] `InstalledVersion` — name, version, version_int, uuid, location, install_date, size, selected
- [x] `CacheEntry` — path, uuid, display_name, size, selected
- [x] `InfoBase` — name, uuid, version, connection, is_file_base, size, selected
- [x] Все структуры: `#[derive(Debug, Clone)]`
- [x] `cargo build` проходит без ошибок

---

## Фаза 2: Платформенный слой

### 2.1 Утилиты (src/platform/utils.rs)
- [x] `get_dir_size(path: &Path) -> u64` — walkdir, рекурсивно, игнорировать ошибки
- [x] `format_size(bytes: u64) -> String` — ГБ / МБ / КБ / Б на русском
- [x] `parse_install_date(s: &str) -> String` — yyyyMMdd → yyyy-MM-dd

### 2.2 Версии платформы (src/platform/versions.rs)
- [x] `get_installed_versions() -> Result<Vec<InstalledVersion>>`
- [x] Читать реестр: KEY_WOW64_32KEY и KEY_WOW64_64KEY
- [x] Фильтр Publisher: "1С-Софт" / "1C-Soft" / "1C" / "1С"
- [x] Вычислять `version_int` для сортировки
- [x] Дедупликация по UUID
- [x] Сортировка по `version_int`
- [x] `uninstall_version(uuid: &str) -> Result<bool>` — msiexec.exe /x{uuid} /q

### 2.3 Кэш метаданных (src/platform/cache.rs)
- [x] `get_cache_entries() -> Result<Vec<CacheEntry>>`
- [x] Искать директории `%LOCALAPPDATA%\1C\1cv8*\`
- [x] Фильтровать UUID-директории по regex
- [x] Считать размер каждой директории
- [x] `delete_cache_entry(path: &str) -> Result<()>`

### 2.4 Информационные базы (src/platform/infobases.rs)
- [x] `get_infobases_path() -> Result<PathBuf>` — %APPDATA%\1C\1CEStart\ibases.v8i
- [x] `get_info_bases() -> Result<Vec<InfoBase>>` — парсинг INI-формата
- [x] Определение `is_file_base` по Connect
- [x] `extract_file_path(connection) -> Option<String>` — File="path"; → path
- [x] Считать размер для файловых баз
- [x] `delete_info_bases(names: &[String]) -> Result<()>`
- [x] Создавать бэкап `ibases.v8i_backup_{timestamp}` до изменений
- [x] Удалять директории файловых баз (не UNC!)
- [x] Перезаписывать файл без удалённых секций

### 2.5 Реэкспорт
- [x] `src/platform/mod.rs` — pub use всех публичных функций
- [x] `cargo build` проходит без ошибок

---

## Фаза 3: UI

### 3.1 Точка входа (src/main.rs)
- [x] `gpui_platform::application().with_assets(Assets)`
- [x] `gpui_component::init(cx)` вызван первым
- [x] `WindowOptions` с `TitleBar::title_bar_options()` и размером 900×640
- [x] Заголовок окна "TwoClear — Очистка 1С"
- [x] Первая view обёрнута в `Root::new(view, window, cx)`

### 3.2 Структура приложения (src/app.rs)
- [x] Struct `TwoClearApp` со всеми полями состояния
- [x] `new(window, cx)` — инициализация + `load_all_data`
- [x] `load_versions` — async через `background_executor`
- [x] `load_infobases_and_cache` — async, затем сопоставление UUID
- [x] Сопоставление: `display_name` и `selected` по UUID
- [x] `impl Render for TwoClearApp` — базовый скелет

### 3.3 TitleBar и вкладки
- [x] `render_title_bar` — `TitleBar::new()` + `TabBar` с 3 вкладками
- [x] Переключение `active_tab` через `on_click`
- [x] `render_status_bar` — статус и ошибки внизу окна

### 3.4 Вкладка «Версии платформы»
- [x] `render_versions_tab` — spinner / пустое состояние / список
- [x] Тулбар: "Выбрать все", "Снять выбор", "Обновить", статистика, "Удалить"
- [x] Заголовок таблицы: Название | Версия | Дата | Размер
- [x] Строки со `Checkbox`, сортировка по version_int
- [x] Кнопка "Удалить" disabled при `sel_count == 0 || operation_in_progress`
- [x] Пустое состояние: "Установленные версии 1С не найдены"

### 3.5 Вкладка «Кэш метаданных»
- [x] `render_cache_tab` — spinner / пустое состояние / список
- [x] Тулбар: "Выбрать все", "Снять выбор", "Выбрать осиротевшие", "Обновить", статистика, "Удалить кэш"
- [x] Строки: Checkbox + название базы + UUID (truncate) + размер
- [x] `start_delete_cache` — async удаление + `retain` списка
- [x] Пустое состояние: "Кэш метаданных 1С пуст"

### 3.6 Вкладка «Информационные базы»
- [x] `render_infobases_tab` — spinner / пустое состояние / список
- [x] Тулбар: "Выбрать все", "Снять выбор", "Обновить", статистика, "Удалить из списка"
- [x] Строки: Checkbox + название + тип (Файловая/Серверная) + версия + путь + размер
- [x] `start_delete_infobases` — async + `retain` списка
- [x] Пустое состояние: "Список информационных баз пуст"

### 3.7 Вспомогательный render_empty_state
- [x] `render_empty_state(message, cx)` — централизованный рендер пустого состояния
- [x] Используется во всех трёх вкладках

---

## Фаза 4: Операции

### 4.1 Удаление версий с прогрессом
- [x] `start_uninstall` — последовательный msiexec для каждой версии
- [x] Обновление `status_message` после каждой версии ("Удаление N из M...")
- [x] `versions.retain` после успешного удаления каждой
- [x] `operation_in_progress = false` в конце (всегда, в т.ч. при ошибке)

### 4.2 Корректная обработка ошибок операций
- [x] Ошибки пишутся в `error_message`, не паникуют
- [x] `operation_in_progress` сбрасывается при любом исходе
- [x] При частичных ошибках (один элемент не удалился) операция продолжается для остальных

---

## Фаза 5: Полировка

### 5.1 Качество кода
- [ ] `cargo clippy -- --deny warnings` — нулевые предупреждения
- [ ] `cargo fmt` — код отформатирован
- [x] Нет `unwrap()` в продакшн-коде
- [x] Все пользовательские строки на русском языке

### 5.2 Корректность поведения
- [x] Кнопки деструктивных операций: `disabled(sel_count == 0 || operation_in_progress)`
- [x] Бэкап `ibases.v8i` создаётся до любых изменений
- [x] UNC-пути (`\\server\share`) не удаляются физически
- [x] `Root::new(view, window, cx)` — первая view в окне
- [x] `gpui_component::init(cx)` вызывается до любых компонентов

### 5.3 Финальная проверка
- [ ] `cargo build --release` проходит без ошибок
- [ ] Приложение запускается: `cargo run`
- [ ] Все три вкладки отображаются корректно
- [ ] Загрузка данных работает в фоне (UI не фризит)
- [ ] Деструктивные операции выполняются корректно