/** Background calls share a read; user actions wait, then request a fresh read. */
export function createRefreshQueue(read: (force: boolean) => Promise<void>) {
  let flight: Promise<void> | null = null;
  return async (force = false): Promise<void> => {
    if (flight && !force) return flight;
    while (flight) {
      try { await flight; } catch { /* A previous read cannot fail the next action. */ }
    }
    const current = read(force);
    flight = current;
    try { await current; } finally { if (flight === current) flight = null; }
  };
}
