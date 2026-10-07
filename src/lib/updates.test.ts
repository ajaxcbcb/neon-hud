import { describe, expect, it, vi } from 'vitest';
import { UpdateController, type PackageUpdate, type UpdateStatus } from './updates';
function fixture() {
  const states: UpdateStatus[] = [];
  const update: PackageUpdate = { version: '0.2.0', close: vi.fn(async () => {}), install: vi.fn(async () => {}), download: vi.fn(async progress => { progress({ event: 'Started', data: { contentLength: 100 } }); progress({ event: 'Progress', data: { chunkLength: 60 } }); progress({ event: 'Finished' }); }) };
  const check = vi.fn(async () => update);
  return { states, update, check, controller: new UpdateController(check, state => states.push(state)) };
}
describe('signed application update flow', () => {
  it('automatically checks/downloads but waits for an explicit install', async () => {
    const f = fixture(); await f.controller.check(true);
    expect(f.states.at(-1)?.phase).toBe('ready');
    expect(f.states.find(state => state.received === 60)?.total).toBe(100);
    expect(f.update.install).not.toHaveBeenCalled();
    await f.controller.install();
    expect(f.update.install).toHaveBeenCalledWith({ restartAfterInstall: true });
    expect(f.states.at(-1)?.phase).toBe('restart');
  });
  it('cannot install when downloading or signature verification failed; permits retry', async () => {
    const f = fixture(); vi.mocked(f.update.download).mockRejectedValueOnce(new Error('bad signature/private diagnostic'));
    await f.controller.check(true); await f.controller.install();
    expect(f.states.at(-1)?.message).not.toContain('private'); expect(f.update.install).not.toHaveBeenCalled();
    await f.controller.download(); expect(f.states.at(-1)?.phase).toBe('ready');
  });
  it('deduplicates simultaneous checks and closes retained resources', async () => {
    const f = fixture(); let finish!: (value: PackageUpdate) => void;
    f.check.mockImplementationOnce(() => new Promise(resolve => finish = resolve));
    const first = f.controller.check(); await f.controller.check(); expect(f.check).toHaveBeenCalledTimes(1);
    finish(f.update); await first; await f.controller.dispose(); expect(f.update.close).toHaveBeenCalledOnce();
  });
  it(' reports current only after a successful no-update check', async () => {
    const states: UpdateStatus[] = [];
    const c = new UpdateController(async () => null, state => states.push(state)); await c.check(true);
    expect(states.at(-1)?.phase).toBe('current');
    const failure = new UpdateController(async () => { throw Error('offline'); }, state => states.push(state)); await failure.check();
    expect(states.at(-1)?.phase).toBe('error');
  });
  it('does not treat completed transfer as a verified signature', async () => {
    const f = fixture(); let verify!: () => void;
    vi.mocked(f.update.download).mockImplementationOnce(async progress => {
      progress({ event: 'Finished' });
      await new Promise<void>(resolve => verify = resolve);
    });
    await f.controller.check(); const download = f.controller.download();
    expect(f.states.at(-1)?.phase).toBe('downloading');
    await f.controller.install(); expect(f.update.install).not.toHaveBeenCalled();
    verify(); await download; expect(f.controller.canInstall).toBe(true);
  });
  it('releases a package after an in-flight download finishes during disposal', async () => {
    const f = fixture(); let finish!: () => void;
    vi.mocked(f.update.download).mockImplementationOnce(() => new Promise<void>(resolve => finish = resolve));
    await f.controller.check(); const download = f.controller.download();
    await f.controller.dispose(); expect(f.update.close).not.toHaveBeenCalled();
    finish(); await download;
    expect(f.update.close).toHaveBeenCalledOnce();
    expect(f.states.some(state => state.phase === 'ready')).toBe(false);
    expect(f.controller.canInstall).toBe(false);
  });
});
