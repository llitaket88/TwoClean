use std::fs;
use std::process::Command;

fn main() {
    println!("=== Запуск автоматической публикации релиза ===");

    // 1. Проверяем, что мы находимся именно на ветке master
    let current_branch = Command::new("git")
        .args(&["branch", "--show-current"])
        .output()
        .expect("Не удалось проверить текущую ветку Git");

    let branch_name = String::from_utf8_lossy(&current_branch.stdout)
        .trim()
        .to_string();
    if branch_name != "master" {
        println!("Ошибка: Публикация релиза возможна только из ветки 'master'!");
        println!(
            "Текущая ветка: '{}'. Переключитесь на master и повторите.",
            branch_name
        );
        std::process::exit(1);
    }

    // 2. Читаем Cargo.toml и вытягиваем версию
    let cargo_toml = fs::read_to_string("Cargo.toml")
        .expect("Не удалось прочитать Cargo.toml. Убедитесь, что запускаете из корня проекта.");

    let version = cargo_toml
        .lines()
        .find(|line| line.trim().starts_with("version"))
        .and_then(|line| line.split('=').nth(1))
        .map(|v| v.trim().trim_matches('"'))
        .expect("Не удалось найти поле 'version' в Cargo.toml");

    let tag = format!("v{}", version);
    println!("В Cargo.toml установлена версия: {}", tag);

    // 3. Проверяем, существует ли уже такой Git-тег
    let check_tag = Command::new("git")
        .args(&["tag", "-l", &tag])
        .output()
        .expect("Не удалось выполнить git tag");

    let tag_exists_locally = !String::from_utf8_lossy(&check_tag.stdout).trim().is_empty();

    if tag_exists_locally {
        println!("Ошибка: Тег {} уже существует!", tag);
        println!("Пожалуйста, обновите версию в Cargo.toml перед созданием нового релиза.");
        std::process::exit(1);
    }

    // 4. Проверяем, существует ли файл изменений в папке docs
    let changelog_path = format!("docs/CHANGELOG-{}.md", tag);
    if !fs::metadata(&changelog_path).is_ok() {
        println!(
            "Ошибка: Файл описания изменений '{}' не найден!",
            changelog_path
        );
        println!("Убедитесь, что файл создан и находится в папке 'docs/' перед публикацией.");
        std::process::exit(1);
    }

    // 5. Создаем тег локально и пушим ТОЛЬКО его
    println!("Создание локального тега {}...", tag);
    run_cmd("git", &["tag", &tag]);

    println!("Отправка тега {} в удаленный репозиторий...", tag);
    run_cmd("git", &["push", "origin", &tag]);

    println!(
        "Успех! Тег {} успешно отправлен. GitHub Actions запустит сборку релиза на основе этого тега.",
        tag
    );
}

fn run_cmd(program: &str, args: &[&str]) {
    let status = Command::new(program)
        .args(args)
        .status()
        .expect(&format!("Не удалось выполнить команду: {}", program));

    if !status.success() {
        println!("Ошибка при выполнении команды Git.");
        std::process::exit(1);
    }
}
