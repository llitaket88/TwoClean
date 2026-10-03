// src/platform/infobases.rs

use crate::models::InfoBase;
use std::path::PathBuf;

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

    // Убираем UTF-8 BOM (\u{feff}) если присутствует
    let content = content.strip_prefix('\u{feff}').unwrap_or(&content);

    let bases = parse_ibases_v8i(content);
    Ok(bases)
}

fn parse_ibases_v8i(content: &str) -> Vec<InfoBase> {
    let mut bases = Vec::new();
    let mut current_name: Option<String> = None;
    let mut connect = String::new();
    let mut id = String::new();

    let flush = |name: Option<String>, connect: &str, id: &str, bases: &mut Vec<InfoBase>| {
        if let Some(name) = name {
            if name.is_empty() || id.is_empty() {
                return;
            }
            let uuid = id.trim_matches(|c| c == '{' || c == '}').to_string();

            bases.push(InfoBase {
                name,
                uuid,
                connection: connect.to_string(),
            });
        }
    };

    for line in content.lines() {
        let line = line.trim();

        if line.starts_with('[') && line.ends_with(']') {
            flush(current_name.take(), &connect, &id, &mut bases);
            current_name = Some(line[1..line.len() - 1].to_string());
            connect.clear();
            id.clear();
        } else if let Some((key, value)) = split_key_value(line) {
            match key.to_ascii_lowercase().as_str() {
                "connect" => connect = value.to_string(),
                "id" => id = value.to_string(),
                _ => {}
            }
        }
    }

    flush(current_name.take(), &connect, &id, &mut bases);

    bases
}

fn split_key_value(line: &str) -> Option<(&str, &str)> {
    let pos = line.find('=')?;
    let key = line[..pos].trim();
    let value = line[pos + 1..].trim();
    Some((key, value))
}
