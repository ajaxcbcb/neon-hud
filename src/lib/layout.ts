import type { Settings } from './model';
export const PILL_HEIGHT = 56;
export function windowSize(settings: Settings, configuration: boolean, detail: boolean, pillHeight = PILL_HEIGHT) {
  if (configuration) return { width: 760, height: 680, minWidth: 430, minHeight: 540 };
  const scale = settings.textScale;
  if (detail) return { width: 680 * scale, height: 440 * scale, minWidth: 300, minHeight: 200 };
  if (settings.size === 'expanded') return { width: 680 * scale, height: 500 * scale, minWidth: 300, minHeight: 200 };
  return { width: 280 * scale, height: Math.max(PILL_HEIGHT, pillHeight) * scale, minWidth: 250 * scale, minHeight: PILL_HEIGHT * scale };
}
export function dockPosition(corner: Settings['corner'], area: { x: number; y: number; width: number; height: number }, size: { width: number; height: number }, scale: number, pill: boolean, textScale = 1) {
  const padding = Math.round(20 * scale);
  const x = corner.endsWith('right') ? area.x + area.width - size.width - padding : area.x + padding;
  // Middle docks keep the capsule stationary when a popover grows below it.
  const anchorHeight = pill ? PILL_HEIGHT * textScale * scale : size.height;
  const y = corner.startsWith('bottom') ? area.y + area.height - size.height - padding
    : corner.startsWith('middle') ? area.y + (area.height - anchorHeight) / 2 : area.y + padding;
  const clamp = (value: number, origin: number, length: number, occupied: number) => Math.round(Math.max(origin, Math.min(value, origin + Math.max(0, length - occupied))));
  return { x: clamp(x, area.x, area.width, size.width), y: clamp(y, area.y, area.height, size.height) };
}
