import { describe, expect, it } from 'vitest';
import { defaults } from './model';
import { displayIndexAt, dockPosition, floatingPosition, popoverDirection, rememberPosition, windowSize } from './layout';
describe('pill desktop geometry', () => {
  const area = { x: -1920, y: 0, width: 1920, height: 1040 };
  it('reserves only the capsule at rest and accommodates readability scaling', () => {
    expect(windowSize(defaults, false, false)).toMatchObject({ width: 280, height: 56 });
    expect(windowSize({ ...defaults, textScale: 1.15 }, false, false, 192).height).toBeCloseTo(220.8);
    expect(windowSize(defaults, true, false).width).toBe(760);
  });
  it('keeps middle and bottom capsules stationary as popovers open', () => {
    const small = dockPosition('middle-right', area, { width: 280, height: 56 }, 1, true);
    const tall = dockPosition('middle-right', area, { width: 280, height: 192 }, 1, true);
    expect(tall).toEqual(small);
    const bottom = dockPosition('bottom-right', area, { width: 280, height: 56 }, 1, true);
    const bottomTall = dockPosition('bottom-right', area, { width: 280, height: 192 }, 1, true);
    expect(bottomTall.y + 192).toBe(bottom.y + 56);
    expect(small.x).toBe(-300);
  });
  it('uses physical DPI and clamps an oversized utility to the work area', () => {
    expect(dockPosition('middle-left', { x: 0, y: 0, width: 100, height: 80 }, { width: 280, height: 192 }, 2, true)).toEqual({ x: 0, y: 0 });
    expect(dockPosition('top-left', area, { width: 560, height: 112 }, 2, true)).toEqual({ x: -1880, y: 40 });
  });
  it('compresses only the resting pill and leaves details readable', () => {
    const settings = { ...defaults, size: 'compressed' as const };
    expect(windowSize(settings, false, false)).toMatchObject({ width: 160, height: 56 });
    expect(windowSize(settings, false, false, 208)).toMatchObject({ width: 280, height: 208 });
    expect(windowSize(settings, false, true).width).toBe(680);
  });
  it('retains a dragged location across monitor ordering and DPI changes', () => {
    const display = { name: 'Left monitor', scaleFactor: 2, area: { x: -1920, y: -100, width: 1920, height: 1080 } };
    const saved = rememberPosition({ x: -1520, y: 200 }, [display], defaults, { width: 560, height: 112 });
    expect(saved).toEqual({ x: 200, y: 150, monitor: 'Left monitor' });
    const settings = { ...defaults, windowPosition: saved };
    const changed = { ...display, scaleFactor: 1 };
    expect(floatingPosition(settings, [{ name: 'Other', scaleFactor: 1, area }, changed], { width: 280, height: 56 })).toEqual({ x: -1720, y: 50 });
  });
  it('keeps a compressed bottom-right capsule anchored while opening and moving details', () => {
    const display = { name: 'Main', scaleFactor: 1, area: { x: 0, y: 0, width: 1920, height: 1040 } };
    const settings = { ...defaults, size: 'compressed' as const, corner: 'bottom-right' as const, windowPosition: { x: 1700, y: 900, monitor: 'Main' } };
    expect(floatingPosition(settings, [display], { width: 160, height: 56 })).toEqual({ x: 1700, y: 900 });
    const point = floatingPosition(settings, [display], { width: 280, height: 208 }, true)!;
    expect(point).toEqual({ x: 1580, y: 748 });
    expect(rememberPosition(point, [display], settings, { width: 280, height: 208 }, true)).toEqual(settings.windowPosition);
  });
  it('recovers on a remaining monitor and clamps saved positions to its usable area', () => {
    const display = { name: 'Laptop', scaleFactor: 1, area: { x: 0, y: 0, width: 800, height: 600 } };
    const settings = { ...defaults, monitor: 99, windowPosition: { x: 1700, y: -100, monitor: 'Unplugged' } };
    expect(floatingPosition(settings, [display], { width: 280, height: 56 })).toEqual({ x: 520, y: 0 });
    expect(floatingPosition(settings, [], { width: 280, height: 56 })).toBeNull();
    expect(rememberPosition({ x: 0, y: 0 }, [], settings, { width: 280, height: 56 })).toBeNull();
  });
  it('opens custom popovers toward free space regardless of the docking preset', () => {
    const display = { name: 'Main', scaleFactor: 1, area: { x: 0, y: 0, width: 1920, height: 1040 } };
    for (const [x, y, left, up] of [[20, 20, false, false], [1700, 20, true, false], [20, 950, false, true], [1700, 950, true, true]] as const) {
      const settings = { ...defaults, size: 'compressed' as const, corner: left ? 'top-left' as const : 'bottom-right' as const,
        windowPosition: { x, y, monitor: 'Main' } };
      const direction = popoverDirection(settings, [display]);
      expect(direction).toEqual({ left, up });
      const point = floatingPosition(settings, [display], { width: 280, height: 392 }, true)!;
      expect(point).toEqual({ x: x - (left ? 120 : 0), y: y - (up ? 336 : 0) });
      expect(rememberPosition(point, [display], settings, { width: 280, height: 392 }, true, direction)).toEqual(settings.windowPosition);
    }
  });
  it('restores the actual selected display when two monitors share a name', () => {
    const displays = [0, 1920].map(x => ({ name: 'Identical', scaleFactor: 1, area: { x, y: 0, width: 1920, height: 1040 } }));
    const point = { x: 2100, y: 100 };
    const size = { width: 160, height: 56 };
    const monitor = displayIndexAt(point, displays, size);
    expect(monitor).toBe(1);
    const settings = { ...defaults, monitor, windowPosition: rememberPosition(point, displays, defaults, size) };
    expect(floatingPosition(settings, displays, size)).toEqual(point);
  });
});
