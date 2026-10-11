# Автоматическая публикация

- запускать локально в ветке master
- версия будет прочитана из Cargo.toml

```sh
cargo run -p publish
```

# Ручная публикация

- запускать локально в ветке master

```sh
git tag v1.0.0
git push origin v1.0.0
```

# Удаление релиза

#### Удаление тега

```sh
git tag -d v1.0.0
git push origin --delete v1.0.0
```

#### Вручную удалить релиз в интерфейсе GitHub
