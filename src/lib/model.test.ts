import { describe, it, expect } from 'vitest';
import { normalizeSettings, defaults, countdown, windowStatus, selectedNetwork, sampleSystem, severity } from './model';
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
});
