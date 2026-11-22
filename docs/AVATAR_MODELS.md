# Модели персонажа MeshPilot (Втубер-стиль)

PNG — это только fallback-заглушка. Настоящие Втубер-модели — **Live2D** (2D) и **VRM** (3D).
MeshPilot сам определяет тип по файлу и рендерит через WebEngine (Cubism SDK / three.js).

## Форматы

### Live2D (рекомендуется для аниме-персонажа)
- Файлы: `*.model3.json` + `*.moc3` + `texture_*.png` + `physics3.json` + `expressions/`
- Папка целиком, например: `assets/models/miko/miko.model3.json`
- Плюсы: лёгкий, анимированный (волосы/одежда physics), эмоции, lip-sync, взгляд
- Минусы: нужен риг (готовые модели уже с ригом)

### VRM (3D)
- Файл: один `*.vrm` (меш + текстуры + blendshapes внутри)
- Плюсы: 3D, вращение, полный рост, VRoid Studio кастомизация
- Минусы: тяжелее Live2D, нужен GPU

### PNG (fallback)
- Один `*.png` — статичная картинка с blink/bounce. Только если нет Live2D/VRM.

## Где скачать бесплатно

### Live2D (2D Втуберы)
| Источник | Ссылка | Что там |
|---|---|---|
| **Booth.pm** (главный) | https://booth.pm/en/search/live2D?max_price=0 | 5+ страниц бесплатных Live2D, zip с .model3.json |
| **itch.io** | https://itch.io/game-assets/tag-live2d/free | Бесплатные Live2D пакеты |
| **Gumroad** | https://gumroad.com/livesearch?q=live2d+vtuber | Free Live2D, фильтр price=0 |
| **Ko-fi** | https://ko-fi.com/search?q=live2d+vtuber | Бесплатные модели, напр. rrebdas |
| **Reddit r/Live2D** | https://www.reddit.com/r/Live2D/ | Free showcases, chibi half-body |
| **VTube Studio sample** | Входит в VTube Studio | Тестовая модель для проверки |
| **Live2D official** | https://www.live2d.com/en/download/ | Hiyori и др. сэмплы (учебные) |
| **Stream Skins Becca** | https://streamskins.net/free-vtuber-model/ | Полный риг + physics + expressions (Discord) |
| **TwitchGFX список 50+** | https://www.twitchgfx.com/2d-and-3d-free-vtuber-models-list/ | Каталог бесплатных 2D/3D |

### VRM (3D Втуберы)
| Источник | Ссылка | Что там |
|---|---|---|
| **VRoid Hub** (главный) | https://hub.vroid.com/ | Тысячи бесплатных VRM, аниме-стиль |
| **Booth.pm VRM** | https://booth.pm/en/search/vrm?max_price=0 | Бесплатные VRM |
| **Sketchfab** | https://sketchfab.com/search?q=vrm&type=models | Фильтр Free + VRM |
| **Gumroad VRM** | https://gumroad.com/livesearch?q=vrm | Free VRM |
| **Free3D** | https://free3d.com/3d-models/vrm | VRM раздел |

## Какую модель выбрать для MeshPilot

Для аниме-ассистента (Мико) идеально:
- **Live2D chibi half-body** — сидит на панели, милая, лёгкая, с physics волос
- С expressions: smile, sad, surprised, thinking (MeshPilot маппит состояния на них)
- С motions: idle, tap, flick (реакции)
- Текстуры 2048px+ для чёткости

Для Jarvis (бот) / Khan (сталкер):
- Live2D с маской/шлемом или VRM с капюшоном — атмосферно
- Или оставить PNG/иконку для них, а Live2D только для Мико

## Установка

```bash
# 1. Скачай zip с Booth/itch.io/Gumroad/VRoid Hub
# 2. Распакуй в assets/models/
unzip myvtuber.zip -d assets/models/miko/
# 3. Проверь структуру:
ls assets/models/miko/
#   miko.model3.json  miko.moc3  texture_00.png  physics3.json  expressions/

# 4. Укажи в config/default.yaml:
#    avatar:
#      model: assets/models/miko/miko.model3.json
#    или просто папку:
#      model: assets/models/miko/

# 5. Запусти:
meshpilot gui
```

Проверка модели без GUI:
```bash
PYTHONPATH=src python3 -c "from meshpilot.avatar.model_loader import load_model; print(load_model('assets/models/miko/'))"
```

## Лицензии — важно!

- Читай **license.txt** в пакете модели. Типы:
  - `personal use` — только для себя (MeshPilot локально — ок)
  - `commercial` — можно монетизировать (стримы, видео)
  - `redistribution` — обычно **запрещена** (нельзя перевыкладывать модель)
- MeshPilot локальный (не стримит модель в интернет) — в большинстве случаев personal use достаточно
- Всегда указывай автора в `assets/models/CREDITS.md`
- Не бери модели с «no redistribution» для публичных репозиториев — клади их в `.gitignore`

## Своя модель

- **VRoid Studio** (бесплатно): https://vroid.com/en/studio — сделай VRM за 30 минут, экспортируй .vrm
- **Live2D Cubism Editor** (бесплатный tier): https://www.live2d.com/en/cubism/download/ — риг PSD → .model3.json
- **PSD → Live2D**: найди PSD на Booth (free), открой в Cubism, экспортируй

## Что уже в репозитории

- `assets/models/meshpilot.png` — fallback PNG (сгенерированная аниме-девушка)
- `assets/models/.gitkeep` — папка для твоих моделей (не коммитить чужие модели без лицензии!)
