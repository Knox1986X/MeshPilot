"""Lip-sync: амплитуда аудио -> открытие рта (viseme approximation)."""
from __future__ import annotations
import io

def amplitude_from_wav(wav_bytes: bytes) -> float:
    """RMS амплитуда WAV, 0..1."""
    try:
        import soundfile as sf
        data, _sr = sf.read(io.BytesIO(wav_bytes))
        import numpy as np
        return float(min(1.0, np.sqrt((data ** 2).mean()) * 3.0))
    except Exception:
        return 0.0

def amplitude_from_pcm(pcm_int16) -> float:
    try:
        import numpy as np
        arr = np.asarray(pcm_int16, dtype=np.float32) / 32768.0
        return float(min(1.0, np.sqrt((arr ** 2).mean()) * 3.0))
    except Exception:
        return 0.0
