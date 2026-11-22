"""Тёмная тема MeshPilot (QSS): glassmorphism, акцент Jarvis-cyan."""

ACCENT = "#38bdf8"      # Jarvis cyan
ACCENT_DARK = "#0ea5e9"
BG = "#0f172a"          # slate-900
BG_CARD = "#1e293b"     # slate-800
BG_INPUT = "#0b1220"
TEXT = "#e2e8f0"
TEXT_DIM = "#94a3b8"
BORDER = "#334155"
SUCCESS = "#22c55e"
WARNING = "#f59e0b"
DANGER = "#ef4444"

QSS = f"""
QMainWindow {{ background: {BG}; color: {TEXT}; }}
QWidget {{ background: {BG}; color: {TEXT}; font-family: "Inter", "Segoe UI", sans-serif; font-size: 13px; }}
QLabel {{ color: {TEXT}; }}
QLabel#dim {{ color: {TEXT_DIM}; }}
QLabel#title {{ font-size: 20px; font-weight: 700; color: white; }}
QLabel#subtitle {{ font-size: 12px; color: {TEXT_DIM}; }}
QLabel#stat_value {{ font-size: 22px; font-weight: 700; color: {ACCENT}; }}
QLabel#stat_label {{ font-size: 11px; color: {TEXT_DIM}; text-transform: uppercase; }}

QPushButton {{
    background: {BG_CARD}; color: {TEXT}; border: 1px solid {BORDER};
    border-radius: 10px; padding: 9px 16px; font-weight: 600;
}}
QPushButton:hover {{ background: #2d3a4f; border-color: {ACCENT}; }}
QPushButton:pressed {{ background: {ACCENT_DARK}; }}
QPushButton#primary {{
    background: {ACCENT}; color: #082f49; border: none; font-weight: 700;
}}
QPushButton#primary:hover {{ background: #7dd3fc; }}
QPushButton#danger {{ background: #7f1d1d; border-color: {DANGER}; }}
QPushButton#nav {{
    background: transparent; border: none; text-align: left;
    padding: 12px 18px; font-size: 14px; border-radius: 0;
}}
QPushButton#nav:checked {{ background: {BG_CARD}; color: {ACCENT}; border-left: 3px solid {ACCENT}; }}
QPushButton#nav:hover {{ background: #1a2740; }}

QLineEdit, QTextEdit, QListWidget {{
    background: {BG_INPUT}; border: 1px solid {BORDER}; border-radius: 10px;
    padding: 8px; color: {TEXT}; selection-background-color: {ACCENT_DARK};
}}
QLineEdit:focus, QTextEdit:focus {{ border-color: {ACCENT}; }}
QComboBox {{
    background: {BG_CARD}; border: 1px solid {BORDER}; border-radius: 8px;
    padding: 6px 10px; color: {TEXT};
}}
QComboBox::drop-down {{ border: none; }}
QCheckBox {{ spacing: 8px; }}
QCheckBox::indicator {{ width: 16px; height: 16px; border-radius: 4px; border: 1px solid {BORDER}; background: {BG_INPUT}; }}
QCheckBox::indicator:checked {{ background: {ACCENT}; border-color: {ACCENT}; }}

QFrame#card {{
    background: {BG_CARD}; border: 1px solid {BORDER}; border-radius: 14px;
}}
QFrame#sidebar {{ background: #0b1220; border-right: 1px solid {BORDER}; }}
QFrame#topbar {{ background: #0b1220; border-bottom: 1px solid {BORDER}; }}

QProgressBar {{
    background: {BG_INPUT}; border: 1px solid {BORDER}; border-radius: 8px;
    text-align: center; color: {TEXT};
}}
QProgressBar::chunk {{ background: {ACCENT}; border-radius: 7px; }}

QScrollBar:vertical {{ background: transparent; width: 8px; margin: 0; }}
QScrollBar::handle:vertical {{ background: {BORDER}; border-radius: 4px; min-height: 30px; }}
QScrollBar::handle:vertical:hover {{ background: {ACCENT}; }}
QScrollBar::add-line:vertical, QScrollBar::sub-line:vertical {{ height: 0; }}
"""

def apply_theme(app) -> None:
    app.setStyleSheet(QSS)
