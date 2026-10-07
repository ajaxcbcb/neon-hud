import { afterEach, describe, expect, it, vi } from 'vitest';
import { createSettingsWriter } from './preferences';

afterEach(() => vi.useRealTimers());
describe('profile persistence', () => {
  it('flushes the latest edit before exit instead of losing the debounce', async () => {
    vi.useFakeTimers();
    const save = vi.fn().mockResolvedValue(undefined);
    const writer = createSettingsWriter(save, vi.fn());
    let theme = 'circuit';
    writer.schedule(() => ({ theme }));
    theme = 'aurora';
    await writer.flush(() => ({ theme }));
    await vi.runAllTimersAsync();
    expect(save).toHaveBeenCalledExactlyOnceWith({ theme: 'aurora' });
    writer.destroy();
  });
  it('keeps writes ordered so a slow previous save cannot reset the profile', async () => {
    let finish!: () => void;
    const save = vi.fn().mockImplementationOnce(() => new Promise<void>(resolve => { finish = resolve; })).mockResolvedValue(undefined);
    const writer = createSettingsWriter(save, vi.fn());
    const first = writer.flush(() => 'old');
    await vi.waitFor(() => expect(finish).toBeTypeOf('function'));
    const second = writer.flush(() => 'new');
    expect(save).toHaveBeenCalledTimes(1);
    finish();
    await Promise.all([first, second]);
    expect(save.mock.calls.map(call => call[0])).toEqual(['old', 'new']);
    writer.destroy();
  });
  it('reports failure and can save again after write access is restored', async () => {
    const publish = vi.fn();
    const save = vi.fn().mockRejectedValueOnce(new Error('denied')).mockResolvedValue(undefined);
    const writer = createSettingsWriter(save, publish);
    await expect(writer.flush(() => 'first')).rejects.toThrow('denied');
    expect(publish).toHaveBeenLastCalledWith(false, true);
    await writer.flush(() => 'retry');
    expect(publish).toHaveBeenLastCalledWith(false, false);
    writer.destroy();
  });
});
