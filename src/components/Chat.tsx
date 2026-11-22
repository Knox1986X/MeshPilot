import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface Msg { role: "user" | "assistant"; content: string; }

export default function Chat() {
  const [msgs, setMsgs] = useState<Msg[]>([
    { role: "assistant", content: "Привет! Я MeshPilot. Спроси: «почисти кэш и поставь лучшие зеркала» — я сам вызову оптимизатор." },
  ]);
  const [input, setInput] = useState("");
  const [busy, setBusy] = useState(false);

  const send = async () => {
    const text = input.trim();
    if (!text || busy) return;
    setInput("");
    setMsgs((m) => [...m, { role: "user", content: text }]);
    setBusy(true);
    try {
      const history = msgs.map((m) => ({ role: m.role, content: m.content }));
      history.push({ role: "user", content: text });
      const res = await invoke<{ content: string; provider: string }>("chat", { messages: history });
      setMsgs((m) => [...m, { role: "assistant", content: `${res.content}  [${res.provider}]` }]);
    } catch (e) {
      setMsgs((m) => [...m, { role: "assistant", content: `Ошибка: ${e}. Проверь .env (GROQ_API_KEY) или запусти ollama serve.` }]);
    }
    setBusy(false);
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      <h1 style={{ fontSize: 20, fontWeight: 700, color: "white", marginBottom: 12 }}>Чат с MeshPilot</h1>
      <div style={{ flex: 1, overflowY: "auto", marginBottom: 12 }}>
        {msgs.map((m, i) => (
          <div key={i} className={`chat-msg ${m.role}`}>{m.content}</div>
        ))}
        {busy && <div className="chat-msg ai">Думаю...</div>}
      </div>
      <div style={{ display: "flex", gap: 8 }}>
        <input
          className="input"
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && send()}
          placeholder="Напиши сообщение... Enter — отправить"
        />
        <button className="btn btn-primary" onClick={send}>Отправить</button>
        <button className="btn">🎤 Голос</button>
      </div>
    </div>
  );
}
