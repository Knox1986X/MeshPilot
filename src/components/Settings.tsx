import { useState } from "react";

const VOICES = [
  { id: "jarvis", label: "🎩 Джарвис", desc: "Тони Старк\nбританский, формальный" },
  { id: "khan", label: "🎖️ Хан", desc: "Metro\nглубокий, хриплый" },
  { id: "anime", label: "🌸 Мико", desc: "аниме\nэнергичная, милая" },
];

export default function Settings() {
  const [voice, setVoice] = useState("jarvis");
  return (
    <div>
      <h1 style={{ fontSize: 20, fontWeight: 700, color: "white", marginBottom: 12 }}>Настройки</h1>
      <div style={{ color: "#6b7280", marginBottom: 8 }}>Голос ассистента</div>
      <div className="voice-cards">
        {VOICES.map((v) => (
          <div key={v.id} className={`voice-card ${voice === v.id ? "active" : ""}`} onClick={() => setVoice(v.id)}>
            <div style={{ fontWeight: 700 }}>{v.label}</div>
            <div style={{ color: "#6b7280", fontSize: 12, whiteSpace: "pre-line", marginTop: 4 }}>{v.desc}</div>
          </div>
        ))}
      </div>
      <div style={{ color: "#6b7280", margin: "16px 0 8px" }}>LLM каскад</div>
      <div>Groq (Llama 3.3 70B) → GigaChat (Sber) → Hermes локально (Ollama) → OpenAI-compat</div>
      <div style={{ color: "#6b7280", marginTop: 8 }}>Ключи в .env: GROQ_API_KEY, GIGACHAT_CLIENT_ID/SECRET. Hermes: ollama pull nous-hermes3:8b</div>
      <div style={{ color: "#6b7280", margin: "16px 0 8px" }}>Модель персонажа (Live2D)</div>
      <div>Текущая: /models/jane/简/简.model3.json (Jane Doe ZZZ, Booth #6235947)</div>
      <div style={{ color: "#6b7280", marginTop: 8 }}>Скопируй модель: cp -r assets/models/jane public/models/</div>
    </div>
  );
}
