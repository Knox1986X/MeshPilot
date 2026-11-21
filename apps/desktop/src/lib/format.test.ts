import { describe, expect, it } from 'vitest';
import { formatBytes, formatPercent, formatRate, formatUptime, looksLikeSafeHomePath } from './format';
import { RingHistory, toPolylineSegments } from './history';

describe('formatBytes', () => {
  it('null/NaN/отрицательные → тире', () => {
    expect(formatBytes(null)).toBe('—');
    expect(formatBytes(Number.NaN)).toBe('—');
    expect(formatBytes(-1)).toBe('—');
  });
  it('единицы измерения', () => {
    expect(formatBytes(0)).toBe('0 Б');
    expect(formatBytes(512)).toBe('512 Б');
    expect(formatBytes(2048)).toBe('2.0 КБ');
    expect(formatBytes(5 * 1024 * 1024)).toBe('5.0 МБ');
    expect(formatBytes(3 * 1024 ** 3)).toBe('3.0 ГБ');
    expect(formatBytes(2 * 1024 ** 4)).toBe('2.0 ТБ');
  });
});

describe('formatRate / formatPercent / formatUptime', () => {
  it('скорость сети', () => {
    expect(formatRate(null)).toBe('—');
    expect(formatRate(1500)).toBe('1.5 КБ/с');
    expect(formatRate(2_500_000)).toBe('2.4 МБ/с');
  });
  it('процент', () => {
    expect(formatPercent(null)).toBe('—');
    expect(formatPercent(3.25)).toBe('3.3%');
    expect(formatPercent(87.4)).toBe('87%');
  });
  it('аптайм', () => {
    expect(formatUptime(0)).toBe('0 мин');
    expect(formatUptime(3600 * 5 + 60 * 12)).toBe('5 ч 12 мин');
    expect(formatUptime(86400 * 3 + 3600 * 4)).toBe('3 д 4 ч');
    expect(formatUptime(-5)).toBe('—');
  });
});

describe('looksLikeSafeHomePath', () => {
  it('принимает обычные абсолютные пути', () => {
    expect(looksLikeSafeHomePath('/home/u/.cache/pip')).toBe(true);
  });
  it('отклоняет выход за пределы', () => {
    expect(looksLikeSafeHomePath('/home/u/../etc/passwd')).toBe(false);
    expect(looksLikeSafeHomePath('relative/path')).toBe(false);
    expect(looksLikeSafeHomePath('')).toBe(false);
  });
});

describe('RingHistory', () => {
  it('вытесняет старые точки, сохраняет порядок', () => {
    const h = new RingHistory<number>(3);
    h.push(1); h.push(2); h.push(3); h.push(4);
    expect(h.toArray()).toEqual([2, 3, 4]);
  });
  it('null сохраняется как разрыв', () => {
    const h = new RingHistory<number>(4);
    h.push(10); h.push(null); h.push(30);
    expect(h.toArray()).toEqual([10, null, 30]);
  });
  it('отказ на capacity < 2', () => {
    expect(() => new RingHistory<number>(1)).toThrow();
  });
});

describe('toPolylineSegments', () => {
  it('строит сегменты без null-точек', () => {
    const segs = toPolylineSegments([1, null, 50, 100], 100, 40, 100);
    expect(segs.length).toBe(1);
    expect(segs[0]).toMatch(/^[\d., -]+$/);
  });
  it('один сегмент на непрерывный ряд', () => {
    const segs = toPolylineSegments([0, 25, 50, 75, 100], 100, 40, 100);
    expect(segs).toHaveLength(1);
  });
  it('малый ряд → пусто', () => {
    expect(toPolylineSegments([42], 100, 40)).toEqual([]);
  });
});
