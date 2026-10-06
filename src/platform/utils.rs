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
}
