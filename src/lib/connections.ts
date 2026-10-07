import type { Surface, Usage } from './model';

export interface ConnectionAttempt {
  phase: 'idle' | 'pending' | 'done' | 'error';
  message: string;
}
export const idleAttempt = (): ConnectionAttempt => ({ phase: 'idle', message: '' });

export async function runConnection(
  perform: () => Promise<string>,
  update: (attempt: ConnectionAttempt) => void,
  progress: string,
  failure: string,
): Promise<void> {
  update({ phase: 'pending', message: progress });
  try { update({ phase: 'done', message: await perform() }); }
  catch { update({ phase: 'error', message: failure }); }
}

export function connectionView(surface: Surface, usage: Usage | undefined, attempt: ConnectionAttempt, bridgeEnabled: boolean) {
  if (attempt.phase === 'pending') return { label: 'Working…', tone: 'pending', message: attempt.message };
  if (attempt.phase === 'error') return { label: 'Failed', tone: 'error', message: attempt.message };
  if (surface === 'chatgpt') return { label: 'Limited access', tone: 'idle', message: usage?.message || 'No supported chat quota source.' };
  if (surface === 'claude-code' && !bridgeEnabled) return { label: 'Not enabled', tone: 'idle', message: attempt.message || 'Enable the bridge to receive readings from new Claude Code sessions.' };
  if (usage?.state === 'connected') return { label: 'Connected', tone: 'connected', message: usage.message };
  if (usage?.state === 'needs-login') return { label: 'Sign-in needed', tone: 'waiting', message: usage.message };
  if (usage?.state === 'error') return { label: 'Not connected', tone: 'error', message: usage.message };
  if ((surface === 'claude-code' || surface === 'claude') && bridgeEnabled) return { label: 'Waiting for readings', tone: 'waiting', message: 'Bridge enabled. Start a new Claude Code session and send a message to receive supported readings.' };
  return { label: 'Not connected', tone: 'idle', message: usage?.message || attempt.message || 'Connect a local source to receive readings.' };
}
