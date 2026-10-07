use gpui_http_client::{HttpClient, github::latest_github_release};
use std::sync::Arc;

use sysinfo::{ProcessesToUpdate, System};
use walkdir::WalkDir;
use windows::{
    Win32::{
        Foundation::{CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HANDLE},
        System::Threading::CreateMutexW,
        UI::WindowsAndMessaging::{MB_ICONINFORMATION, MB_OK, MessageBoxW},
    },
    core::PCWSTR,
};

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

pub struct SingleInstance {
    handle: HANDLE,
}

impl SingleInstance {
    pub fn new(name: &str) -> Option<Self> {
        let name: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
        let handle = unsafe { CreateMutexW(None, false, PCWSTR(name.as_ptr())) }.ok()?;
        if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
            unsafe {
                let _ = CloseHandle(handle);
            }
            return None;
        }
        Some(Self { handle })
    }
}

impl Drop for SingleInstance {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.handle);
        }
    }
}

/// Отображает сообщение о запущенном приложении
pub fn show_already_running_message() {
    let title: Vec<u16> = "Внимание\0".encode_utf16().collect();
    let message: Vec<u16> = "Приложение уже запущено!\0".encode_utf16().collect();
    unsafe {
        let _ = MessageBoxW(
            None,
            PCWSTR(message.as_ptr()),
            PCWSTR(title.as_ptr()),
            MB_OK | MB_ICONINFORMATION,
        );
    }
}

/// Проверка запущенных процессов 1с
pub fn has_running_1c_processes() -> bool {
    let mut system = System::new();
    system.refresh_processes(ProcessesToUpdate::All, true);
    system.processes().values().any(|process| {
        let name = process.name().to_string_lossy().to_ascii_lowercase();
        name.starts_with("1cv8") && name.ends_with(".exe")
    })
}

/// Получает данные о новой версии программы с GitHub
pub async fn get_data_from_github(http_client: Arc<dyn HttpClient>) -> Option<String> {
    match latest_github_release("llitaket88/TwoClean", false, false, http_client).await {
        Ok(release) => Some(release.tag_name.trim_start_matches("v").to_string()),
        Err(_e) => None,
    }
}
