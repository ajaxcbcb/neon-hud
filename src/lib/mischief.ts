import { animate, spring } from 'animejs';
type Options = { mode: string; reduced: boolean };

// Interaction-only bursts, bounded to 24 particles, never a persistent render loop.
export function mischief(node: HTMLElement, options: Options) {
  let settings = options;
  const active = new Set<ReturnType<typeof animate>>();
  const particles = new Set<HTMLElement>();
  const preference = matchMedia('(prefers-reduced-motion: reduce)');
  function stop() { active.forEach(a => a.cancel()); active.clear(); particles.forEach(p => p.remove()); particles.clear(); }
  function burst(event: MouseEvent) {
    if (settings.reduced || settings.mode !== 'chaotic' || preference.matches || document.hidden) return;
    const target = (event.target as HTMLElement).closest('button, summary');
    if (!target || (target as HTMLButtonElement).disabled || particles.size >= 24) return;
    const rect = target.getBoundingClientRect();
    const x = event.detail ? event.clientX : rect.x + rect.width / 2;
    const y = event.detail ? event.clientY : rect.y + rect.height / 2;
    for (let index = 0; index < 8; index++) {
      const particle = document.createElement('span');
      particle.className = 'chaos-particle';
      particle.textContent = index % 3 === 0 ? '✦' : index % 3 === 1 ? '+' : '·';
      particle.style.left = `${x}px`; particle.style.top = `${y}px`;
      particle.style.color = index % 2 ? 'var(--accent)' : 'var(--second)';
      node.append(particle); particles.add(particle);
      const angle = index * Math.PI / 4;
      const distance = 28 + index % 3 * 12;
      const animation = animate(particle, {
        x: Math.cos(angle) * distance, y: Math.sin(angle) * distance,
        rotate: index % 2 ? 140 : -100, scale: [1.4, .15], opacity: [1, 0],
        ease: spring({ bounce: .45, duration: 650 }),
        onComplete: () => { particle.remove(); particles.delete(particle); active.delete(animation); },
      });
      active.add(animation);
    }
  }
  const visibility = () => { if (document.hidden) stop(); };
  const changed = () => { if (preference.matches) stop(); };
  node.addEventListener('click', burst);
  document.addEventListener('visibilitychange', visibility);
  preference.addEventListener('change', changed);
  return {
    update(next: Options) { settings = next; if (next.reduced || next.mode !== 'chaotic') stop(); },
    destroy() { stop(); node.removeEventListener('click', burst); document.removeEventListener('visibilitychange', visibility); preference.removeEventListener('change', changed); },
  };
}
