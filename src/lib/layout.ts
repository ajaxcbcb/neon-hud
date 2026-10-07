import type { Settings } from './model';
export const PILL_HEIGHT = 56;
export const COMPRESSED_WIDTH = 160;
export function isPill(settings: Settings) { return settings.size !== 'expanded'; }
export function pillWidth(settings: Settings) { return settings.size === 'compressed' ? COMPRESSED_WIDTH : 280; }
export function windowSize(settings: Settings, configuration: boolean, detail: boolean, pillHeight = PILL_HEIGHT) {
  if (configuration) return { width: 760, height: 680, minWidth: 430, minHeight: 540 };
  const scale = settings.textScale;
  if (detail) return { width: 680 * scale, height: 440 * scale, minWidth: 300, minHeight: 200 };
  if (settings.size === 'expanded') return { width: 680 * scale, height: 500 * scale, minWidth: 300, minHeight: 200 };
  const width = pillHeight > PILL_HEIGHT ? 280 : pillWidth(settings);
  return { width: width * scale, height: Math.max(PILL_HEIGHT, pillHeight) * scale, minWidth: pillWidth(settings) * scale, minHeight: PILL_HEIGHT * scale };
}
export interface DisplayArea { name: string | null; scaleFactor: number; area: { x: number; y: number; width: number; height: number }; }
const clamp = (value: number, origin: number, length: number, occupied: number) => Math.round(Math.max(origin, Math.min(value, origin + Math.max(0, length - occupied))));
export interface PopoverDirection { left: boolean; up: boolean; }
export function displayIndex(settings: Settings, displays: DisplayArea[]) {
  const name = settings.windowPosition?.monitor;
  if (name && displays[settings.monitor]?.name === name) return settings.monitor;
  const named = name ? displays.findIndex(m => m.name === name) : -1;
  return named >= 0 ? named : displays[settings.monitor] ? settings.monitor : 0;
}
export function popoverDirection(settings: Settings, displays: DisplayArea[]): PopoverDirection {
  const display = displays[displayIndex(settings, displays)];
  const saved = settings.windowPosition;
  if (!display || !saved) return { left: settings.corner.endsWith('right'), up: settings.corner.startsWith('bottom') };
  return { left: saved.x > (display.area.width / display.scaleFactor - pillWidth(settings) * settings.textScale) / 2,
    up: saved.y > (display.area.height / display.scaleFactor - PILL_HEIGHT * settings.textScale) / 2 };
}
function popoverOffset(settings: Settings, scale: number, size: { width: number; height: number }, popover: boolean, direction: PopoverDirection) {
  return { x: popover && settings.size === 'compressed' && direction.left ? size.width - COMPRESSED_WIDTH * settings.textScale * scale : 0,
    y: popover && direction.up ? size.height - PILL_HEIGHT * settings.textScale * scale : 0 };
}
export function floatingPosition(settings: Settings, displays: DisplayArea[], size: { width: number; height: number }, popover = false) {
  const saved = settings.windowPosition;
  const display = displays[displayIndex(settings, displays)];
  if (!display) return null;
  if (!saved) return dockPosition(settings.corner, display.area, size, display.scaleFactor, isPill(settings) && popover, settings.textScale);
  const offset = popoverOffset(settings, display.scaleFactor, size, popover, popoverDirection(settings, displays));
  return { x: clamp(display.area.x + saved.x * display.scaleFactor - offset.x, display.area.x, display.area.width, size.width),
    y: clamp(display.area.y + saved.y * display.scaleFactor - offset.y, display.area.y, display.area.height, size.height) };
}
export function displayIndexAt(point: { x: number; y: number }, displays: DisplayArea[], size: { width: number; height: number }) {
  // Choose the display with the largest intersection, including negative origins.
  const areaOf = (m: DisplayArea) => Math.max(0, Math.min(point.x + size.width, m.area.x + m.area.width) - Math.max(point.x, m.area.x))
    * Math.max(0, Math.min(point.y + size.height, m.area.y + m.area.height) - Math.max(point.y, m.area.y));
  return displays.reduce((best, display, index) => best < 0 || areaOf(display) > areaOf(displays[best]) ? index : best, -1);
}
export function rememberPosition(point: { x: number; y: number }, displays: DisplayArea[], settings: Settings, size: { width: number; height: number }, popover = false, direction = popoverDirection(settings, displays)): Settings['windowPosition'] {
  const display = displays[displayIndexAt(point, displays, size)];
  if (!display) return null;
  const offset = popoverOffset(settings, display.scaleFactor, size, popover, direction);
  return { x: (point.x + offset.x - display.area.x) / display.scaleFactor, y: (point.y + offset.y - display.area.y) / display.scaleFactor, monitor: display.name };
}
export function dockPosition(corner: Settings['corner'], area: { x: number; y: number; width: number; height: number }, size: { width: number; height: number }, scale: number, pill: boolean, textScale = 1) {
  const padding = Math.round(20 * scale);
  const x = corner.endsWith('right') ? area.x + area.width - size.width - padding : area.x + padding;
  // Middle docks keep the capsule stationary when a popover grows below it.
  const anchorHeight = pill ? PILL_HEIGHT * textScale * scale : size.height;
  const y = corner.startsWith('bottom') ? area.y + area.height - size.height - padding
    : corner.startsWith('middle') ? area.y + (area.height - anchorHeight) / 2 : area.y + padding;
  return { x: clamp(x, area.x, area.width, size.width), y: clamp(y, area.y, area.height, size.height) };
}
