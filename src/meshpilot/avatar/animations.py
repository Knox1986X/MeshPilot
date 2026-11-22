"""Анимации персонажа: idle, blink, bounce, wave, sleep, react."""
from __future__ import annotations
import math

def idle_bounce(t: float) -> float:
    return math.sin(t * 2.0) * 4.0

def blink_curve(t: float, period: float = 3.2, duration: float = 0.12) -> float:
    return 1.0 if (t % period) < duration else 0.0

def wave(t: float) -> float:
    return math.sin(t * 10.0) * 0.3

def sleep_breath(t: float) -> float:
    return math.sin(t * 1.2) * 0.05

def react_happy(t: float) -> float:
    return abs(math.sin(t * 6.0)) * 0.15
