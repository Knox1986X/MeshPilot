"""Долговременная память MeshPilot (SQLite): диалоги, предпочтения, факты."""
from __future__ import annotations
import json
import sqlite3
import time
from pathlib import Path

DB_PATH = Path.home() / ".local/share/meshpilot/memory.db"

class Memory:
    def __init__(self, path: Path = DB_PATH) -> None:
        self.path = path
        self.path.parent.mkdir(parents=True, exist_ok=True)
        self.conn = sqlite3.connect(str(self.path), check_same_thread=False)
        self.conn.row_factory = sqlite3.Row
        self._init()

    def _init(self) -> None:
        self.conn.executescript("""
        CREATE TABLE IF NOT EXISTS messages (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            role TEXT NOT NULL, content TEXT NOT NULL,
            ts REAL NOT NULL
        );
        CREATE TABLE IF NOT EXISTS facts (
            key TEXT PRIMARY KEY, value TEXT NOT NULL, ts REAL NOT NULL
        );
        CREATE TABLE IF NOT EXISTS prefs (
            key TEXT PRIMARY KEY, value TEXT NOT NULL
        );
        """)
        self.conn.commit()

    def add_message(self, role: str, content: str) -> None:
        self.conn.execute(
            "INSERT INTO messages (role, content, ts) VALUES (?, ?, ?)",
            (role, content, time.time()),
        )
        self.conn.commit()

    def recent(self, n: int = 20) -> list[dict]:
        rows = self.conn.execute(
            "SELECT role, content FROM messages ORDER BY id DESC LIMIT ?", (n,)
        ).fetchall()
        return [{"role": r["role"], "content": r["content"]} for r in reversed(rows)]

    def set_fact(self, key: str, value: str) -> None:
        self.conn.execute(
            "INSERT OR REPLACE INTO facts (key, value, ts) VALUES (?, ?, ?)",
            (key, value, time.time()),
        )
        self.conn.commit()

    def get_fact(self, key: str, default: str = "") -> str:
        row = self.conn.execute("SELECT value FROM facts WHERE key=?", (key,)).fetchone()
        return row["value"] if row else default

    def all_facts(self) -> dict[str, str]:
        rows = self.conn.execute("SELECT key, value FROM facts").fetchall()
        return {r["key"]: r["value"] for r in rows}

    def set_pref(self, key: str, value: Any) -> None:
        self.conn.execute(
            "INSERT OR REPLACE INTO prefs (key, value) VALUES (?, ?)",
            (key, json.dumps(value, ensure_ascii=False)),
        )
        self.conn.commit()

    def get_pref(self, key: str, default: Any = None) -> Any:
        row = self.conn.execute("SELECT value FROM prefs WHERE key=?", (key,)).fetchone()
        if not row:
            return default
        try:
            return json.loads(row["value"])
        except Exception:
            return row["value"]

    def clear(self) -> None:
        self.conn.execute("DELETE FROM messages")
        self.conn.commit()

memory = Memory()
