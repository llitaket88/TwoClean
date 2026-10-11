# Деплой новой версии

## Переключиться на ветку master
```sh
git tag v1.0.0
git push origin v1.0.0
```

# Удаление релиза

### Удаление тега

```sh
git tag -d v1.0.0
git push origin --delete v1.0.0
```

### Затем вручную удалить релиз в GitHub
