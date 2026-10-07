/** Debounce normal edits, serialize writes, and flush the latest profile before exit. */
export function createSettingsWriter<T>(save: (value: T) => Promise<void>, publish: (pending: boolean, failed: boolean) => void, delay = 350) {
  let timer: ReturnType<typeof setTimeout> | undefined;
  let queue = Promise.resolve();
  let revision = 0;
  let failed = false;
  function schedule(read: () => T) {
    clearTimeout(timer);
    revision++;
    publish(true, failed);
    timer = setTimeout(() => { void flush(read).catch(() => {}); }, delay);
  }
  async function flush(read: () => T) {
    clearTimeout(timer);
    const current = ++revision;
    const value = read();
    publish(true, failed);
    const writing = queue.catch(() => {}).then(() => save(value));
    queue = writing;
    try {
      await writing;
      if (current === revision) { failed = false; publish(false, false); }
    } catch (error) {
      if (current === revision) { failed = true; publish(false, true); }
      throw error;
    }
  }
  return { schedule, flush, destroy: () => { clearTimeout(timer); revision++; } };
}
