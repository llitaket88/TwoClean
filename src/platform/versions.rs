// src/platform/versions.rs
use std::collections::HashSet;
use winreg::{RegKey, enums::*};
use crate::{models::InstalledVersion, platform::utils::{get_dir_size, parse_install_date}};

const UNINSTALL_KEY: &str = "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall";

const VALID_PUBLISHERS: &[&str] = &["1С-Софт", "1C-Soft", "1C", "1С"];

fn is_1c_publisher(publisher: &str) -> bool {
    VALID_PUBLISHERS.iter().any(|&p| p.eq_ignore_ascii_case(publisher))
}

fn version_to_int(version: &str) -> u64 {
    let parts: Vec<u64> = version
        .split('.')
        .filter_map(|p| p.parse::<u64>().ok())
        .collect();

    if parts.len() != 4 {
        return 0;
    }

    parts[0] * 10_000_000_000
        + parts[1] * 100_000_000
        + parts[2] * 100_000
        + parts[3]
}

fn read_versions_from_key(key: &RegKey, seen: &mut HashSet<String>, result: &mut Vec<InstalledVersion>) {
    for subkey_name in key.enum_keys().filter_map(|k| k.ok()) {
        let subkey = match key.open_subkey(&subkey_name) {
            Ok(k) => k,
            Err(_) => continue,
        };

        let publisher: String = subkey.get_value("Publisher").unwrap_or_default();
        if !is_1c_publisher(&publisher) {
            continue;
        }

        let name: String = subkey.get_value("DisplayName").unwrap_or_default();
        if name.is_empty() {
            continue;
        }

        let location: String = subkey.get_value("InstallLocation").unwrap_or_default();
        if location.is_empty() {
            continue;
        }

        let uuid = subkey_name.clone();
        if seen.contains(&uuid) {
            continue;
        }
        seen.insert(uuid.clone());

        let version: String = subkey.get_value("DisplayVersion").unwrap_or_default();
        let install_date_raw: String = subkey.get_value("InstallDate").unwrap_or_default();
        let install_date = parse_install_date(&install_date_raw);
        let version_int = version_to_int(&version);
        let size = get_dir_size(std::path::Path::new(&location));

        result.push(InstalledVersion {
            name,
            version,
            version_int,
            uuid,
            location,
            install_date,
            size,
            selected: false,
        });
    }
}

pub fn get_installed_versions() -> anyhow::Result<Vec<InstalledVersion>> {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let mut result: Vec<InstalledVersion> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    if let Ok(key32) = hklm.open_subkey_with_flags(UNINSTALL_KEY, KEY_READ | KEY_WOW64_32KEY) {
        read_versions_from_key(&key32, &mut seen, &mut result);
    }

    if let Ok(key64) = hklm.open_subkey_with_flags(UNINSTALL_KEY, KEY_READ | KEY_WOW64_64KEY) {
        read_versions_from_key(&key64, &mut seen, &mut result);
    }

    result.sort_by_key(|v| v.version_int);

    Ok(result)
}

pub fn uninstall_version(uuid: &str) -> anyhow::Result<bool> {
    let status = std::process::Command::new("msiexec.exe")
        .args(["/x", uuid, "/q"])
        .status()?;

    Ok(status.success())
}
