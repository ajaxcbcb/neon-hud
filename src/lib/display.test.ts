import { describe, expect, it, vi } from 'vitest';
import { DisplayTween, type FrameClock } from './display';

class FakeFrames implements FrameClock {
  time = 0;
  next = 1;
  callbacks = new Map<number, FrameRequestCallback>();
  request = (callback: FrameRequestCallback) => {
    const id = this.next++;
    this.callbacks.set(id, callback);
    return id;
  };
  cancel = (id: number) => { this.callbacks.delete(id); };
  now = () => this.time;
  tick(ms: number) {
    this.time += ms;
    const pending = [...this.callbacks.values()];
    this.callbacks.clear();
    pending.forEach(callback => callback(this.time));
  }
}

describe('DisplayTween', () => {
  it('paints on every supplied 144 Hz frame, settles by elapsed time, and idles without frames', () => {
    const clock = new FakeFrames();
    const paint = vi.fn();
    const tween = new DisplayTween(paint, clock);
    tween.set(0);
    tween.set(100);
    for (let frame = 0; frame < 26; frame++) clock.tick(1000 / 144);
    expect(paint.mock.calls.length).toBeGreaterThan(20);
    expect(paint.mock.lastCall?.[0]).toBeCloseTo(100, 4);
    expect(clock.callbacks.size).toBe(0);
    const count = paint.mock.calls.length;
    clock.tick(1000);
    expect(paint).toHaveBeenCalledTimes(count);
  });

  it('cancels the previous target and stops all work after destroy', () => {
    const clock = new FakeFrames();
    const paint = vi.fn();
    const tween = new DisplayTween(paint, clock);
    tween.set(0);
    tween.set(100);
    clock.tick(60);
    const interrupted = paint.mock.lastCall?.[0] as number;
    tween.set(20);
    expect(clock.callbacks.size).toBe(1);
    clock.tick(180);
    expect(paint.mock.lastCall?.[0]).toBeCloseTo(20, 4);
    expect(interrupted).toBeGreaterThan(20);
    tween.set(75);
    tween.destroy();
    const count = paint.mock.calls.length;
    expect(clock.callbacks.size).toBe(0);
    clock.tick(500);
    tween.set(5);
    expect(paint).toHaveBeenCalledTimes(count);
  });

  it('applies reduced-motion and missing readings immediately', () => {
    const clock = new FakeFrames();
    const paint = vi.fn();
    const tween = new DisplayTween(paint, clock);
    tween.set(35);
    tween.set(80);
    expect(clock.callbacks.size).toBe(1);
    tween.set(55, false);
    expect(clock.callbacks.size).toBe(0);
    expect(paint.mock.lastCall?.[0]).toBe(55);
    tween.set(null);
    expect(clock.callbacks.size).toBe(0);
    expect(paint.mock.lastCall?.[0]).toBe(0);
  });
});
