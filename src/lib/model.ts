export type Theme = 'circuit' | 'cyberpunk' | 'aurora';
export type Surface = 'chatgpt' | 'codex' | 'claude' | 'claude-code';
export interface Settings {
  version: 1;
  completed: boolean;
  step: number;
  theme: Theme;
  size: 'compact' | 'expanded';
  corner: 'top-left' | 'top-right' | 'bottom-left' | 'bottom-right';
  monitor: number;
  alwaysOnTop: boolean;
  opacity: number;
  textScale: number;
  reducedMotion: boolean;
  motion: 'chaotic' | 'playful' | 'quiet';
  launchAtLogin: boolean;
  notifications: boolean;
  interface: string;
  warning: number;
  critical: number;
  metrics: { cpu: boolean; ram: boolean; network: boolean; ai: boolean };
  codexEnabled: boolean;
}
export interface SystemSnapshot {
  sampledAt: number;
  cpu: number | null;
  cores: { usage: number; frequencyMhz: number }[];
  memory: { used: number; total: number; available: number };
  networks: { name: string; down: number; up: number; received: number; transmitted: number }[];
  defaultInterface: string | null;
  temperatures: { label: string; celsius: number }[];
}
export interface UsageWindow {
  label: string;
  minutes: number;
  usedPercent: number;
  resetsAt: number | null;
}
export interface Usage {
  surface: Surface;
  source: string;
  state: 'connected' | 'unavailable' | 'needs-login' | 'error';
  message: string;
  fetchedAt: number | null;
  windows: UsageWindow[];
  tokenUsage?: { sessionId: string; total: number; sampledAt: number };
}
export interface Attention {
  id: string;
  surface: Surface;
  reason: string;
  occurredAt: number;
  sessionId: string;
}
export interface ProviderSnapshot { usages: Usage[]; attention: Attention[]; }
export const defaults: Settings = {
  version: 1, completed: false, step: 1, theme: 'circuit', size: 'compact',
  corner: 'bottom-right', monitor: 0, alwaysOnTop: true, opacity: 1, textScale: 1,
  reducedMotion: false, motion: 'chaotic', launchAtLogin: false, notifications: false, interface: 'auto',
  warning: 20, critical: 10, metrics: { cpu: true, ram: true, network: true, ai: true },
  codexEnabled: false,
};
export function normalizeSettings(value: unknown): Settings {
  const input = (value && typeof value === 'object' ? value : {}) as Partial<Settings>;
  const result = { ...defaults, ...input, metrics: { ...defaults.metrics, ...input.metrics }, version: 1 } as Settings;
  if (!['circuit', 'cyberpunk', 'aurora'].includes(result.theme)) result.theme = defaults.theme;
  if (!['compact', 'expanded'].includes(result.size)) result.size = defaults.size;
  if (!['chaotic', 'playful', 'quiet'].includes(result.motion)) result.motion = defaults.motion;
  if (!['top-left', 'top-right', 'bottom-left', 'bottom-right'].includes(result.corner)) result.corner = defaults.corner;
  const clamp = (n: number, low: number, high: number, fallback: number) => Number.isFinite(n) ? Math.min(high, Math.max(low, n)) : fallback;
  result.opacity = clamp(result.opacity, .85, 1, 1);
  result.textScale = clamp(result.textScale, .9, 1.15, 1);
  result.monitor = Math.floor(clamp(result.monitor, 0, 100, 0));
  result.step = Math.floor(clamp(result.step, 1, 3, 1));
  result.warning = clamp(result.warning, 1, 100, 20);
  result.critical = clamp(result.critical, 0, result.warning, 10);
  for (const name of ['completed', 'alwaysOnTop', 'reducedMotion', 'launchAtLogin', 'notifications', 'codexEnabled'] as const) {
    result[name] = typeof result[name] === 'boolean' ? result[name] : defaults[name];
  }
  for (const name of ['cpu', 'ram', 'network', 'ai'] as const) result.metrics[name] = typeof result.metrics[name] === 'boolean' ? result.metrics[name] : true;
  if (typeof result.interface !== 'string') result.interface = 'auto';
  return result;
}
export const remaining = (used: number) => Math.max(0, Math.min(100, 100 - used));
export function windowStatus(window: UsageWindow, fetchedAt: number | null, now: number) {
  if (window.resetsAt !== null && now >= window.resetsAt) return 'Awaiting refresh';
  if (!fetchedAt || now - fetchedAt > 600) return 'Stale reading';
  return 'Current';
}
export function countdown(reset: number | null, now: number): string {
  if (reset === null) return 'Reset not reported';
  const seconds = Math.ceil(reset - now);
  if (seconds <= 0) return 'Awaiting refresh';
  const minutes = Math.ceil(seconds / 60);
  return `${Math.floor(minutes / 60)}h ${String(minutes % 60).padStart(2, '0')}m`;
}
export const gib = (bytes: number) => (bytes / 1073741824).toFixed(1);
export const rate = (bytes: number) => (bytes / 1048576).toFixed(bytes >= 10485760 ? 1 : 2);
export function severity(value: number, settings: Settings) {
  return value <= settings.critical ? 'critical' : value <= settings.warning ? 'warning' : 'normal';
}
export function selectedNetwork(snapshot: SystemSnapshot | null, name: string) {
  if (!snapshot) return undefined;
  const chosen = name === 'auto' ? snapshot.defaultInterface : name;
  return snapshot.networks.find(n => n.name === chosen);
}
export const sampleSystem: SystemSnapshot = {
  sampledAt: 0, cpu: 42, cores: [32, 54, 27, 66, 41, 35, 52, 29].map(usage => ({usage, frequencyMhz: 3200})),
  memory: { used: 10.4 * 1073741824, total: 32 * 1073741824, available: 21.6 * 1073741824 },
  networks: [{ name: 'Sample Wi-Fi', down: 2.3 * 1048576, up: .6 * 1048576, received: 450 * 1048576, transmitted: 36 * 1048576 }],
  defaultInterface: 'Sample Wi-Fi', temperatures: [],
};
export function sampleProviders(now: number): ProviderSnapshot {
  return { attention: [], usages: [
    { surface: 'codex', source: 'Sample data', state: 'connected', message: 'Codex allowance', fetchedAt: now, windows: [{ label: '5 hours', minutes: 300, usedPercent: 35, resetsAt: now + 7800 }] },
    { surface: 'claude', source: 'Sample data', state: 'connected', message: 'Shared Claude allowance', fetchedAt: now, windows: [{ label: '5 hours', minutes: 300, usedPercent: 62, resetsAt: now + 3600 }] },
  ]};
}
