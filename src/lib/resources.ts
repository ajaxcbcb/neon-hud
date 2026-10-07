import type { Settings, SystemSnapshot } from './model';
import { gpuCurrent, gpuLoad } from './model';

export type ResourceMode = 'normal' | 'pressure' | 'critical';
export interface ResourcePolicy {
  mode: ResourceMode;
  systemSeconds: number;
  providerSeconds: number;
  quiet: boolean;
  reasons: string[];
  suggestions: string[];
}
export const resourceBudget = (mode: ResourceMode, samplingMs = 250) => ({
  systemSeconds: mode === 'critical' ? 8 : mode === 'pressure' ? 4 : ([250, 500, 1000, 2000].includes(samplingMs) ? samplingMs : 250) / 1000,
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
  private recoveryGpus = new Map<string, { load: boolean; temperature: boolean }>();
  private reasons: string[] = [];
  private suggestions: string[] = [];
  update(snapshot: SystemSnapshot | null, settings: Settings, now: number): ResourcePolicy {
    const result = () => ({ mode: this.mode, ...resourceBudget(this.mode, settings.resources.samplingMs), reasons: [...this.reasons], suggestions: [...this.suggestions] });
    if (!settings.resources.adaptive) {
      this.mode = 'normal'; this.periods.clear(); this.recoveryGpus.clear(); this.healthySince = null; this.sampleTime = -1;
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
    const gpus = (snapshot.gpus || []).filter(gpu => gpuCurrent(gpu, now));
    const gpuTemps = gpus.filter(gpu => Number.isFinite(gpu.temperatureCelsius)).map(gpu => ({ label: gpu.name, celsius: gpu.temperatureCelsius! }));
    const hot = [...temps, ...gpuTemps].filter(t => t.celsius >= limit.temperatureCelsius);
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
    const present = new Set(gpus.map(gpu => `gpu:${gpu.id}`));
    for (const key of this.periods.keys()) if (key.startsWith('gpu:') && !present.has(key.replace('gpu:critical:', 'gpu:'))) this.periods.delete(key);
    for (const gpu of gpus) for (const [key, threshold] of [[`gpu:${gpu.id}`, limit.gpuPercent], [`gpu:critical:${gpu.id}`, Math.max(98, limit.gpuPercent)]] as const) {
      const prior = this.periods.get(key), load = gpuLoad(gpu, now);
      if (load === null || load < threshold) this.periods.delete(key);
      else if (prior?.last !== gpu.sampledAt) this.periods.set(key, { start: prior && gpu.sampledAt > prior.last && gpu.sampledAt - prior.last <= 12 ? prior.start : gpu.sampledAt, last: gpu.sampledAt });
    }
    const sustained = (key: string) => { const p = this.periods.get(key); return !!p && p.last - p.start >= 10; };
    const cpuPressure = sustained('cpu'), ramPressure = sustained('ram');
    const gpuPressure = gpus.filter(gpu => sustained(`gpu:${gpu.id}`));
    const desired: ResourceMode = veryHot || sustained('cpu-critical') || sustained('ram-critical') || gpus.some(gpu => sustained(`gpu:critical:${gpu.id}`)) ? 'critical' : hot.length || cpuPressure || ramPressure || gpuPressure.length ? 'pressure' : 'normal';
    const rank = { normal: 0, pressure: 1, critical: 2 };
    if (rank[desired] > rank[this.mode]) { this.mode = desired; this.healthySince = null; }
    if (this.mode !== 'normal') for (const gpu of gpus) {
      const load = gpuPressure.some(active => active.id === gpu.id);
      const temperature = Number.isFinite(gpu.temperatureCelsius) && gpu.temperatureCelsius! >= limit.temperatureCelsius;
      if (load || temperature) {
        const prior = this.recoveryGpus.get(gpu.id);
        this.recoveryGpus.set(gpu.id, { load: load || !!prior?.load, temperature: temperature || !!prior?.temperature });
      }
    }
    // A GPU which caused pressure must report healthy data again. Disconnection,
    // an unsupported field or a stale cache cannot stand in for recovery evidence.
    const gpuRecovery = [...this.recoveryGpus].every(([id, required]) => {
      const gpu = gpus.find(candidate => candidate.id === id), load = gpuLoad(gpu, now);
      return !!gpu && (!required.load || (load !== null && load < limit.gpuPercent - 5)) &&
        (!required.temperature || (Number.isFinite(gpu.temperatureCelsius) && gpu.temperatureCelsius! < limit.temperatureCelsius - 5));
    });
    const healthy = cpu !== null && ram !== null && cpu < limit.cpuPercent - 5 && ram < limit.memoryPercent - 5 && [...temps, ...gpuTemps].every(t => t.celsius < limit.temperatureCelsius - 5) && gpus.every(gpu => (gpuLoad(gpu, now) ?? 0) < limit.gpuPercent - 5) && gpuRecovery;
    if (this.mode !== 'normal' && healthy) {
      this.healthySince ??= now;
      if (now - this.healthySince >= 20) {
        this.mode = this.mode === 'critical' ? 'pressure' : 'normal';
        this.healthySince = this.mode === 'normal' ? null : now;
        if (this.mode === 'normal') this.recoveryGpus.clear();
      }
    } else this.healthySince = null;
    this.reasons = [
      ...(cpuPressure ? [`Sustained CPU load ${cpu!.toFixed(0)}%.`] : []),
      ...(ramPressure ? [`Sustained memory use ${ram!.toFixed(0)}%.`] : []),
      ...gpuPressure.map(gpu => `${gpu.name}: sustained GPU load ${gpuLoad(gpu, now)!.toFixed(0)}%.`),
      ...hot.map(t => `${t.label}: ${t.celsius.toFixed(0)} °C reported.`),
    ];
    if (!this.reasons.length) this.reasons = [this.mode === 'normal' ? 'Normal monitoring budget.' : 'Recovering · waiting for 20 seconds of healthy readings per step.'];
    this.suggestions = [
      ...(cpuPressure ? ['Review busy apps in Task Manager or Activity Monitor; pause work you no longer need.'] : []),
      ...(ramPressure ? ['Close unused apps or tabs to free memory.'] : []),
      ...(gpuPressure.length ? ['Reduce graphics-heavy apps or rendering work while the GPU settles.'] : []),
      ...(hot.length ? ['Check ventilation and reduce heavy work while temperatures settle. Throttling is not measured.'] : []),
    ];
    return result();
  }
}
