// src/platform/cache.rs — чтение и удаление кэша метаданных 1C

use std::path::Path;

use crate::{models::CacheEntry, platform::utils::get_dir_size};

/// Проверяет, является ли имя директории UUID (8-4-4-4-12 hex)
fn is_uuid_dir(name: &str) -> bool {
    let parts: Vec<&str> = name.split('-').collect();
    if parts.len() != 5 {
        return false;
    }
    let expected_lens = [8, 4, 4, 4, 12];
    for (part, &expected_len) in parts.iter().zip(expected_lens.iter()) {
        if part.len() != expected_len {
            return false;
        }
        if !part.chars().all(|c| c.is_ascii_hexdigit()) {
            return false;
        }
    }
    true
}

/// Возвращает список записей кэша метаданных 1C из %LOCALAPPDATA%\1C\1cv8*\{UUID}
pub fn get_cache_entries() -> anyhow::Result<Vec<CacheEntry>> {
    let local_app_data = std::env::var("LOCALAPPDATA")
        .map_err(|_| anyhow::anyhow!("LOCALAPPDATA не определена"))?;

    let base_dir = Path::new(&local_app_data).join("1C");
    if !base_dir.exists() {
        return Ok(Vec::new());
    }

    let mut entries = Vec::new();

    let read_dir = match std::fs::read_dir(&base_dir) {
        Ok(rd) => rd,
        Err(_) => return Ok(Vec::new()),
    };

    for entry in read_dir.filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let dir_name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };

        // Только директории с именем начинающимся на "1cv8"
        if !dir_name.starts_with("1cv8") {
            continue;
        }

        let sub_read_dir = match std::fs::read_dir(&path) {
            Ok(rd) => rd,
            Err(_) => continue,
        };

        for sub_entry in sub_read_dir.filter_map(|e| e.ok()) {
            let sub_path = sub_entry.path();
            if !sub_path.is_dir() {
                continue;
            }

            let sub_name = match sub_path.file_name().and_then(|n| n.to_str()) {
                Some(n) => n.to_string(),
                None => continue,
            };

            if !is_uuid_dir(&sub_name) {
                continue;
            }

            let size = get_dir_size(&sub_path);
            let path_str = sub_path.to_string_lossy().to_string();

            entries.push(CacheEntry {
                path: path_str,
                uuid: sub_name,
                display_name: "<База не найдена>".to_string(),
                size,
                selected: false, // будет скорректировано в app.rs
            });
        }
    }

    Ok(entries)
}

/// Удаляет директорию кэша по указанному пути
pub fn delete_cache_entry(path: &str) -> anyhow::Result<()> {
    std::fs::remove_dir_all(path).map_err(Into::into)
}
