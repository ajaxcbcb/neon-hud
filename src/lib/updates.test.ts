import { describe, expect, it, vi } from 'vitest';
import { UpdateController, automaticInstallReady, systemAllowsInstall, type AutoInstallContext, type PackageUpdate, type UpdateStatus } from './updates';
import { defaults, sampleSystem } from './model';
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
  it('installs a verified package automatically once, with Windows restart enabled', async () => {
    const f = fixture(); await f.controller.check(true);
    expect(await f.controller.install(true)).toBe(true);
    expect(f.update.install).toHaveBeenCalledWith({ restartAfterInstall: true });
    expect(await f.controller.install(true)).toBe(false);
    expect(f.update.install).toHaveBeenCalledOnce();
  });
  it('does not repeatedly attempt a failed automatic installation; permits a manual retry', async () => {
    const f = fixture(); await f.controller.check(true);
    vi.mocked(f.update.install).mockRejectedValueOnce(new Error('private installer error'));
    expect(await f.controller.install(true)).toBe(false);
    expect(f.states.at(-1)?.message).not.toContain('private');
    await f.controller.install(true); await f.controller.install(true);
    expect(f.update.install).toHaveBeenCalledOnce();
    expect(await f.controller.install()).toBe(true);
    expect(f.update.install).toHaveBeenCalledTimes(2);
  });
  it('honors automatic updates being disabled while the release check is in flight', async () => {
    const f = fixture(); let enabled = true, finish!: (value: PackageUpdate) => void;
    f.check.mockImplementationOnce(() => new Promise(resolve => finish = resolve));
    const checking = f.controller.check(() => enabled);
    enabled = false; await vi.waitFor(() => expect(finish).toBeTypeOf('function')); finish(f.update); await checking;
    expect(f.update.download).not.toHaveBeenCalled();
    expect(f.update.install).not.toHaveBeenCalled();
  });
  it('never automatically installs a signature failure or a disposed package', async () => {
    const f = fixture(); vi.mocked(f.update.download).mockRejectedValueOnce(Error('signature'));
    await f.controller.check(true); expect(await f.controller.install(true)).toBe(false);
    await f.controller.download(); await f.controller.dispose();
    expect(await f.controller.install(true)).toBe(false);
    expect(f.update.install).not.toHaveBeenCalled();
  });
});

describe('automatic installation quiet window', () => {
  const quiet: AutoInstallContext = { enabled: true, autoUpdates: true, idleMs: 60000, readyMs: 60000,
    configuring: false, paused: false, connecting: false, saving: false, attention: false, pressure: false };
  it('requires opt-in, both update switches and a full quiet minute after verification', () => {
    expect(automaticInstallReady(quiet)).toBe(true);
    for (const changed of [{ enabled: false }, { autoUpdates: false }, { idleMs: 59999 }, { readyMs: 59999 }]) {
      expect(automaticInstallReady({ ...quiet, ...changed })).toBe(false);
    }
  });
  it('re-evaluates interaction, settings, pending questions and pressure before installation', () => {
    for (const key of ['configuring', 'paused', 'connecting', 'saving', 'attention', 'pressure'] as const) {
      expect(automaticInstallReady({ ...quiet, [key]: true })).toBe(false);
    }
    expect(automaticInstallReady({ ...quiet, idleMs: 0 })).toBe(false);
  });
  it('requires fresh healthy real readings even with adaptive monitoring disabled', () => {
    const healthy = { ...sampleSystem, sampledAt: 100, gpus: [] };
    const settings = { ...defaults, resources: { ...defaults.resources, adaptive: false } };
    expect(systemAllowsInstall(healthy, settings, 100)).toBe(true);
    for (const bad of [null, { ...healthy, sampledAt: 87 }, { ...healthy, sampledAt: 106 }, { ...healthy, sampledAt: NaN },
      { ...healthy, cpu: null }, { ...healthy, cpu: 95 }, { ...healthy, memory: { ...healthy.memory, used: healthy.memory.total } },
      { ...healthy, temperatures: [{ label: 'CPU', celsius: 90 }] }]) expect(systemAllowsInstall(bad, settings, 100)).toBe(false);
  });
  it('defers for a fresh overloaded or hot GPU while allowing unsupported GPU telemetry', () => {
    const gpu = { ...sampleSystem.gpus![0], sampledAt: 100 };
    const healthy = { ...sampleSystem, sampledAt: 100, temperatures: [] };
    expect(systemAllowsInstall({ ...healthy, gpus: [{ ...gpu, utilizationPercent: null }] }, defaults, 100)).toBe(true);
    for (const stressed of [{ ...gpu, utilizationPercent: 95 }, { ...gpu, temperatureCelsius: 90 }]) {
      expect(systemAllowsInstall({ ...healthy, gpus: [stressed] }, defaults, 100)).toBe(false);
    }
  });
});
