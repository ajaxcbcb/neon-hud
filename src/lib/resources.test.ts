import { describe, expect, it } from 'vitest';
import { defaults, normalizeSettings, sampleSystem } from './model';
import { ResourceGovernor, resourceBudget } from './resources';
const sample = (time: number, cpu = 40, ram = 50, temp = 50) => ({ ...sampleSystem, sampledAt: time, cpu, memory: { total: 100, used: ram, available: 100 - ram }, temperatures: [{ label: 'CPU Package', celsius: temp }] });
describe('adaptive monitoring budgets', () => {
  it('requires sustained workload, escalates, and preserves warnings across slower samples', () => {
    const governor = new ResourceGovernor();
    expect(governor.update(sample(100, 95), defaults, 100).mode).toBe('normal');
    for (let t = 102; t < 110; t += 2) governor.update(sample(t, 95), defaults, t);
    expect(governor.update(sample(110, 95), defaults, 110)).toMatchObject({ mode: 'pressure', systemSeconds: 4, providerSeconds: 10, quiet: true });
    governor.update(sample(114, 99, 98), defaults, 114);
    governor.update(sample(118, 99, 98), defaults, 118);
    expect(governor.update(sample(126, 99, 98), defaults, 126)).toMatchObject({ mode: 'critical', systemSeconds: 8, providerSeconds: 15 });
    expect(governor.update(sample(134, 99, 98), defaults, 134).reasons.join()).toContain('CPU');
  });
  it('responds to real heat immediately and recovers one step after each healthy 20s', () => {
    const governor = new ResourceGovernor();
    expect(governor.update(sample(100, 40, 50, 95), defaults, 100).mode).toBe('critical');
    for (let t = 108; t <= 124; t += 8) expect(governor.update(sample(t), defaults, t).mode).toBe('critical');
    expect(governor.update(sample(132), defaults, 132).mode).toBe('pressure');
    for (let t = 136; t < 152; t += 4) expect(governor.update(sample(t), defaults, t).mode).toBe('pressure');
    expect(governor.update(sample(152), defaults, 152)).toMatchObject({ mode: 'normal', quiet: false });
  });
  it('does not treat stale/unknown samples as recovery or accumulate gaps as sustained load', () => {
    const governor = new ResourceGovernor();
    governor.update(sample(1, 99), defaults, 1);
    expect(governor.update(sample(40, 99), defaults, 40).mode).toBe('normal');
    governor.update(sample(42, 40, 50, 96), defaults, 42);
    expect(governor.update(sample(42), defaults, 100).mode).toBe('critical');
    expect(governor.update(null, defaults, 150).mode).toBe('critical');
    expect(governor.update(sample(152), defaults, 152).mode).toBe('critical');
    expect(governor.update(sample(160), normalizeSettings({ resources: { adaptive: false } }), 160).mode).toBe('normal');
    expect(normalizeSettings({ resources: { adaptive: 'bad' } }).resources.adaptive).toBe(true);
  });
  it('keeps low drive space as a capacity warning without misclassifying it as compute pressure', () => {
    const governor = new ResourceGovernor();
    expect(governor.update({ ...sample(100), drives: [{ ...sampleSystem.drives[0], availableBytes: 1 }] }, defaults, 100).mode).toBe('normal');
  });
});

describe('GPU budgets and recovery', () => {
  const gpu = { ...sampleSystem.gpus![0], utilizationPercent: 95, temperatureCelsius: null };
  const withGpu = (time: number, gpuTime = time, load: number | null = 95, temperature: number | null = null) => ({ ...sample(time), gpus: [{ ...gpu, sampledAt: gpuTime, utilizationPercent: load, temperatureCelsius: temperature }] });
  it('keeps fast hardware sampling independent of GPU and provider clocks', () => {
    expect(resourceBudget('normal', 250)).toEqual({ systemSeconds: .25, providerSeconds: 5, quiet: false });
    expect(resourceBudget('normal', 2000).systemSeconds).toBe(2);
    expect(resourceBudget('normal', 16).systemSeconds).toBe(.25);
    expect(resourceBudget('critical', 250)).toEqual({ systemSeconds: 8, providerSeconds: 15, quiet: true });
    const governor = new ResourceGovernor();
    for (let t = 100; t <= 110; t++) expect(governor.update(withGpu(t, 100), defaults, t).mode).toBe('normal');
    expect(governor.update(withGpu(111), defaults, 111).mode).toBe('pressure');
  });
  it('holds GPU pressure across missing, stale and unsupported recovery readings', () => {
    const governor = new ResourceGovernor();
    for (let t = 100; t <= 110; t++) governor.update(withGpu(t), defaults, t);
    for (let t = 111; t <= 140; t++) expect(governor.update({ ...sample(t), gpus: [] }, defaults, t).mode).toBe('pressure');
    for (let t = 141; t <= 165; t++) expect(governor.update(withGpu(t, t, null), defaults, t).mode).toBe('pressure');
    for (let t = 166; t <= 190; t++) expect(governor.update(withGpu(t, 140, 30), defaults, t).mode).toBe('pressure');
    for (let t = 191; t < 211; t++) expect(governor.update(withGpu(t, t, 30), defaults, t).mode).toBe('pressure');
    expect(governor.update(withGpu(211, 211, 30), defaults, 211).mode).toBe('normal');
  });
  it('requires a real cooling reading after measured GPU heat', () => {
    const governor = new ResourceGovernor();
    expect(governor.update(withGpu(100, 100, null, 96), defaults, 100).mode).toBe('critical');
    for (let t = 101; t <= 125; t++) expect(governor.update(withGpu(t, t, null, null), defaults, t).mode).toBe('critical');
    for (let t = 126; t < 146; t++) expect(governor.update(withGpu(t, t, null, 50), defaults, t).mode).toBe('critical');
    expect(governor.update(withGpu(146, 146, null, 50), defaults, 146).mode).toBe('pressure');
    expect(governor.update(withGpu(147, 147, null, null), normalizeSettings({ resources: { adaptive: false } }), 147).mode).toBe('normal');
  });
});
