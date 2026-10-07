import { describe, expect, it, vi } from 'vitest';
import { createRefreshQueue } from './refresh';

describe('provider refresh queue', () => {
  it('reads fresh data for an action even when the previous forced read failed', async () => {
    let reject!: (error: Error) => void;
    const read = vi.fn().mockImplementationOnce(() => new Promise<void>((_, fail) => reject = fail)).mockResolvedValue(undefined);
    const refresh = createRefreshQueue(read);
    const first = refresh(true);
    const failure = expect(first).rejects.toThrow('offline');
    const action = refresh(true);
    reject(Error('offline'));
    await failure; await action;
    expect(read).toHaveBeenCalledTimes(2);
    expect(read).toHaveBeenLastCalledWith(true);
  });
  it('shares a background read and serializes simultaneous action reads', async () => {
    let resolve!: () => void;
    let active = 0, maximum = 0;
    const read = vi.fn(async () => {
      active++; maximum = Math.max(maximum, active);
      if (read.mock.calls.length === 1) await new Promise<void>(done => resolve = done);
      active--;
    });
    const refresh = createRefreshQueue(read);
    const first = refresh(), background = refresh(), action1 = refresh(true), action2 = refresh(true);
    expect(read).toHaveBeenCalledTimes(1);
    resolve(); await Promise.all([first, background, action1, action2]);
    expect(read).toHaveBeenCalledTimes(3); expect(maximum).toBe(1);
  });
});
