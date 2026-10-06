<script lang="ts">
  import Icon from './Icon.svelte';
  import Gauge from './Gauge.svelte';
  import { DrainTracker, drainText } from '../lib/drain';
  import { countdown, remaining, gib, rate, selectedNetwork, severity, windowStatus } from '../lib/model';
  import type { Settings, SystemSnapshot, ProviderSnapshot } from '../lib/model';
  export let settings: Settings;
  export let snapshot: SystemSnapshot | null;
  export let providers: ProviderSnapshot;
  export let now: number;
  export let preview = false;
  export let history: { down: number; up: number }[] = [];
  export let drainTracker = new DrainTracker();
  export let onSettings = () => {};
  export let onExpand = () => {};
  export let onHide = () => {};
  export let onProvider = (_surface: string) => {};
  $: network = selectedNetwork(snapshot, preview ? 'auto' : settings.interface);
  $: ram = snapshot?.memory.total ? snapshot.memory.used / snapshot.memory.total * 100 : null;
  $: shownProviders = [providers.usages.find(p => p.surface === 'codex'), providers.usages.find(p => p.surface === 'claude')];
  $: expanded = settings.size === 'expanded';
  $: scale = Math.max(1048576, ...history.flatMap(p => [p.down, p.up]), network?.down || 0, network?.up || 0);
  $: attentionIds = providers.attention.map(a => a.id).join(',');
</script>

