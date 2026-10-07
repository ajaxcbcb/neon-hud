import { describe, it, expect } from 'vitest';
import { normalizeSettings, defaults, countdown, windowStatus, selectedNetwork, selectedDrives, drivePercent, sampleSystem, severity, selectedGpu, gpuLoad, gpuDetails } from './model';
describe('settings and measurement boundaries', () => {
  it('keeps startup and notifications off until selected', () => {
    expect(normalizeSettings(null)).toEqual(defaults);
    expect(normalizeSettings({launchAtLogin:true}).launchAtLogin).toBe(true);
    expect(defaults.notifications).toBe(false);
  });
  it('clamps malformed preferences and never inverts alert thresholds', () => {
    const value = normalizeSettings({theme:'invalid', opacity:0, textScale:99, warning:5, critical:80, completed:'yes'});
    expect(value.theme).toBe('circuit'); expect(value.opacity).toBe(.85);
    expect(value.textScale).toBe(1.15); expect(value.critical).toBe(5); expect(value.completed).toBe(false);
  });
  it('does not invent reset times or fresh readings', () => {
    expect(countdown(null, 10)).toBe('Reset not reported');
    expect(countdown(5, 10)).toBe('Awaiting refresh');
    expect(windowStatus({label:'5 hours',minutes:300,usedPercent:30,resetsAt:5}, 8, 10)).toBe('Awaiting refresh');
    expect(windowStatus({label:'Weekly',minutes:10080,usedPercent:30,resetsAt:9000}, 10, 800)).toBe('Stale reading');
  });
  it('selects one default route and leaves unknown routes unavailable', () => {
    expect(selectedNetwork(sampleSystem, 'auto')?.name).toBe('Sample Wi-Fi');
    expect(selectedNetwork({...sampleSystem,defaultInterface:null}, 'auto')).toBeUndefined();
    expect(selectedNetwork(sampleSystem, 'VPN')).toBeUndefined();
  });
  it('compares remaining quota with configured thresholds', () => {
    expect(severity(20,defaults)).toBe('warning'); expect(severity(10,defaults)).toBe('critical'); expect(severity(21,defaults)).toBe('normal');
  });
  it('keeps drive choices across disconnects and rejects malformed settings', () => {
    const settings = normalizeSettings({ storageDriveIds: ['sample-system', 'unplugged', 'sample-system', 99] });
    expect(settings.storageDriveIds).toEqual(['sample-system', 'unplugged']);
    expect(selectedDrives(sampleSystem, settings.storageDriveIds).map(d => d.id)).toEqual(['sample-system']);
    expect(selectedDrives(sampleSystem, []).length).toBe(2);
    expect(selectedDrives(null, [])).toEqual([]);
    expect(normalizeSettings({ storageDriveIds: 'C:' }).storageDriveIds).toEqual([]);
    expect(normalizeSettings({ metrics: { cpu: false } }).metrics.storage).toBe(true);
  });
  it('handles unavailable and out-of-range capacity without invented percentages', () => {
    const drive = sampleSystem.drives[0];
    expect(drivePercent(drive)).toBeCloseTo(59.765625);
    expect(drivePercent({ ...drive, totalBytes: 0 })).toBeNull();
    expect(drivePercent({ ...drive, usedBytes: drive.totalBytes * 2 })).toBe(100);
  });
});

describe('GPU identity and capability boundaries', () => {
  const gpu = { ...sampleSystem.gpus![0], sampledAt: 100 };
  it('selects the busiest fresh adapter or the exact saved adapter without fallback', () => {
    const snapshot = { ...sampleSystem, gpus: [gpu, { ...gpu, id: 'second', name: 'Discrete GPU', utilizationPercent: 91 }, { ...gpu, id: 'old', sampledAt: 1, utilizationPercent: 100 }] };
    expect(selectedGpu(snapshot, 'auto', 100)?.id).toBe('second');
    expect(selectedGpu(snapshot, gpu.id, 100)?.id).toBe(gpu.id);
    expect(selectedGpu(snapshot, 'disconnected', 100)).toBeUndefined();
    expect(selectedGpu(null, 'auto', 100)).toBeUndefined();
  });
  it('keeps unsupported, nonfinite, future and stale readings unavailable', () => {
    expect(gpuLoad({ ...gpu, utilizationPercent: 150 }, 100)).toBe(100);
    for (const bad of [{ ...gpu, utilizationPercent: null }, { ...gpu, utilizationPercent: NaN }, { ...gpu, sampledAt: 87 }, { ...gpu, sampledAt: 106 }, { ...gpu, status: 'unavailable' as const }]) expect(gpuLoad(bad, 100)).toBeNull();
    expect(gpuDetails({ ...gpu, sampledAt: 1 }, 100).join()).toContain('Stale');
    expect(gpuDetails({ ...gpu, memoryUsedBytes: null, temperatureCelsius: null }, 100).join()).toContain('unavailable');
  });
  it('migrates old preferences and constrains polling, adapter and threshold values', () => {
    const old = normalizeSettings({ metrics: { cpu: false }, resources: { adaptive: false } });
    expect(old.metrics.gpu).toBe(true); expect(old.gpuId).toBe('auto');
    expect(old.resources).toEqual({ adaptive: false, samplingMs: 250 });
    expect(normalizeSettings({ gpuId: ' ', resources: { samplingMs: 16 }, performance: { gpuPercent: 999 } })).toMatchObject({ gpuId: 'auto', resources: { samplingMs: 250 }, performance: { gpuPercent: 100 } });
  });
});
