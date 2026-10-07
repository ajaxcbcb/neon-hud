import { describe, expect, it } from 'vitest';
import { defaults } from './model';
import { dockPosition, windowSize } from './layout';
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
});