<section class="hud" class:expanded class:preview aria-label={preview ? 'HUD preview with sample data' : 'Performance HUD'}>
  <header class="hud-header" data-tauri-drag-region>
    <div class="hud-brand" data-tauri-drag-region><span class="brand-mark">N</span><span data-tauri-drag-region>NEON <b>HUD</b></span></div>
    <span class="status-dot" class:sample={preview}></span><span class="live-label">{preview ? 'SAMPLE DATA' : snapshot ? 'SYSTEM LIVE' : 'NO SYSTEM SOURCE'}</span>
    {#if !preview}<div class="hud-actions">
      <button class="icon-button" aria-label={expanded ? 'Compact HUD' : 'Expand HUD'} onclick={onExpand}><Icon name="expand" size={14}/></button>
      <button class="icon-button" aria-label="Open configuration" onclick={onSettings}><Icon name="settings" size={14}/></button>
      <button class="icon-button" aria-label="Hide to tray" onclick={onHide}><Icon name="close" size={14}/></button>
    </div>{/if}
  </header>

  <div class="system-row">
    {#if settings.metrics.cpu}<div class="cpu-module"><div class="metric-label"><Icon name="cpu" size={13}/> CPU</div><Gauge value={snapshot?.cpu ?? null} label="LOAD"/>
      {#if expanded}<span class="metric-note">{snapshot?.cores.length || '—'} cores · {snapshot?.cores[0]?.frequencyMhz ? (snapshot.cores[0].frequencyMhz / 1000).toFixed(2) + ' GHz' : 'Clock unavailable'}</span>{/if}
    </div>{/if}
    {#if settings.metrics.ram}<div class="ram-module"><div class="metric-label"><Icon name="ram" size={13}/> MEMORY</div>
      <div class="metric-value">{ram === null ? '—' : ram.toFixed(0)}<span>%</span></div>
      <div class="meter" role="meter" aria-label="RAM usage" aria-valuenow={ram ?? undefined} aria-valuemin="0" aria-valuemax="100"><span style={`width:${ram || 0}%`}></span></div>
      <div class="metric-note">{snapshot ? `${gib(snapshot.memory.used)} / ${gib(snapshot.memory.total)} GiB` : 'Unavailable'}</div>
      {#if expanded && snapshot}<div class="metric-note">{gib(snapshot.memory.available)} GiB available</div>{/if}
    </div>{/if}
    {#if settings.metrics.network}<div class="network-module"><div class="metric-label"><Icon name="network" size={13}/> NETWORK</div>
      <div class="transfer"><span>↓</span><b>{network ? rate(network.down) : '—'}</b><small>MiB/s</small></div>
      <div class="meter network-down"><span style={`width:${network ? Math.min(100, network.down / scale * 100) : 0}%`}></span></div>
      <div class="transfer"><span>↑</span><b>{network ? rate(network.up) : '—'}</b><small>MiB/s</small></div>
      <div class="meter network-up"><span style={`width:${network ? Math.min(100, network.up / scale * 100) : 0}%`}></span></div>
      {#if expanded}<div class="metric-note">{network?.name || 'Select an interface'}</div>{/if}
    </div>{/if}
  </div>

  {#if settings.metrics.ai}<div class="ai-row">
    {#each shownProviders as usage, index}
      {@const surface = index === 0 ? 'codex' : 'claude'}
      {@const quota = usage?.windows.find(w => w.minutes === 300) || usage?.windows[0]}
      {@const status = quota ? windowStatus(quota, usage?.fetchedAt ?? null, now) : ''}
      {@const value = quota ? remaining(quota.usedPercent) : null}
      {@const drain = usage && quota ? drainTracker.reading(usage, quota, now) : null}
      {@const tokens = usage ? drainTracker.tokenReading(usage, now) : null}
      {@const alerts = providers.attention.filter(a => index === 0 ? a.surface === 'codex' : a.surface === 'claude' || a.surface === 'claude-code')}
      <button class="ai-module" class:has-question={alerts.length > 0} onclick={() => onProvider(surface)} aria-label={`${index === 0 ? 'ChatGPT / Codex' : 'Claude'} usage details${alerts.length ? ', requires attention' : ''}`}>
        <div class="ai-copy">
          <div class="ai-title">{#key attentionIds}<span class:shake={alerts.length > 0}><Icon name={surface} size={17}/></span>{/key}<b>{index === 0 ? 'ChatGPT' : 'Claude'}</b>{#if drain?.fast || tokens?.fast}<span class="fast-drain-badge" title="Consumption is faster than the comparison pace">⚡</span>{/if}{#if alerts.length}<span class="question-badge">?</span>{/if}</div>
          <span class="ai-surface">{index === 0 ? 'CODEX ALLOWANCE' : 'SHARED ALLOWANCE'}</span>
          <span class="ai-reset">{quota ? `${quota.label} · ${countdown(quota.resetsAt, now)}` : usage?.state === 'needs-login' ? 'Sign in to connect' : 'Usage unavailable'}</span>
          {#if status && status !== 'Current'}<span class="stale-label">{status}</span>{/if}
          {#if expanded && quota}<span class="drain-label" class:fast-drain={drain?.fast}>{drain?.fast ? '⚡ Fast drain · ' : ''}{drainText(drain)}</span>{/if}
          {#if expanded && usage?.tokenUsage}<span class="drain-label" class:fast-drain={tokens?.fast}>{tokens ? `${tokens.fast ? '⚡ Token surge · ' : ''}${tokens.perMinute.toFixed(0)} tokens/min avg · ${(tokens.observedSeconds / 60).toFixed(0)} min` : 'Tokens: collecting ≥2 min of readings'}</span>{/if}
        </div>
        <Gauge value={value} label="LEFT" tone={value === null || status !== 'Current' ? 'unavailable' : severity(value, settings)}/>
      </button>
    {/each}
  </div>{/if}

  {#if expanded}
    {#if settings.metrics.cpu}<div class="detail-section"><div class="detail-title">CORE ACTIVITY <span>{snapshot?.temperatures[0] ? `${snapshot.temperatures[0].celsius.toFixed(0)} °C · ${snapshot.temperatures[0].label}` : 'Temperature not reported'}</span></div>
      <div class="core-chart" aria-label="Per-core CPU utilization">
        {#each snapshot?.cores || [] as core, index}<div class="core" title={`Core ${index + 1}: ${core.usage.toFixed(1)}%`}><div class="core-track"><span style={`height:${core.usage}%`}></span></div><small>{index + 1}</small></div>{/each}
        {#if !snapshot}<span class="metric-note">Native system readings are available in the desktop app.</span>{/if}
      </div>
    </div>{/if}
    {#if settings.metrics.network}<div class="detail-section network-history"><div class="detail-title">RECENT TRAFFIC <span>Relative scale · last 60 seconds</span></div>
      <div class="history-chart" aria-label="Network traffic history">
        {#each history as point}<div class="history-point"><span style={`height:${point.down / scale * 100}%`}></span><i style={`height:${point.up / scale * 100}%`}></i></div>{/each}
      </div><div class="history-total">↓ {network ? gib(network.received) : '—'} GiB received <span>↑ {network ? gib(network.transmitted) : '—'} GiB sent · interface lifetime</span></div>
    </div>{/if}
    {#if settings.metrics.ai}<div class="quota-details">
      {#each shownProviders as usage}{#each usage?.windows || [] as window}
        <div class="quota-line"><span>{usage?.surface === 'codex' ? 'Codex' : 'Claude'} · {window.label}</span><div class="meter"><span style={`width:${remaining(window.usedPercent)}%`}></span></div><b>{remaining(window.usedPercent).toFixed(0)}% left</b><small>{countdown(window.resetsAt, now)}</small></div>
        <div class="drain-detail">{drainText(drainTracker.reading(usage!, window, now))} · average of up to 30 min</div>
      {/each}{/each}
    </div>{/if}
  {/if}
  <footer class="hud-footer"><span>{preview ? 'Preview only · no account connection' : 'LOCAL FIRST'}</span><span>{preview ? 'NEON CIRCUIT / 01' : 'Click an AI icon for all limits'}</span></footer>
</section>
