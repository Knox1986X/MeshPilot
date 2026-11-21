// Кольцевой буфер истории для графиков (шаг ~1 с, окно 60 с).
// Недоступная точка — разрыв (null), а не ноль.

export class RingHistory<T> {
  private buf: (T | null)[];
  private index = 0;
  private count = 0;

  constructor(private readonly capacity: number) {
    if (capacity < 2) throw new Error('capacity must be >= 2');
    this.buf = new Array(capacity).fill(null);
  }

  push(value: T | null): void {
    this.buf[this.index] = value;
    this.index = (this.index + 1) % this.capacity;
    if (this.count < this.capacity) this.count += 1;
  }

  /** От старых к новым; длина == count. */
  toArray(): (T | null)[] {
    if (this.count < this.capacity) {
      return this.buf.slice(0, this.count);
    }
    return [...this.buf.slice(this.index), ...this.buf.slice(0, this.index)];
  }

  get size(): number {
    return this.count;
  }
}

export interface ChartPoint {
  value: number | null;
}

/** Строит SVG-polyline-точки из истории; null → пропуск сегмента. */
export function toPolylineSegments(
  history: (number | null)[],
  width: number,
  height: number,
  maxValue = 100,
): string[] {
  const n = history.length;
  if (n < 2) return [];
  const stepX = width / (n - 1);
  const segments: string[] = [];
  let current: string[] = [];
  for (let i = 0; i < n; i += 1) {
    const v = history[i];
    if (v == null) {
      if (current.length >= 2) segments.push(current.join(' '));
      current = [];
      continue;
    }
    const clamped = Math.max(0, Math.min(maxValue, v));
    const x = Number((i * stepX).toFixed(2));
    const y = Number((height - (clamped / maxValue) * (height - 4) - 2).toFixed(2));
    current.push(`${x},${y}`);
  }
  if (current.length >= 2) segments.push(current.join(' '));
  return segments;
}
