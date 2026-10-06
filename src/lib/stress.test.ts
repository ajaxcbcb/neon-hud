import { describe, expect, it } from 'vitest';
import { stressColor, stressGradient, stressPercent } from './stress';
describe('stress color scales', () => {
  it('maps measured load from green through amber to hot pink and reverses allowance', () => {
    expect(stressColor(0)).toBe('rgb(130, 245, 162)');
    expect(stressColor(70)).toBe('rgb(255, 209, 102)');
    expect(stressColor(100)).toBe('rgb(255, 102, 142)');
    expect(stressColor(10, true)).toBe(stressColor(90));
    expect(stressColor(100, true)).toBe(stressColor(0));
    expect(stressGradient(30)).not.toContain(stressColor(100));
    expect(stressGradient(100)).toContain('70%');
  });
  it('clamps out of range input and leaves unknown measurements neutral', () => {
    expect(stressPercent(-10)).toBe(0);
    expect(stressPercent(200)).toBe(100);
    expect(stressColor(null)).toBe('var(--muted)');
    expect(stressGradient(Number.NaN)).toBe('var(--muted)');
  });
});
