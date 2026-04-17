// src/platform/infobases.rs

use crate::models::InfoBase;
use crate::platform::utils::get_dir_size;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn get_infobases_path() -> anyhow::Result<PathBuf> {
    let appdata = std::env::var("APPDATA")
        .map_err(|_| anyhow::anyhow!("Переменная окружения APPDATA не найдена"))?;
    Ok(PathBuf::from(appdata)
        .join("1C")
        .join("1CEStart")
        .join("ibases.v8i"))
}

pub fn get_info_bases() -> anyhow::Result<Vec<InfoBase>> {
    let path = get_infobases_path()?;

    if !path.exists() {
        return Ok(Vec::new());
    }

    let content = std::fs::read_to_string(&path)
        .map_err(|e| anyhow::anyhow!("Не удалось прочитать ibases.v8i: {}", e))?;

    let bases = parse_ibases_v8i(&content);
    Ok(bases)
}

fn parse_ibases_v8i(content: &str) -> Vec<InfoBase> {
    let mut bases = Vec::new();
    let mut current_name: Option<String> = None;
    let mut connect = String::new();
    let mut id = String::new();
    let mut version: Option<String> = None;

    let flush = |name: Option<String>,
                 connect: &str,
                 id: &str,
                 version: Option<String>,
                 bases: &mut Vec<InfoBase>| {
        if let Some(name) = name {
            if name.is_empty() || id.is_empty() {
                return;
            }
            let uuid = id.trim_matches(|c| c == '{' || c == '}').to_string();
            let is_file_base = connect.to_ascii_lowercase().starts_with("file=");
            let size = if is_file_base {
                extract_file_path(connect)
                    .as_deref()
                    .map(|p| get_dir_size(std::path::Path::new(p)))
                    .unwrap_or(0)
            } else {
                0
            };
            bases.push(InfoBase {
                name,
                uuid,
                version,
                connection: connect.to_string(),
                is_file_base,
                size,
                selected: false,
            });
        }
    };

    for line in content.lines() {
        let line = line.trim();

        if line.starts_with('[') && line.ends_with(']') {
            flush(
                current_name.take(),
                &connect,
                &id,
                version.take(),
                &mut bases,
            );
            current_name = Some(line[1..line.len() - 1].to_string());
            connect.clear();
            id.clear();
            version = None;
        } else if let Some((key, value)) = split_key_value(line) {
            match key.to_ascii_lowercase().as_str() {
                "connect" => connect = value.to_string(),
                "id" => id = value.to_string(),
                "defaultversion" | "version" => {
                    version = Some(value.to_string());
                }
                _ => {}
            }
        }
    }

    flush(
        current_name.take(),
        &connect,
        &id,
        version.take(),
        &mut bases,
    );

    bases
}

fn split_key_value(line: &str) -> Option<(&str, &str)> {
    let pos = line.find('=')?;
    let key = line[..pos].trim();
    let value = line[pos + 1..].trim();
    Some((key, value))
}

fn extract_file_path(connection: &str) -> Option<String> {
    if connection.to_ascii_lowercase().starts_with("file=") {
        let rest = &connection[5..];
        let path = rest
            .trim_start_matches('"')
            .trim_end_matches(';')
            .trim_end_matches('"');
        if !path.is_empty() {
            Some(path.to_string())
        } else {
            None
        }
    } else {
        None
    }
}

pub fn delete_info_bases(names: &[String]) -> anyhow::Result<()> {
    if names.is_empty() {
        return Ok(());
    }

    let path = get_infobases_path()?;

    if !path.exists() {
        return Err(anyhow::anyhow!("Файл ibases.v8i не найден"));
    }

    let content = std::fs::read_to_string(&path)
        .map_err(|e| anyhow::anyhow!("Не удалось прочитать ibases.v8i: {}", e))?;

    // Создаём бэкап
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let backup_path = path.with_file_name(format!("ibases.v8i_backup_{}", timestamp));
    std::fs::copy(&path, &backup_path)
        .map_err(|e| anyhow::anyhow!("Не удалось создать бэкап ibases.v8i: {}", e))?;

    // Собираем информацию о базах для удаления файловых директорий
    let bases = parse_ibases_v8i(&content);
    for base in &bases {
        if names.contains(&base.name)
            && base.is_file_base
            && let Some(file_path) = extract_file_path(&base.connection)
        {
            // Не удаляем UNC-пути (\\server\share)
            if !file_path.starts_with("\\\\") {
                let dir = std::path::Path::new(&file_path);
                if dir.exists() {
                    let _ = std::fs::remove_dir_all(dir);
                }
            }
        }
    }

    // Перезаписываем файл без удалённых секций
    let new_content = remove_sections(&content, names);
    std::fs::write(&path, new_content)
        .map_err(|e| anyhow::anyhow!("Не удалось записать ibases.v8i: {}", e))?;

    Ok(())
}

fn remove_sections(content: &str, names_to_remove: &[String]) -> String {
    let mut result = String::new();
    let mut skip = false;

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            let section_name = &trimmed[1..trimmed.len() - 1];
            skip = names_to_remove.iter().any(|n| n == section_name);
        }

        if !skip {
            result.push_str(line);
            result.push('\n');
        }
    }

    result
}
