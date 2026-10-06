import { describe, expect, it } from 'vitest';
import { defaults, normalizeSettings, sampleSystem } from './model';
import { PressureTracker } from './pressure';

describe('measured pressure warnings', () => {
  it('waits for sustained workload and drops stale or interrupted evidence', () => {
    const tracker = new PressureTracker();
    const snapshot = { ...sampleSystem, cpu: 95, memory: { total: 100, used: 95, available: 5 }, sampledAt: 100 };
    tracker.observe(snapshot, defaults);
    expect(tracker.reading(snapshot, defaults, 100).cpu).toEqual([]);
    for (let time = 102; time <= 110; time += 2) tracker.observe({ ...snapshot, sampledAt: time }, defaults);
    expect(tracker.reading({ ...snapshot, sampledAt: 110 }, defaults, 110).cpu).toHaveLength(1);
    expect(tracker.reading({ ...snapshot, sampledAt: 110 }, defaults, 110).memory).toHaveLength(1);
    expect(tracker.reading({ ...snapshot, sampledAt: 110 }, defaults, 123).cpu).toEqual([]);
    tracker.observe({ ...snapshot, sampledAt: 140 }, defaults);
    expect(tracker.reading({ ...snapshot, sampledAt: 140 }, defaults, 140).cpu).toEqual([]);
  });
  it('labels actual sensors and full volumes, without inventing a CPU temperature', () => {
    const tracker = new PressureTracker();
    const snapshot = { ...sampleSystem, sampledAt: 100, temperatures: [{ label: 'GPU', celsius: 96 }], drives: [{ ...sampleSystem.drives[0], usedBytes: 510 * 1073741824, availableBytes: 2 * 1073741824 }] };
    tracker.observe(snapshot, defaults);
    const alerts = tracker.reading(snapshot, defaults, 100);
    expect(alerts.cpu).toEqual([]);
    expect(alerts.temperature[0]).toContain('GPU: 96.0');
    expect(alerts.drives['sample-system']).toContain('low free space');
    expect(tracker.reading({ ...snapshot, temperatures: [] }, defaults, 100).temperature).toEqual([]);
  });
  it('normalizes thresholds and respects the configured temperature limit', () => {
    const settings = normalizeSettings({ performance: { cpuPercent: 3, temperatureCelsius: 1000 } });
    expect(settings.performance.cpuPercent).toBe(50);
    expect(settings.performance.temperatureCelsius).toBe(120);
    const tracker = new PressureTracker();
    expect(tracker.reading({ ...sampleSystem, sampledAt: 1, temperatures: [{ label: 'CPU Package', celsius: 95 }] }, settings, 1).temperature).toEqual([]);
  });
});
