// src/platform/utils.rs — утилиты платформенного слоя

use walkdir::WalkDir;

/// Рекурсивно вычисляет размер директории в байтах.
pub fn get_dir_size(path: &std::path::Path) -> u64 {
    WalkDir::new(path)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter_map(|e| e.metadata().ok())
        .filter(|m| m.is_file())
        .map(|m| m.len())
        .sum()
}

/// Форматирует размер в байтах в читаемую строку.
pub fn format_size(bytes: u64) -> String {
    const GB: u64 = 1_073_741_824;
    const MB: u64 = 1_048_576;
    const KB: u64 = 1_024;

    if bytes >= GB {
        format!("{:.1} ГБ", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} МБ", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} КБ", bytes as f64 / KB as f64)
    } else {
        format!("{} Б", bytes)
    }
}

/// Преобразует дату из формата "yyyyMMdd" в "yyyy-MM-dd".
/// Если строка не соответствует формату — возвращает исходную строку.
pub fn parse_install_date(s: &str) -> String {
    if s.len() != 8 || !s.chars().all(|c| c.is_ascii_digit()) {
        return s.to_string();
    }
    format!("{}-{}-{}", &s[0..4], &s[4..6], &s[6..8])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_size_bytes() {
        assert_eq!(format_size(0), "0 Б");
        assert_eq!(format_size(512), "512 Б");
        assert_eq!(format_size(1023), "1023 Б");
    }

    #[test]
    fn test_format_size_kb() {
        assert_eq!(format_size(1024), "1.0 КБ");
        assert_eq!(format_size(2048), "2.0 КБ");
    }

    #[test]
    fn test_format_size_mb() {
        assert_eq!(format_size(1_048_576), "1.0 МБ");
    }

    #[test]
    fn test_format_size_gb() {
        assert_eq!(format_size(1_073_741_824), "1.0 ГБ");
    }

    #[test]
    fn test_parse_install_date_valid() {
        assert_eq!(parse_install_date("20230415"), "2023-04-15");
        assert_eq!(parse_install_date("20001231"), "2000-12-31");
    }

    #[test]
    fn test_parse_install_date_invalid() {
        assert_eq!(parse_install_date("2023041"), "2023041");
        assert_eq!(parse_install_date("abcdefgh"), "abcdefgh");
        assert_eq!(parse_install_date(""), "");
    }
}
