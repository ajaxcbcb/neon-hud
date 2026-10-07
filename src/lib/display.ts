export interface FrameClock {
  request: (callback: FrameRequestCallback) => number;
  cancel: (handle: number) => void;
  now: () => number;
}
const frames: FrameClock = { request: callback => requestAnimationFrame(callback), cancel: handle => cancelAnimationFrame(handle), now: () => performance.now() };

// Interpolate geometry only. Numbers continue to show the actual latest sample.
// The display chooses the cadence; idle meters have no animation-frame loop.
export class DisplayTween {
  private frame: number | null = null;
  private current: number | null = null;
  private target = 0;
  private disposed = false;
  constructor(private paint: (value: number) => void, private clock = frames) {}
  set(value: number | null, animate = true) {
    if (this.disposed) return;
    this.target = Number.isFinite(value) ? Math.max(0, Math.min(100, value!)) : 0;
    if (this.target === this.current && this.frame === null) return;
    this.cancel();
    const start = this.current;
    if (start === null || !animate || value === null) { this.current = this.target; this.paint(this.target); return; }
    const begun = this.clock.now();
    const target = this.target;
    const step = (time: number) => {
      this.frame = null;
      if (this.disposed) return;
      const progress = Math.max(0, Math.min(1, (time - begun) / 180));
      this.current = start + (target - start) * (1 - (1 - progress) ** 3);
      this.paint(this.current);
      if (progress < 1) this.frame = this.clock.request(step);
    };
    this.frame = this.clock.request(step);
  }
  settle() { this.set(this.target, false); }
  private cancel() { if (this.frame !== null) this.clock.cancel(this.frame); this.frame = null; }
  destroy() { this.disposed = true; this.cancel(); }
}
export interface MeterDisplay { value: number | null; kind?: 'width' | 'arc'; reduced?: boolean; }
export function displayMeter(node: HTMLElement | SVGElement, initial: MeterDisplay) {
  let options = initial;
  let nativeVisible = true;
  const media = matchMedia('(prefers-reduced-motion: reduce)');
  const tween = new DisplayTween(value => {
    if (options.kind === 'arc') node.setAttribute('stroke-dasharray', `${value} 100`);
    else (node as HTMLElement).style.width = `${value}%`;
  });
  const update = (value: MeterDisplay) => { options = value; tween.set(value.value, nativeVisible && !document.hidden && !value.reduced && !media.matches); };
  const visibility = () => { if (document.hidden) tween.settle(); };
  const nativeVisibility = (event: Event) => { nativeVisible = (event as CustomEvent<boolean>).detail; if (!nativeVisible) tween.settle(); };
  const reduce = () => { if (media.matches) tween.settle(); };
  document.addEventListener('visibilitychange', visibility);
  window.addEventListener('neon-visibility', nativeVisibility);
  media.addEventListener('change', reduce);
  update(initial);
  return { update, destroy() {
    tween.destroy();
    document.removeEventListener('visibilitychange', visibility);
    window.removeEventListener('neon-visibility', nativeVisibility);
    media.removeEventListener('change', reduce);
  } };
}
