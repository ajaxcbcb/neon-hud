import type { Settings, SystemSnapshot } from './model';

export type ResourceMode = 'normal' | 'pressure' | 'critical';
export interface ResourcePolicy {
  mode: ResourceMode;
  systemSeconds: number;
  providerSeconds: number;
  quiet: boolean;
  reasons: string[];
  suggestions: string[];
}
export const resourceBudget = (mode: ResourceMode) => ({
  systemSeconds: mode === 'critical' ? 8 : mode === 'pressure' ? 4 : 2,
  providerSeconds: mode === 'critical' ? 15 : mode === 'pressure' ? 10 : 5,
  quiet: mode !== 'normal',
});

// Escalation uses consecutive real samples. Recovery requires fresh, comfortably
// healthy readings for 20 seconds at each step; missing data cannot prove recovery.
export class ResourceGovernor {
  private mode: ResourceMode = 'normal';
  private sampleTime = -1;
  private periods = new Map<string, { start: number; last: number }>();
  private healthySince: number | null = null;
  private reasons: string[] = [];
  private suggestions: string[] = [];
  update(snapshot: SystemSnapshot | null, settings: Settings, now: number): ResourcePolicy {
    const result = () => ({ mode: this.mode, ...resourceBudget(this.mode), reasons: [...this.reasons], suggestions: [...this.suggestions] });
    if (!settings.resources.adaptive) {
      this.mode = 'normal'; this.periods.clear(); this.healthySince = null; this.sampleTime = -1;
      this.reasons = ['Adaptive resource management is off.']; this.suggestions = [];
      return result();
    }
    if (!snapshot || now - snapshot.sampledAt > 12 || snapshot.sampledAt > now + 5) {
      this.periods.clear(); this.healthySince = null;
      this.reasons = ['Waiting for fresh system readings.'];
      return result();
    }
    const ram = snapshot.memory.total > 0 ? snapshot.memory.used / snapshot.memory.total * 100 : null;
    const cpu = snapshot.cpu;
    const temps = snapshot.temperatures.filter(t => Number.isFinite(t.celsius));
    const limit = settings.performance;
    const hot = temps.filter(t => t.celsius >= limit.temperatureCelsius);
    const veryHot = hot.some(t => t.celsius >= limit.temperatureCelsius + 10);
    const thresholds = [
      ['cpu', cpu !== null && cpu >= limit.cpuPercent],
      ['ram', ram !== null && ram >= limit.memoryPercent],
      ['cpu-critical', cpu !== null && cpu >= Math.max(98, limit.cpuPercent)],
      ['ram-critical', ram !== null && ram >= Math.max(97, limit.memoryPercent)],
    ] as const;
    if (snapshot.sampledAt !== this.sampleTime) {
      for (const [key, active] of thresholds) {
        const prior = this.periods.get(key);
        if (!active) this.periods.delete(key);
        else this.periods.set(key, { start: prior && snapshot.sampledAt > prior.last && snapshot.sampledAt - prior.last <= 12 ? prior.start : snapshot.sampledAt, last: snapshot.sampledAt });
      }
      this.sampleTime = snapshot.sampledAt;
    }
    const sustained = (key: string) => { const p = this.periods.get(key); return !!p && p.last - p.start >= 10; };
    const cpuPressure = sustained('cpu'), ramPressure = sustained('ram');
    const desired: ResourceMode = veryHot || sustained('cpu-critical') || sustained('ram-critical') ? 'critical' : hot.length || cpuPressure || ramPressure ? 'pressure' : 'normal';
    const rank = { normal: 0, pressure: 1, critical: 2 };
    if (rank[desired] > rank[this.mode]) { this.mode = desired; this.healthySince = null; }
    const healthy = cpu !== null && ram !== null && cpu < limit.cpuPercent - 5 && ram < limit.memoryPercent - 5 && temps.every(t => t.celsius < limit.temperatureCelsius - 5);
    if (this.mode !== 'normal' && healthy) {
      this.healthySince ??= now;
      if (now - this.healthySince >= 20) {
        this.mode = this.mode === 'critical' ? 'pressure' : 'normal';
        this.healthySince = this.mode === 'normal' ? null : now;
      }
    } else this.healthySince = null;
    this.reasons = [
      ...(cpuPressure ? [`Sustained CPU load ${cpu!.toFixed(0)}%.`] : []),
      ...(ramPressure ? [`Sustained memory use ${ram!.toFixed(0)}%.`] : []),
      ...hot.map(t => `${t.label}: ${t.celsius.toFixed(0)} °C reported.`),
    ];
    if (!this.reasons.length) this.reasons = [this.mode === 'normal' ? 'Normal monitoring budget.' : 'Recovering · waiting for 20 seconds of healthy readings per step.'];
    this.suggestions = [
      ...(cpuPressure ? ['Review busy apps in Task Manager or Activity Monitor; pause work you no longer need.'] : []),
      ...(ramPressure ? ['Close unused apps or tabs to free memory.'] : []),
      ...(hot.length ? ['Check ventilation and reduce heavy work while temperatures settle. Throttling is not measured.'] : []),
    ];
    return result();
  }
}
