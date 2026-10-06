import { remaining, windowStatus } from './model';
import type { Usage, UsageWindow } from './model';

export interface DrainSample { at: number; used: number; reset: number | null }
export interface DrainReading {
  percentPerHour: number;
  secondsLeft: number | null;
  fast: boolean;
  observedSeconds: number;
}
// Keep a bounded 30-minute observation per provider/window. These are allowance
// readings, never token counts: providers do not expose a fixed token budget.
export class DrainTracker {
  private samples = new Map<string, DrainSample[]>();
  private tokens = new Map<string, { session: string; points: {at: number; total: number}[] }>();
  observe(usages: Usage[], now: number) {
    for (const usage of usages) {
      const token = usage.tokenUsage;
      if (!token || !Number.isFinite(token.total) || token.total < 0 || !Number.isFinite(token.sampledAt) || now - token.sampledAt > 600) continue;
      let entry = this.tokens.get(usage.surface);
      const previous = entry?.points.at(-1);
      if (!entry || entry.session !== token.sessionId || (previous && (token.total < previous.total || token.sampledAt < previous.at))) entry = { session: token.sessionId, points: [] };
      if (entry.points.at(-1)?.at !== token.sampledAt) entry.points.push({at: token.sampledAt, total: token.total});
      while (entry.points.length > 2 && entry.points[1].at < token.sampledAt - 1800) entry.points.shift();
      entry.points = entry.points.slice(-361);
      this.tokens.set(usage.surface, entry);
    }
    for (const usage of usages) for (const window of usage.windows) {
      if (usage.state !== 'connected' || usage.fetchedAt === null || windowStatus(window, usage.fetchedAt, now) !== 'Current' || !Number.isFinite(window.usedPercent)) continue;
      const key = this.key(usage, window);
      let points = this.samples.get(key) || [];
      const last = points.at(-1);
      if (last && (last.reset !== window.resetsAt || window.usedPercent < last.used || usage.fetchedAt < last.at)) points = [];
      if (points.at(-1)?.at === usage.fetchedAt) continue;
      points.push({ at: usage.fetchedAt, used: window.usedPercent, reset: window.resetsAt });
      const cutoff = usage.fetchedAt - 1800;
      // Retain the point bracketing the cutoff for a time-weighted average.
      while (points.length > 2 && points[1].at < cutoff) points.shift();
      this.samples.set(key, points.slice(-361));
    }
  }
  reading(usage: Usage, window: UsageWindow, now: number): DrainReading | null {
    if (usage.state !== 'connected' || windowStatus(window, usage.fetchedAt, now) !== 'Current') return null;
    const points = this.samples.get(this.key(usage, window));
    if (!points || points.length < 2) return null;
    const first = points[0], last = points[points.length - 1];
    if (last.reset !== window.resetsAt || last.used !== window.usedPercent) return null;
    const elapsed = last.at - first.at;
    if (elapsed < 120) return null;
    const percentPerHour = (last.used - first.used) / elapsed * 3600;
    const secondsLeft = percentPerHour > 0 ? remaining(last.used) / percentPerHour * 3600 : null;
    // Fast means the measured rate projects exhaustion before the reported reset.
    const fast = secondsLeft !== null && window.resetsAt !== null && secondsLeft < window.resetsAt - now;
    return { percentPerHour, secondsLeft, fast, observedSeconds: elapsed };
  }
  private key(usage: Usage, window: UsageWindow) { return `${usage.surface}:${window.minutes}:${window.label}`; }
  tokenReading(usage: Usage, now: number) {
    const token = usage.tokenUsage, entry = this.tokens.get(usage.surface);
    if (!token || !entry || entry.session !== token.sessionId || now - token.sampledAt > 600 || entry.points.length < 2) return null;
    const first = entry.points[0], last = entry.points.at(-1)!;
    const elapsed = last.at - first.at;
    if (elapsed < 120) return null;
    const average = (last.total - first.total) / elapsed * 60;
    const recent = entry.points.find(p => p.at >= last.at - 300) || first;
    const recentElapsed = last.at - recent.at;
    const recentRate = recentElapsed >= 60 ? (last.total - recent.total) / recentElapsed * 60 : average;
    return { perMinute: average, recentPerMinute: recentRate, total: last.total, observedSeconds: elapsed, fast: elapsed >= 600 && recentRate > average * 2 && recentRate > 0 };
  }
}
export function drainText(reading: DrainReading | null) {
  if (!reading) return 'Drain: collecting ≥2 min of readings';
  const rate = `${reading.percentPerHour.toFixed(1)}%/h`;
  if (reading.secondsLeft === null) return `${rate} · no drain observed`;
  const minutes = Math.max(1, Math.ceil(reading.secondsLeft / 60));
  const eta = minutes < 60 ? `${minutes}m` : `${Math.floor(minutes / 60)}h ${minutes % 60}m`;
  return `${rate} · ~${eta} left at this pace`;
}
