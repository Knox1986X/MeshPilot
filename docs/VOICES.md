# Голоса MeshPilot

## Jarvis (Тони Старк)
- Piper: `en_GB-alan_medium` | Edge: `en-GB-RyanNeural` | Groq: `Fritz-PlayAI`
- Rate 0.95, pitch -2. Формальный, британский, спокойный.
- Скачать: https://huggingface.co/rhasspy/piper-voices/resolve/main/en/en_GB/alan/medium/en_GB-alan_medium.onnx

## Khan (Metro)
- Piper: `ru_RU-dmitry_medium` | Edge: `ru-RU-DmitryNeural` | Groq: `Arista-PlayAI`
- Rate 0.82, pitch -12. Глубокий, хриплый, медленный.
- Скачать: https://huggingface.co/rhasspy/piper-voices/resolve/main/ru/ru_RU/dmitry/medium/ru_RU-dmitry_medium.onnx

## Anime (Мико)
- Piper: `ru_RU-irina_medium` | Edge: `ru-RU-SvetlanaNeural` | Groq: `Ava-PlayAI`
- Rate 1.12, pitch +14. Энергичная, высокая, быстрая.
- Скачать: https://huggingface.co/rhasspy/piper-voices/resolve/main/ru/ru_RU/irina/medium/ru_RU-irina_medium.onnx

## Клонирование своих голосов (будущее)
- **RVC** (Retrieval-based Voice Conversion): 3-10 мин записи → свой голос
- **SoVITS / GPT-SoVITS**: 1-5 мин, высокое качество, RU поддержка
- **XTTS-v2** (Coqui): zero-shot клонирование по 6 сек референсу
- **Piper fine-tune**: переобучение ONNX под свой голос
- План: `meshpilot voice clone --name myvoice --samples ./samples/` (см. IDEAS.md)

## Wake word обучение
openWakeWord: https://github.com/dscripka/openWakeWord#training
Нужны позитивные/негативные примеры «MeshPilot» на 16kHz.
