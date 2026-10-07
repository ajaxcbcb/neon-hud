import { describe, expect, it } from 'vitest';
import { connectionView, idleAttempt, runConnection, type ConnectionAttempt } from './connections';
import type { Usage } from './model';

const usage = (state: Usage['state']): Usage => ({ surface: 'codex', state, message: state === 'needs-login' ? 'Finish sign-in in your browser.' : 'Account allowance', fetchedAt: 1, source: 'Fixture', windows: [] });

describe('connection feedback', () => {
  it('shows progress before a slow connection resolves', async () => {
    let resolve!: (value: string) => void;
    const task = new Promise<string>(done => { resolve = done; });
    let attempt = idleAttempt();
    const pending = runConnection(() => task, value => { attempt = value; }, 'Connecting…', 'Retry');
    expect(connectionView('codex', usage('connected'), attempt, false).tone).toBe('pending');
    resolve('Checked'); await pending;
    expect(connectionView('codex', usage('connected'), attempt, false).label).toBe('Connected');
  });
  it('distinguishes installed bridge from connected allowance and removal from cached readings', () => {
    expect(connectionView('claude-code', undefined, idleAttempt(), true).label).toBe('Waiting for readings');
    expect(connectionView('claude-code', usage('connected'), idleAttempt(), true).label).toBe('Connected');
    expect(connectionView('claude-code', usage('connected'), idleAttempt(), false).label).toBe('Not enabled');
  });
  it('shows sign-in and unavailable quota honestly after a successful command', () => {
    const done: ConnectionAttempt = { phase: 'done', message: 'Connected to account' };
    expect(connectionView('codex', usage('needs-login'), done, false).label).toBe('Sign-in needed');
    expect(connectionView('codex', usage('unavailable'), done, false).label).toBe('Not connected');
  });
  it('reports rejection, then clears failure on retry', async () => {
    let attempt = idleAttempt();
    const update = (value: ConnectionAttempt) => { attempt = value; };
    await runConnection(async () => { throw new Error('private detail'); }, update, 'Enabling…', 'Check settings access and retry.');
    expect(connectionView('claude-code', undefined, attempt, true)).toMatchObject({ label: 'Failed', message: 'Check settings access and retry.' });
    await runConnection(async () => 'Enabled', update, 'Enabling…', 'Retry');
    expect(connectionView('claude-code', undefined, attempt, true).tone).toBe('waiting');
  });
});
