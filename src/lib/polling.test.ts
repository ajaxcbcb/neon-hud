import { describe, expect, it, vi } from 'vitest';
import { createPoller, type PollClock } from './polling';

class FakePollClock implements PollClock {
  time = 0;
  next = 1;
  timers = new Map<ReturnType<typeof setTimeout>, { due: number; callback: () => void }>();
  now = () => this.time;
  schedule = (callback: () => void, delay: number) => {
    const id = this.next++ as unknown as ReturnType<typeof setTimeout>;
    this.timers.set(id, { due: this.time + delay, callback });
    return id;
  };
  cancel = (timer: ReturnType<typeof setTimeout>) => { this.timers.delete(timer); };
  advanceTo(time: number) {
    while (true) {
      const due = [...this.timers].filter(([, timer]) => timer.due <= time).sort((a, b) => a[1].due - b[1].due)[0];
      if (!due) break;
      this.time = due[1].due;
      this.timers.delete(due[0]);
      due[1].callback();
    }
    this.time = time;
  }
}

const flush = async () => { await Promise.resolve(); await Promise.resolve(); };

describe('createPoller', () => {
  it('samples hardware every 250 ms while a slow provider read remains in flight', async () => {
    const clock = new FakePollClock();
    const hardware = vi.fn(async () => {});
    const provider = vi.fn(() => new Promise<void>(() => {}));
    const systemPoller = createPoller(hardware, () => 250, clock);
    const providerPoller = createPoller(provider, () => 5000, clock);
    systemPoller.setEnabled(true);
    providerPoller.setEnabled(true);
    clock.advanceTo(0); await flush();
    for (const time of [250, 500, 750]) { clock.advanceTo(time); await flush(); }
    expect(hardware).toHaveBeenCalledTimes(4);
    expect(provider).toHaveBeenCalledTimes(1);
    systemPoller.destroy(); providerPoller.destroy();
  });

  it('never overlaps reads, even after reschedule or time advancing beyond a period', async () => {
    const clock = new FakePollClock();
    let finish!: () => void;
    let active = 0, maximum = 0;
    const read = vi.fn(() => {
      active++;
      maximum = Math.max(maximum, active);
      return new Promise<void>(resolve => { finish = () => { active--; resolve(); }; });
    });
    const poller = createPoller(read, () => 250, clock);
    poller.setEnabled(true);
    clock.advanceTo(0);
    clock.advanceTo(1000);
    poller.reschedule();
    expect(read).toHaveBeenCalledTimes(1);
    expect(clock.timers.size).toBe(0);
    finish(); await flush();
    clock.advanceTo(1000);
    expect(read).toHaveBeenCalledTimes(2);
    expect(maximum).toBe(1);
    poller.destroy();
    finish(); await flush();
  });

  it('cancels on hidden pause and resumes immediately when enabled', async () => {
    const clock = new FakePollClock();
    const read = vi.fn(async () => {});
    const poller = createPoller(read, () => 250, clock);
    poller.setEnabled(true);
    clock.advanceTo(0); await flush();
    poller.setEnabled(false);
    expect(clock.timers.size).toBe(0);
    clock.advanceTo(1000);
    expect(read).toHaveBeenCalledTimes(1);
    poller.setEnabled(true);
    clock.advanceTo(1000); await flush();
    expect(read).toHaveBeenCalledTimes(2);
    poller.destroy();
  });

  it('cannot schedule a next read when an in-flight read completes after destroy', async () => {
    const clock = new FakePollClock();
    let finish!: () => void;
    const read = vi.fn(() => new Promise<void>(resolve => { finish = resolve; }));
    const poller = createPoller(read, () => 250, clock);
    poller.setEnabled(true);
    clock.advanceTo(0);
    poller.destroy();
    finish(); await flush();
    clock.advanceTo(5000);
    expect(read).toHaveBeenCalledTimes(1);
    expect(clock.timers.size).toBe(0);
  });
});
