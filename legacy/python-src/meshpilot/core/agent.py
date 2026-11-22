"""Hermes-стиль агент: цикл ReAct + function calling поверх Brain.

Лёгкий аналог Hermes Agent / LangGraph без тяжёлых зависимостей:
- модель решает: ответить текстом или вызвать tool
- tools = оптимизатор CachyOS + системные утилиты
- лимит шагов, защита от зацикливания, dry-run для опасных операций
"""
from __future__ import annotations
import json
import logging
from typing import Any, Awaitable, Callable

from meshpilot.core.brain import Brain
from meshpilot.core.memory import memory

log = logging.getLogger("meshpilot.agent")

ToolFn = Callable[..., Awaitable[str]]

class Agent:
    def __init__(self, brain: Brain | None = None, max_steps: int = 6) -> None:
        self.brain = brain or Brain()
        self.max_steps = max_steps
        self.tools: dict[str, tuple[dict, ToolFn]] = {}

    def register(self, name: str, schema: dict, fn: ToolFn) -> None:
        self.tools[name] = (schema, fn)

    def tool_schemas(self) -> list[dict]:
        return [s for s, _ in self.tools.values()]

    async def run(self, user_text: str) -> str:
        history = memory.recent(10)
        messages: list[dict] = [{"role": "system", "content": self.brain.system_prompt}]
        messages.extend(history)
        messages.append({"role": "user", "content": user_text})
        memory.add_message("user", user_text)

        for step in range(self.max_steps):
            res = await self.brain.chat(messages, tools=self.tool_schemas() or None)
            tool_calls = res.get("tool_calls") or []
            content = res.get("content", "")
            if not tool_calls:
                memory.add_message("assistant", content)
                return content
            # выполняем tool calls
            messages.append({"role": "assistant", "content": content or None, "tool_calls": tool_calls})
            for tc in tool_calls:
                name = tc.get("name") or tc.get("function", {}).get("name", "")
                args_raw = tc.get("arguments") or tc.get("function", {}).get("arguments", "{}")
                try:
                    args = json.loads(args_raw) if isinstance(args_raw, str) else args_raw
                except Exception:
                    args = {}
                if name not in self.tools:
                    result = f"Неизвестный инструмент: {name}"
                else:
                    try:
                        result = await self.tools[name][1](**args)
                    except Exception as e:
                        result = f"Ошибка инструмента {name}: {e}"
                log.info("tool %s -> %s", name, str(result)[:200])
                messages.append({"role": "tool", "content": str(result), "tool_call_id": tc.get("id", name)})
        # если вышли по лимиту — финальный ответ без tools
        res = await self.brain.chat(messages)
        text = res.get("content", "Не смог завершить задачу за отведённые шаги.")
        memory.add_message("assistant", text)
        return text

def build_default_agent() -> Agent:
    """Агент со всеми инструментами оптимизатора."""
    from meshpilot.optimizer import tools as opt_tools
    agent = Agent()
    for name, schema, fn in opt_tools.ALL_TOOLS:
        agent.register(name, schema, fn)
    return agent
