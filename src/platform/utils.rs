use futures::AsyncReadExt;
use gpui_http_client::{AsyncBody, HttpClient, github::latest_github_release};
use sha2::{Digest, Sha256};
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

/// Содержит информацию о новом релизе
#[derive(Debug, Clone)]
pub struct UpdateInfo {
    pub version: String,
    pub download_url: String,
    pub sha256: Option<String>,
}

/// Получает данные о новой версии программы
pub async fn get_update_info(http_client: Arc<dyn HttpClient>) -> Option<UpdateInfo> {
    let release = latest_github_release("llitaket88/TwoClean", false, false, http_client)
        .await
        .ok()?;

    let asset = release
        .assets
        .iter()
        .find(|asset| asset.name == "TwoClean.exe")?;

    let sha256 = asset.digest.clone();

    Some(UpdateInfo {
        version: release.tag_name.trim_start_matches('v').to_string(),
        download_url: asset.browser_download_url.clone(),
        sha256,
    })
}

/// Скачивание обновления
pub async fn download_update(
    http_client: Arc<dyn HttpClient>,
    url: &str,
) -> anyhow::Result<Vec<u8>> {
    let mut response = http_client.get(url, AsyncBody::empty(), true).await?;

    if !response.status().is_success() {
        anyhow::bail!("Ошибка скачивания: HTTP {}", response.status());
    }

    let mut bytes = Vec::new();
    response.body_mut().read_to_end(&mut bytes).await?;

    Ok(bytes)
}

/// Проверка SHA256 полученного файла
pub fn verify_sha256(bytes: &[u8], expected: &str) -> bool {
    let hash = Sha256::digest(bytes);
    let actual: String = hash.iter().map(|byte| format!("{byte:02x}")).collect();

    actual.eq_ignore_ascii_case(expected)
}

/// Скачивает обновление и проверяет SHA-256
pub async fn download_and_check_hash(
    http_client: Arc<dyn HttpClient>,
    info: &UpdateInfo,
) -> anyhow::Result<Vec<u8>> {
    let expected = info
        .sha256
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("У релиза отсутствует SHA-256"))?;

    let bytes = download_update(http_client, &info.download_url).await?;

    if !verify_sha256(&bytes, expected) {
        anyhow::bail!("SHA-256 скачанного файла не совпадает");
    }

    Ok(bytes)
}

/// Сохраняет проверенное обновление во временный файл
pub fn save_update(bytes: &[u8]) -> anyhow::Result<std::path::PathBuf> {
    let dir = std::env::temp_dir();
    println!("Temporary directory: {}", dir.display());

    let path = std::env::temp_dir().join("TwoClean-update.exe");

    std::fs::write(&path, bytes)?;

    let current_exe = std::env::current_exe();

    println!("Current exe: {}", current_exe.unwrap().display());

    let pid = std::process::id();
    println!("Current PID: {pid}");

    Ok(path)
}
