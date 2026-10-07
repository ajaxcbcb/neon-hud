export interface PollClock {
  now: () => number;
  schedule: (callback: () => void, delay: number) => ReturnType<typeof setTimeout>;
  cancel: (timer: ReturnType<typeof setTimeout>) => void;
}
const clock: PollClock = { now: () => performance.now(), schedule: (callback, delay) => setTimeout(callback, delay), cancel: timer => clearTimeout(timer) };
// Each source owns a timer. Slow quota reads never hold up cheap system reads.
export function createPoller(read: () => Promise<void>, period: () => number, runtime = clock) {
  let timer: ReturnType<typeof setTimeout> | null = null;
  let enabled = false, stopped = false, busy = false;
  let lastStart = -Infinity;
  function cancel() { if (timer !== null) runtime.cancel(timer); timer = null; }
  function schedule() {
    cancel();
    if (!enabled || stopped || busy) return;
    const delay = Math.max(0, Math.max(250, period()) - (runtime.now() - lastStart));
    timer = runtime.schedule(() => { timer = null; void poll(); }, delay);
  }
  async function poll() {
    if (!enabled || stopped || busy) return;
    busy = true; lastStart = runtime.now();
    try { await read(); } catch { /* The source reports its own error; retry on its next cadence. */ }
    finally { busy = false; schedule(); }
  }
  return {
    setEnabled(value: boolean) { if (stopped || value === enabled) return; enabled = value; cancel(); if (value) { lastStart = -Infinity; schedule(); } },
    reschedule: schedule,
    destroy() { stopped = true; enabled = false; cancel(); },
  };
}
