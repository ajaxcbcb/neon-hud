import { describe, it, expect } from 'vitest';
import { DrainTracker } from './drain';
import type { Usage } from './model';
const usage = (at: number, used: number, reset: number | null = 20000): Usage => ({ surface: 'codex', source: 'test', state: 'connected', message: '', fetchedAt: at, windows: [{ label: '5 hours', minutes: 300, usedPercent: used, resetsAt: reset }] });
describe('allowance drain', () => {
  it('measures actual token counts without interpreting percentage as tokens', () => {
    const tracker = new DrainTracker();
    const first = {...usage(1000,20), tokenUsage:{sessionId:'a',total:100,sampledAt:1000}};
    const last = {...usage(1300,25), tokenUsage:{sessionId:'a',total:1600,sampledAt:1300}};
    tracker.observe([first],1000); tracker.observe([last],1300);
    expect(tracker.tokenReading(last,1300)?.perMinute).toBe(300);
    const changed = {...last,tokenUsage:{sessionId:'b',total:20,sampledAt:1400}};
    tracker.observe([changed],1400);
    expect(tracker.tokenReading(changed,1400)).toBeNull();
  });
  it('uses elapsed time rather than polling count and projects exhaustion', () => {
    const tracker = new DrainTracker();
    for (const value of [usage(1000, 20), usage(1060, 20), usage(1300, 25)]) tracker.observe([value], value.fetchedAt!);
    const value = usage(1300, 25);
    expect(tracker.reading(value, value.windows[0], 1300)).toEqual({ percentPerHour: 60, secondsLeft: 4500, fast: true, observedSeconds: 300 });
  });
  it('does not manufacture rates from repeated cached snapshots', () => {
    const tracker = new DrainTracker(), value = usage(1000, 20);
    tracker.observe([value], 1000); tracker.observe([value], 1300);
    expect(tracker.reading(value, value.windows[0], 1300)).toBeNull();
  });
  it('clears the observation after a reset or decreasing usage', () => {
    const tracker = new DrainTracker();
    tracker.observe([usage(1000, 80)], 1000); tracker.observe([usage(1300, 85)], 1300);
    const value = usage(1400, 2, 30000); tracker.observe([value], 1400);
    expect(tracker.reading(value, value.windows[0], 1400)).toBeNull();
  });
  it('suppresses stale estimates and treats no increase as no observed drain', () => {
    const tracker = new DrainTracker(); tracker.observe([usage(1000, 20)], 1000);
    const value = usage(1300, 20); tracker.observe([value], 1300);
    expect(tracker.reading(value, value.windows[0], 1300)?.secondsLeft).toBeNull();
    expect(tracker.reading(value, value.windows[0], 2000)).toBeNull();
  });
});
