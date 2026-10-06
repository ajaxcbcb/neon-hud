import type { Settings, SystemSnapshot } from './model';
import { drivePercent } from './model';

export interface PressureSignals {
  cpu: string[];
  memory: string[];
  temperature: string[];
  drives: Record<string, string>;
}
const empty = (): PressureSignals => ({ cpu: [], memory: [], temperature: [], drives: {} });

// Require 10 seconds of consecutive samples for workload pressure. A gap or stale
// source clears sustained conditions; a temperature/capacity threshold is immediate.
export class PressureTracker {
  private conditions = new Map<string, { start: number; last: number }>();
  private sampledAt = -1;
  observe(snapshot: SystemSnapshot | null, settings: Settings) {
    if (!snapshot) { this.conditions.clear(); this.sampledAt = -1; return; }
    if (snapshot.sampledAt === this.sampledAt) return;
    const time = snapshot.sampledAt;
    const memory = snapshot.memory.total ? snapshot.memory.used / snapshot.memory.total * 100 : 0;
    for (const [key, active] of [['cpu', snapshot.cpu !== null && snapshot.cpu >= settings.performance.cpuPercent], ['memory', memory >= settings.performance.memoryPercent]] as const) {
      const prior = this.conditions.get(key);
      if (!active) this.conditions.delete(key);
      else this.conditions.set(key, { start: prior && time > prior.last && time - prior.last <= 12 ? prior.start : time, last: time });
    }
    this.sampledAt = time;
  }
  reading(snapshot: SystemSnapshot | null, settings: Settings, now: number): PressureSignals {
    const result = empty();
    if (!snapshot || now - snapshot.sampledAt > 12 || snapshot.sampledAt > now + 5) return result;
    const sustained = (key: string) => { const period = this.conditions.get(key); return !!period && period.last - period.start >= 10; };
    if (sustained('cpu') && snapshot.cpu !== null && snapshot.cpu >= settings.performance.cpuPercent) result.cpu.push(`Sustained CPU load ${snapshot.cpu.toFixed(1)}% ≥ ${settings.performance.cpuPercent}% · may slow other work`);
    const memory = snapshot.memory.total ? snapshot.memory.used / snapshot.memory.total * 100 : 0;
    if (sustained('memory') && memory >= settings.performance.memoryPercent) result.memory.push(`Sustained memory use ${memory.toFixed(1)}% ≥ ${settings.performance.memoryPercent}% · pressure may slow work`);
    for (const sensor of snapshot.temperatures) {
      if (!Number.isFinite(sensor.celsius) || sensor.celsius < settings.performance.temperatureCelsius) continue;
      const message = `${sensor.label}: ${sensor.celsius.toFixed(1)} °C ≥ ${settings.performance.temperatureCelsius} °C · high temperature; throttling is not measured`;
      result.temperature.push(message);
      if (/cpu|core|package|tctl|tdie|peci|processor/i.test(sensor.label)) result.cpu.push(message);
    }
    for (const drive of snapshot.drives || []) {
      const value = drivePercent(drive);
      if (value !== null && value >= settings.performance.storagePercent) result.drives[drive.id] = `${drive.name || drive.mount}: ${value.toFixed(1)}% used ≥ ${settings.performance.storagePercent}% · low free space`;
    }
    return result;
  }
}
