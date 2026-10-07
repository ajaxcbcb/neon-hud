import { invoke, isTauri } from '@tauri-apps/api/core';
import type { Settings, SystemSnapshot, ProviderSnapshot } from './model';
import type { ResourceMode } from './resources';
import { defaults, normalizeSettings } from './model';

export const native = isTauri();
export async function loadSettings(): Promise<Settings> {
  if (native) return normalizeSettings(await invoke('load_settings'));
  try { return normalizeSettings(JSON.parse(localStorage.getItem('neon-hud-settings') || '{}')); }
  catch { return structuredClone(defaults); }
}
export async function saveSettings(settings: Settings): Promise<void> {
  if (native) return invoke('save_settings', { settings });
  localStorage.setItem('neon-hud-settings', JSON.stringify(settings));
}
export async function systemSnapshot(resourceMode: ResourceMode = 'normal', samplingMs = 250): Promise<SystemSnapshot | null> {
  return native ? invoke('system_snapshot', { resourceMode, samplingMs }) : null;
}
export async function providerSnapshot(): Promise<ProviderSnapshot> {
  if (native) return invoke('provider_snapshot');
  return { attention: [], usages: ['chatgpt', 'codex', 'claude', 'claude-code'].map(surface => ({
    surface, source: 'Browser preview', state: 'unavailable', fetchedAt: null, windows: [],
    message: 'Install the desktop app to connect local sources.',
  })) } as ProviderSnapshot;
}
export async function openLink(kind: string) {
  if (native) return invoke('open_link', { kind });
  const links: Record<string, string> = {
    chatgpt: 'https://chatgpt.com', codex: 'https://chatgpt.com/codex/settings/usage',
    claude: 'https://claude.ai/settings/usage', 'claude-code': 'https://code.claude.com/docs/en/setup',
    'codex-install': 'https://developers.openai.com/codex/cli',
  };
  if (links[kind]) window.open(links[kind], '_blank', 'noopener,noreferrer');
}
