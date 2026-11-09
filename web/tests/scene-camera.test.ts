import { describe, expect, it } from 'vitest';
import { clampPan, clampZoom, easeInOut, panLimit, rotateDelta, smoothFactor, stepTween, wheelFactor, ZOOM_MAX, ZOOM_MIN } from '../src/scene/cameraMath';
import { createSceneBus } from '../src/store/scene-bus';

describe('camera maths', () => {
  it('rotateDelta takes the shortest way round', () => {
    expect(rotateDelta(0, 1)).toBe(1);
    expect(rotateDelta(0, 3)).toBe(-1);
    expect(rotateDelta(3, 0)).toBe(1);
    expect(rotateDelta(1, 3)).toBe(2);
    expect(rotateDelta(2, 2)).toBe(0);
  });
  it('four quarter turns come back to the start', () => {
    let v = 0, total = 0;
    for (let i = 0; i < 4; i++) { const n = (v + 1) % 4; total += rotateDelta(v, n); v = n; }
    expect(total).toBe(4);
    expect(v).toBe(0);
  });
  it('zoom is clamped and the wheel direction is right', () => {
    expect(clampZoom(100)).toBe(ZOOM_MAX);
    expect(clampZoom(0.01)).toBe(ZOOM_MIN);
    expect(wheelFactor(-100)).toBeGreaterThan(1);
    expect(wheelFactor(100)).toBeLessThan(1);
    expect(wheelFactor(1e9)).toBeGreaterThan(0.5);
  });
  it('pan limit grows with zoom and clamps both axes', () => {
    expect(panLimit(12, 3)).toBeGreaterThan(panLimit(12, 1));
    expect(panLimit(12, 0.8)).toBe(panLimit(12, 1));
    expect(clampPan(50, -50, 12, 1)).toEqual([panLimit(12, 1), -panLimit(12, 1)]);
    expect(clampPan(0.5, 0.5, 12, 2)).toEqual([0.5, 0.5]);
  });
  it('tween eases from start to end and finishes', () => {
    expect(easeInOut(0)).toBe(0);
    expect(easeInOut(1)).toBe(1);
    expect(easeInOut(0.5)).toBeCloseTo(0.5);
    let t = { from: 0, to: Math.PI / 2, elapsed: 0, duration: 400 };
    let r = stepTween(t, 100);
    expect(r.done).toBe(false);
    expect(r.value).toBeGreaterThan(0);
    expect(r.value).toBeLessThan(Math.PI / 2);
    t = r.tween;
    r = stepTween(t, 400);
    expect(r).toMatchObject({ done: true, value: Math.PI / 2 });
    expect(stepTween({ from: 0, to: 1, elapsed: 0, duration: 0 }, 16)).toMatchObject({ done: true, value: 1 });
  });
  it('smoothing factor is in 0..1', () => {
    expect(smoothFactor(0)).toBe(0);
    expect(smoothFactor(1000)).toBeLessThanOrEqual(1);
  });
});

describe('scene bus', () => {
  it('forwards zoom and reset requests to listeners and stops after unsubscribe', () => {
    const bus = createSceneBus();
    const got: unknown[] = [];
    const off = bus.onCommand((c) => got.push(c));
    bus.requestZoom(1.2);
    bus.requestReset();
    off();
    bus.requestZoom(2);
    expect(got).toEqual([{ type: 'zoom', factor: 1.2 }, { type: 'reset' }]);
  });
  it('readables follow the store contract', () => {
    const bus = createSceneBus();
    const seen: unknown[] = [];
    bus.hover.subscribe((v) => seen.push(v));
    bus.hover.set({ tile: { x: 1, y: 2 }, entity: null, invalidReason: 'Outside the map', buildKind: null });
    expect(seen).toHaveLength(2);
    expect(bus.hover.get()!.invalidReason).toBe('Outside the map');
  });
});
