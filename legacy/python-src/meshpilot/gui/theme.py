"""Тёмная тема MeshPilot (как MeshPrime Dashboard на фото)."""

ACCENT = "#8b7cf6"      # фиолетовый акцент (как на фото)
ACCENT2 = "#38bdf8"     # cyan для Jarvis
BG = "#0a0e1a"          # почти чёрный фон
BG_CARD = "#111827"     # карточки
BG_INPUT = "#0d1424"
TEXT = "#e5e7eb"
TEXT_DIM = "#6b7280"
BORDER = "#1f2937"
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
QLabel#stat_value {{ font-size: 22px; font-weight: 700; color: white; }}
QLabel#stat_label {{ font-size: 11px; color: {TEXT_DIM}; text-transform: uppercase; }}
QLabel#section {{ font-size: 10px; color: {TEXT_DIM}; font-weight: 700; letter-spacing: 1px; }}

QLabel#pill, QLabel#pill_green, QLabel#pill_purple {{
    border-radius: 12px; padding: 4px 12px; font-size: 11px; font-weight: 600;
}}
QLabel#pill {{ background: #1f2937; color: {TEXT_DIM}; border: 1px solid {BORDER}; }}
QLabel#pill_green {{ background: #052e16; color: #4ade80; border: 1px solid #14532d; }}
QLabel#pill_purple {{ background: #2e1065; color: #c4b5fd; border: 1px solid #4c1d95; }}
QLabel#chip {{ background: #1f2937; color: {TEXT_DIM}; border-radius: 8px; padding: 2px 8px; font-size: 10px; }}
QLabel#chip_purple {{ background: #2e1065; color: #c4b5fd; border-radius: 8px; padding: 2px 8px; font-size: 10px; font-weight: 700; }}
QLabel#chip_green {{ background: #052e16; color: #4ade80; border-radius: 8px; padding: 2px 8px; font-size: 10px; font-weight: 700; }}
QLabel#chip_yellow {{ background: #422006; color: #fbbf24; border-radius: 8px; padding: 2px 8px; font-size: 10px; font-weight: 700; }}

QPushButton {{
    background: {BG_CARD}; color: {TEXT}; border: 1px solid {BORDER};
    border-radius: 10px; padding: 9px 16px; font-weight: 600;
}}
QPushButton:hover {{ background: #1a2436; border-color: {ACCENT}; }}
QPushButton:pressed {{ background: #2e1065; }}
QPushButton#primary {{
    background: {ACCENT}; color: white; border: none; font-weight: 700;
}}
QPushButton#primary:hover {{ background: #a78bfa; }}
QPushButton#nav {{
    background: transparent; border: none; text-align: left;
    padding: 10px 14px; font-size: 13px; border-radius: 8px; color: {TEXT_DIM};
}}
QPushButton#nav:checked {{ background: #1e1b4b; color: white; }}
QPushButton#nav:hover {{ background: #111827; color: {TEXT}; }}
QPushButton#nav_phase {{ color: {TEXT_DIM}; }}
QPushButton#nav_phase:hover {{ color: {TEXT}; }}

QLineEdit, QTextEdit, QListWidget {{
    background: {BG_INPUT}; border: 1px solid {BORDER}; border-radius: 10px;
    padding: 8px; color: {TEXT}; selection-background-color: #4c1d95;
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
QFrame#sidebar {{ background: #0d1117; border-right: 1px solid {BORDER}; }}
QFrame#topbar {{ background: #0d1117; border-bottom: 1px solid {BORDER}; }}

QProgressBar {{
    background: #1f2937; border: none; border-radius: 4px;
    text-align: center; color: {TEXT};
}}
QProgressBar::chunk {{ background: {ACCENT}; border-radius: 3px; }}

QScrollBar:vertical {{ background: transparent; width: 8px; margin: 0; }}
QScrollBar::handle:vertical {{ background: #374151; border-radius: 4px; min-height: 30px; }}
QScrollBar::handle:vertical:hover {{ background: {ACCENT}; }}
QScrollBar::add-line:vertical, QScrollBar::sub-line:vertical {{ height: 0; }}
"""

def apply_theme(app) -> None:
    app.setStyleSheet(QSS)
