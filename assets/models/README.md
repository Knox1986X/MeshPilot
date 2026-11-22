# Модели персонажа

Положи сюда свою Втубер-модель (Live2D или VRM). См. docs/AVATAR_MODELS.md.

## Live2D
```
assets/models/miko/
  miko.model3.json   <- укажи этот путь в config/default.yaml
  miko.moc3
  texture_00.png
  physics3.json
  expressions/
```

## VRM
```
assets/models/miko.vrm   <- один файл
```

## PNG (fallback)
```
assets/models/meshpilot.png
```

## Проверка
```bash
PYTHONPATH=src python3 -c "from meshpilot.avatar.model_loader import load_model; print(load_model('assets/models/miko/'))"
```

## Лицензия
Не коммить чужие модели без разрешения автора. Добавь автора в CREDITS.md.
