import { describe, expect, it } from 'vitest';
import { defaults, normalizeSettings, sampleSystem } from './model';
import { ResourceGovernor } from './resources';
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
