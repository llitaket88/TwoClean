use std::env;
use std::path::Path;
use std::process::Command;
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::System::Threading::{OpenProcess, PROCESS_SYNCHRONIZE, WaitForSingleObject};

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 4 {
        eprintln!("Usage: TwoClean-updater <pid> <new_exe> <current_exe>");
        return;
    }

    let pid = &args[1];
    let new_exe = &args[2];
    let current_exe = &args[3];

    println!("PID: {pid}");
    println!("New EXE: {new_exe}");
    println!("Current EXE: {current_exe}");

    let pid: u32 = match pid.parse() {
        Ok(pid) => pid,
        Err(_) => {
            eprintln!("Invalid PID: {pid}");
            return;
        }
    };

    unsafe {
        let process = match OpenProcess(PROCESS_SYNCHRONIZE, false, pid) {
            Ok(handle) => handle,
            Err(error) => {
                eprintln!("Failed to open process: {error}");
                return;
            }
        };

        println!("Waiting for process {pid} to exit...");

        WaitForSingleObject(process, u32::MAX);

        if let Err(error) = replace_and_restart(Path::new(new_exe), Path::new(current_exe)) {
            eprintln!("Ошибка обновления: {error}");
        }

        let _ = CloseHandle(process);
    }

    println!("Main process exited.");
}

fn replace_and_restart(new_exe: &Path, current_exe: &Path) -> anyhow::Result<()> {
    let backup = current_exe.with_extension("exe.bak");
    let staged = current_exe.with_file_name("TwoClean-update.exe");

    // Не перезаписываем существующую резервную копию.
    if backup.exists() {
        anyhow::bail!("Резервная копия уже существует: {}", backup.display());
    }

    // Temp и папка программы могут находиться на разных дисках.
    // Поэтому сначала копируем новый EXE рядом с текущим.
    std::fs::copy(new_exe, &staged)?;

    // Сохраняем старую версию.
    std::fs::rename(current_exe, &backup)?;

    // Устанавливаем новую версию.
    if let Err(error) = std::fs::rename(&staged, current_exe) {
        let _ = std::fs::rename(&backup, current_exe);
        anyhow::bail!("Не удалось заменить EXE: {error}");
    }

    // Запускаем новую версию.
    if let Err(error) = Command::new(current_exe)
        .current_dir(current_exe.parent().unwrap_or(Path::new(".")))
        .spawn()
    {
        // Если запуск не удался, пытаемся восстановить старую версию.
        let _ = std::fs::remove_file(current_exe);

        if let Err(restore_error) = std::fs::rename(&backup, current_exe) {
            anyhow::bail!(
                "Не удалось запустить новую версию: {error}. \
                 Не удалось восстановить резервную копию: {restore_error}"
            );
        }

        anyhow::bail!("Не удалось запустить новую версию: {error}");
    }

    // Новая версия запущена — удаляем резервную копию.
    if let Err(error) = std::fs::remove_file(&backup) {
        eprintln!("Не удалось удалить резервную копию: {error}");
    }

    // Удаляем временную копию, если она осталась.
    let _ = std::fs::remove_file(&staged);

    println!("Обновление завершено!");

    Ok(())
}
