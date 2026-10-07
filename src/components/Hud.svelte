<script lang="ts">
  import { tick } from 'svelte';
  import Icon from './Icon.svelte';
  import Gauge from './Gauge.svelte';
  import { displayMeter } from '../lib/display';
  import { selectedGpu, gpuLoad, gpuDetails } from '../lib/model';
  import { DrainTracker, drainText } from '../lib/drain';
  import { PressureTracker } from '../lib/pressure';
  import { stressColor, stressGradient } from '../lib/stress';
  import { resourceBudget } from '../lib/resources';
  import type { ResourcePolicy } from '../lib/resources';
  import { countdown, remaining, gib, rate, selectedNetwork, selectedDrives, drivePercent, severity, windowStatus } from '../lib/model';
  import type { Settings, SystemSnapshot, ProviderSnapshot } from '../lib/model';
  export let settings: Settings;
  export let snapshot: SystemSnapshot | null;
  export let providers: ProviderSnapshot;
  export let now: number;
  export let preview = false;
  export let history: { time: number; down: number; up: number }[] = [];
  export let resources: ResourcePolicy = { mode: 'normal', ...resourceBudget('normal'), reasons: [], suggestions: [] };
  export let drainTracker = new DrainTracker();
  export let onSettings = () => {};
  export let onExpand = () => {};
  export let onHide = () => {};
  export let onProvider = (_surface: string) => {};
  export let paused = false;
  export let onPause = () => {};
  export let onRefresh = () => {};
  export let onTheme = () => {};
  export let onMotion = () => {};
  export let onPin = () => {};
  let contextOpen = false;
  let hoverKey = '';
  let contextRoot: HTMLElement;
  let restoreFocus: HTMLElement | null = null;
  const pressureTracker = new PressureTracker();
  $: pressureTracker.observe(snapshot, settings);
  $: pressure = pressureTracker.reading(snapshot, settings, now);
  $: network = selectedNetwork(snapshot, preview ? 'auto' : settings.interface);
  $: drives = selectedDrives(snapshot, preview ? [] : settings.storageDriveIds);
  $: ram = snapshot?.memory.total ? snapshot.memory.used / snapshot.memory.total * 100 : null;
  $: gpu = selectedGpu(snapshot, settings.gpuId, now);
  $: gpuValue = gpuLoad(gpu, now);
  $: reduced = settings.reducedMotion || settings.motion === 'quiet' || resources.quiet;
  $: shownProviders = [providers.usages.find(p => p.surface === 'codex'), providers.usages.find(p => p.surface === 'claude')];
  $: expanded = settings.size === 'expanded';
  $: scale = Math.max(1048576, ...history.flatMap(p => [p.down, p.up]), network?.down || 0, network?.up || 0);
  $: hoverLines = [...(hoverKey === 'cpu' ? pressure.cpu : hoverKey === 'ram' ? pressure.memory : hoverKey === 'temperature' ? pressure.temperature : hoverKey.startsWith('drive:') && pressure.drives[hoverKey.slice(6)] ? [pressure.drives[hoverKey.slice(6)]] : []), ...hints(hoverKey, snapshot, providers, now)];
  $: if (hoverKey === 'resources') { resources; hoverLines = hints(hoverKey, snapshot, providers, now); }
  function hints(key: string, system: SystemSnapshot | null, ai: ProviderSnapshot, time: number): string[] {
    if (!key) return [];
    if (key === 'resources') return [...resources.reasons, `System every ${resources.systemSeconds}s · AI every ${resources.providerSeconds}s${resources.quiet ? ' · quiet motion' : ''}`, ...resources.suggestions];
    if (key === 'temperature') return ['High sensor temperature can affect performance. Adjust the threshold in Settings.', 'Sensor availability depends on hardware and the operating system.'];
    if (key === 'cpu') return system ? [`CPU · ${system.cpu?.toFixed(1) ?? '—'}% load · ${system.cores.length} logical cores`, `${system.cores[0]?.frequencyMhz || '—'} MHz reported clock`, ...system.temperatures.filter(sensor => /cpu|core|package|tctl|tdie|peci|processor/i.test(sensor.label)).slice(0, 2).map(sensor => `${sensor.label}: ${sensor.celsius.toFixed(1)} °C`)] : ['CPU readings require the desktop app'];
    if (key === 'gpu') return [...gpuDetails(selectedGpu(system, settings.gpuId, time), time), ...(gpu ? pressure.gpus[gpu.id] || [] : [])];
    if (key === 'ram') return system ? [`Memory · ${gib(system.memory.used)} / ${gib(system.memory.total)} GiB used`, `${gib(system.memory.available)} GiB available`] : ['Memory readings require the desktop app'];
    if (key === 'network') return network ? [network.name, `↓ ${rate(network.down)} · ↑ ${rate(network.up)} MiB/s`, `Interface lifetime: ↓ ${gib(network.received)} · ↑ ${gib(network.transmitted)} GiB`, 'Colors show relative activity, not network capacity or congestion.'] : ['No reading for the selected network interface'];
    if (key.startsWith('drive:')) {
      const drive = system?.drives.find(d => d.id === key.slice(6));
      return drive ? [`${drive.name} · ${drive.mount}`, `${gib(drive.usedBytes)} / ${gib(drive.totalBytes)} GiB used · ${gib(drive.availableBytes)} GiB available`, `Storage capacity cache: ${resources.mode === 'critical' ? 180 : resources.mode === 'pressure' ? 90 : 30} seconds`] : [];
    }
    const usage = ai.usages.find(u => u.surface === key);
    const quota = usage?.windows.find(w => w.minutes === 300) || usage?.windows[0];
    return quota && usage ? [`${key === 'codex' ? 'Codex account' : 'Claude shared account'} · ${remaining(quota.usedPercent).toFixed(1)}% remaining`, `${quota.label} · ${countdown(quota.resetsAt, time)} · ${windowStatus(quota, usage.fetchedAt, time)}`, drainText(drainTracker.reading(usage, quota, time)), `Source: ${usage.source}`] : [usage?.message || 'No supported account reading', 'Click for sources and connection details'];
  }
  function dismissMenu() { contextOpen = false; restoreFocus?.focus(); }
  function run(action: () => void) { contextOpen = false; action(); }
  function interactions(node: HTMLElement) {
    const show = (event: Event) => {
      const target = (event.target as HTMLElement).closest<HTMLElement>('[data-hint]');
      hoverKey = target?.dataset.hint || '';
    };
    const clear = () => { hoverKey = ''; };
    const menu = async (event: MouseEvent) => {
      event.preventDefault(); hoverKey = ''; contextOpen = true;
      restoreFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
      await tick(); contextRoot?.querySelector<HTMLButtonElement>('button')?.focus();
    };
    const keyboard = (event: KeyboardEvent) => {
      if ((event.shiftKey && event.key === 'F10') || event.key === 'ContextMenu') { event.preventDefault(); void menu(new MouseEvent('contextmenu')); }
      if (!contextOpen) return;
      if (event.key === 'Escape') { event.preventDefault(); dismissMenu(); }
      if (['ArrowDown', 'ArrowUp', 'Home', 'End'].includes(event.key)) {
        event.preventDefault();
        const items = [...contextRoot.querySelectorAll<HTMLButtonElement>('button')];
        const index = items.indexOf(document.activeElement as HTMLButtonElement);
        const next = event.key === 'Home' ? 0 : event.key === 'End' ? items.length - 1 : (index + (event.key === 'ArrowDown' ? 1 : -1) + items.length) % items.length;
        items[next]?.focus();
      }
    };
    const outside = (event: PointerEvent) => { if (contextOpen && !contextRoot?.contains(event.target as Node)) contextOpen = false; };
    node.addEventListener('pointerover', show); node.addEventListener('pointerleave', clear);
    node.addEventListener('focusin', show); node.addEventListener('focusout', clear);
    node.addEventListener('contextmenu', menu); node.addEventListener('keydown', keyboard);
    window.addEventListener('pointerdown', outside);
    return { destroy() {
      node.removeEventListener('pointerover', show); node.removeEventListener('pointerleave', clear);
      node.removeEventListener('focusin', show); node.removeEventListener('focusout', clear);
      node.removeEventListener('contextmenu', menu); node.removeEventListener('keydown', keyboard);
      window.removeEventListener('pointerdown', outside);
    }};
  }
</script>

<section use:interactions class="hud" class:expanded class:preview class:with-storage={settings.metrics.storage} aria-label={preview ? 'HUD preview with sample data' : 'Performance HUD'}>
  <header class="hud-header" data-tauri-drag-region>
    <div class="hud-brand" data-tauri-drag-region><span class="brand-mark">N</span><span data-tauri-drag-region>NEON <b>HUD</b></span></div>
    <span class="status-dot" class:sample={preview || paused}></span><span class="live-label">{preview ? 'SAMPLE DATA' : paused ? 'PAUSED' : snapshot ? 'SYSTEM LIVE' : 'NO SYSTEM SOURCE'}</span>
    {#if pressure.temperature.length}<button class="pressure-badge" data-hint="temperature" aria-label="High reported temperature" aria-describedby={hoverKey === 'temperature' ? 'metric-tooltip' : undefined}>!</button>{/if}
    {#if !preview}<div class="hud-actions">
      <button class="icon-button" aria-label={expanded ? 'Compact HUD' : 'Expand HUD'} onclick={onExpand}><Icon name="expand" size={14}/></button>
      <button class="icon-button" aria-label="Open configuration" onclick={onSettings}><Icon name="settings" size={14}/></button>
      <button class="icon-button" aria-label="Hide to tray" onclick={onHide}><Icon name="close" size={14}/></button>
    </div>{/if}
  </header>

  <div class="system-row">
    {#if settings.metrics.cpu}<button class="cpu-module metric-tile" class:under-pressure={pressure.cpu.length > 0} data-hint="cpu" aria-label={`CPU measurement details${pressure.cpu.length ? ', performance pressure' : ''}`} aria-describedby={hoverKey === 'cpu' ? 'metric-tooltip' : undefined}><div class="metric-label"><Icon name="cpu" size={13}/> CPU{#if pressure.cpu.length}<span class="pressure-badge" aria-label="CPU performance pressure">!</span>{/if}</div><Gauge value={snapshot?.cpu ?? null} label="LOAD" {reduced}/>
      {#if expanded}<span class="metric-note">{snapshot?.cores.length || '—'} cores · {snapshot?.cores[0]?.frequencyMhz ? (snapshot.cores[0].frequencyMhz / 1000).toFixed(2) + ' GHz' : 'Clock unavailable'}</span>{/if}
    </button>{/if}
    {#if settings.metrics.gpu}<button class="gpu-module metric-tile" class:under-pressure={!!gpu && !!pressure.gpus[gpu.id]?.length} data-hint="gpu" aria-label="GPU measurement details" aria-describedby={hoverKey === 'gpu' ? 'metric-tooltip' : undefined}><div class="metric-label"><Icon name="gpu" size={13}/> GPU{#if gpu && pressure.gpus[gpu.id]?.length}<span class="pressure-badge" aria-label="GPU performance pressure">!</span>{/if}</div><Gauge value={gpuValue} label="LOAD" {reduced}/><span class="metric-note">{gpu?.name || 'Adapter unavailable'}</span>{#if expanded}<span class="metric-note">{gpuDetails(gpu, now)[2] || 'Memory unavailable'}</span>{/if}</button>{/if}
    {#if settings.metrics.ram}<button class="ram-module metric-tile" class:under-pressure={pressure.memory.length > 0} data-hint="ram" aria-label={`Memory measurement details${pressure.memory.length ? ', performance pressure' : ''}`} aria-describedby={hoverKey === 'ram' ? 'metric-tooltip' : undefined}><div class="metric-label"><Icon name="ram" size={13}/> MEMORY{#if pressure.memory.length}<span class="pressure-badge" aria-label="Memory performance pressure">!</span>{/if}</div>
      <div class="metric-value" style={`color:${stressColor(ram)}`}>{ram === null ? '—' : ram.toFixed(0)}<span>%</span></div>
      <div class="meter" role="meter" aria-label="RAM usage" aria-valuenow={ram ?? undefined} aria-valuemin="0" aria-valuemax="100"><span use:displayMeter={{value: ram, reduced}} style={`background:${stressGradient(ram)}`}></span></div>
      <div class="metric-note">{snapshot ? `${gib(snapshot.memory.used)} / ${gib(snapshot.memory.total)} GiB` : 'Unavailable'}</div>
      {#if expanded && snapshot}<div class="metric-note">{gib(snapshot.memory.available)} GiB available</div>{/if}
    </button>{/if}
    {#if settings.metrics.network}<button class="network-module metric-tile" data-hint="network" aria-label="Network measurement details" aria-describedby={hoverKey === 'network' ? 'metric-tooltip' : undefined}><div class="metric-label"><Icon name="network" size={13}/> NETWORK</div>
      <div class="transfer"><span>↓</span><b>{network ? rate(network.down) : '—'}</b><small>MiB/s</small></div>
      <div class="meter network-down"><span use:displayMeter={{value: network ? network.down / scale * 100 : null, reduced}} style={`background:${stressGradient(network ? network.down / scale * 100 : null)}`}></span></div>
      <div class="transfer"><span>↑</span><b>{network ? rate(network.up) : '—'}</b><small>MiB/s</small></div>
      <div class="meter network-up"><span use:displayMeter={{value: network ? network.up / scale * 100 : null, reduced}} style={`background:${stressGradient(network ? network.up / scale * 100 : null)}`}></span></div>
      {#if expanded}<div class="metric-note">{network?.name || 'Select an interface'}</div>{/if}
    </button>{/if}
  </div>

  {#if settings.metrics.storage}<div class="storage-strip" aria-label="Selected drive storage usage"><Icon name="storage" size={13}/><div class="storage-items">
    {#each drives as drive}{@const percent = drivePercent(drive)}<button class="drive-chip" class:drive-full={!!pressure.drives[drive.id]} data-hint={`drive:${drive.id}`} aria-label={`${drive.name} ${drive.mount}: ${percent?.toFixed(0) ?? 'Unknown'} percent used${pressure.drives[drive.id] ? ', low free space' : ''}`} aria-describedby={hoverKey === `drive:${drive.id}` ? 'metric-tooltip' : undefined}><span>{drive.mount || drive.name}</span>{#if pressure.drives[drive.id]}<span class="pressure-badge" aria-label="Low free space">!</span>{/if}<div class="meter"><span use:displayMeter={{value: percent, reduced}} style={`background:${stressGradient(percent)}`}></span></div><b>{percent?.toFixed(0) ?? '—'}%</b></button>{/each}
    {#if !drives.length}<span class="storage-empty">{snapshot ? 'Selected drives are unavailable' : 'Storage unavailable'}</span>{/if}
  </div></div>{/if}

  {#if settings.metrics.ai}<div class="ai-row">
    {#each shownProviders as usage, index}
      {@const surface = index === 0 ? 'codex' : 'claude'}
      {@const quota = usage?.windows.find(w => w.minutes === 300) || usage?.windows[0]}
      {@const status = quota ? windowStatus(quota, usage?.fetchedAt ?? null, now) : ''}
      {@const value = quota ? remaining(quota.usedPercent) : null}
      {@const drain = usage && quota ? drainTracker.reading(usage, quota, now) : null}
      {@const tokens = usage ? drainTracker.tokenReading(usage, now) : null}
      {@const alerts = providers.attention.filter(a => index === 0 ? a.surface === 'codex' : a.surface === 'claude' || a.surface === 'claude-code')}
      <button class="ai-module" data-hint={surface} aria-describedby={hoverKey === surface ? 'metric-tooltip' : undefined} class:has-question={alerts.length > 0} onclick={() => onProvider(surface)} aria-label={`${index === 0 ? 'ChatGPT / Codex' : 'Claude'} usage details${alerts.length ? ', requires attention' : ''}`}>
        <div class="ai-copy">
          <div class="ai-title">{#key alerts.map(a => a.id).join(',')}<span class:shake={alerts.length > 0}><Icon name={surface} size={17}/></span>{/key}<b>{index === 0 ? 'ChatGPT' : 'Claude'}</b>{#if drain?.fast || tokens?.fast}<span class="fast-drain-badge" title="Consumption is faster than the comparison pace">⚡</span>{/if}{#if alerts.length}<span class="question-badge">?</span>{/if}</div>
          <span class="ai-surface">{index === 0 ? 'CODEX ALLOWANCE' : 'SHARED ALLOWANCE'}</span>
          <span class="ai-reset">{quota ? `${quota.label} · ${countdown(quota.resetsAt, now)}` : usage?.state === 'needs-login' ? 'Sign in to connect' : 'Usage unavailable'}</span>
          {#if status && status !== 'Current'}<span class="stale-label">{status}</span>{/if}
          {#if expanded && quota}<span class="drain-label" class:fast-drain={drain?.fast}>{drain?.fast ? '⚡ Fast drain · ' : ''}{drainText(drain)}</span>{/if}
          {#if expanded && usage?.tokenUsage}<span class="drain-label" class:fast-drain={tokens?.fast}>{tokens ? `${tokens.fast ? '⚡ Token surge · ' : ''}${tokens.perMinute.toFixed(0)} tokens/min avg · ${(tokens.observedSeconds / 60).toFixed(0)} min` : 'Tokens: collecting ≥2 min of readings'}</span>{/if}
        </div>
        <Gauge value={value} label="LEFT" {reduced} inverse tone={value === null || status !== 'Current' ? 'unavailable' : severity(value, settings)}/>
      </button>
    {/each}
  </div>{/if}

  {#if expanded}
    {#if settings.metrics.gpu}<div class="detail-section gpu-details"><div class="detail-title">GRAPHICS ADAPTERS <span>GPU readings cached · {resources.mode === 'normal' ? 1 : resources.systemSeconds}s</span></div>{#each snapshot?.gpus || [] as adapter}<article class="gpu-detail"><b><Icon name="gpu" size={13}/>{adapter.name}{#if pressure.gpus[adapter.id]?.length}<span class="pressure-badge">!</span>{/if}</b><p>{gpuDetails(adapter, now).slice(1).join(' · ')}</p></article>{/each}{#if !snapshot?.gpus?.length}<p class="metric-note">No graphics adapter reported.</p>{/if}</div>{/if}
    {#if settings.metrics.storage}<div class="detail-section drive-details"><div class="detail-title">DRIVE CAPACITY <span>Selected volumes · GiB</span></div>{#each drives as drive}{@const percent = drivePercent(drive)}<div class="drive-detail"><span><b>{drive.name || drive.mount}</b><small>{drive.mount}</small></span><div class="meter"><span use:displayMeter={{value: percent, reduced}} style={`background:${stressGradient(percent)}`}></span></div><strong>{percent?.toFixed(1) ?? '—'}%</strong><small>{gib(drive.usedBytes)} / {gib(drive.totalBytes)} used · {gib(drive.availableBytes)} free</small></div>{/each}</div>{/if}
    {#if settings.metrics.cpu}<div class="detail-section"><div class="detail-title">CORE ACTIVITY <span>{snapshot?.temperatures[0] ? `${snapshot.temperatures[0].celsius.toFixed(0)} °C · ${snapshot.temperatures[0].label}` : 'Temperature not reported'}</span></div>
      <div class="core-chart" aria-label="Per-core CPU utilization">
        {#each snapshot?.cores || [] as core, index}<div class="core" title={`Core ${index + 1}: ${core.usage.toFixed(1)}%`}><div class="core-track"><span style={`height:${core.usage}%;background:${stressGradient(core.usage, false, true)}`}></span></div><small>{index + 1}</small></div>{/each}
        {#if !snapshot}<span class="metric-note">Native system readings are available in the desktop app.</span>{/if}
      </div>
    </div>{/if}
    {#if settings.metrics.network}<div class="detail-section network-history"><div class="detail-title">RECENT TRAFFIC <span>Relative activity · 60s · {history.length} samples</span></div>
      <div class="history-chart" aria-label="Network traffic history">
        {#each history as point}<div class="history-point" style={`left:${(point.time - now + 60) / 60 * 100}%`} title={`${rate(point.down)} down / ${rate(point.up)} up MiB/s`}><span style={`height:${point.down / scale * 100}%;background:${stressGradient(point.down / scale * 100, false, true)}`}></span><i style={`height:${point.up / scale * 100}%;background:${stressGradient(point.up / scale * 100, false, true)}`}></i></div>{/each}
      </div><div class="history-total">↓ {network ? gib(network.received) : '—'} GiB received <span>↑ {network ? gib(network.transmitted) : '—'} GiB sent · interface lifetime</span></div>
    </div>{/if}
    {#if !preview}<div class="detail-section resource-strategy"><div class="detail-title">SMART RESOURCE MODE <span>{settings.resources.adaptive ? resources.mode.toUpperCase() : 'OFF'}</span></div><p>System checks {resources.systemSeconds}s · AI checks {resources.providerSeconds}s{resources.quiet ? ' · motion quieted' : ''}</p>{#each resources.reasons as reason}<p>{reason}</p>{/each}{#each resources.suggestions as suggestion}<p>{suggestion}</p>{/each}</div>{/if}
    {#if settings.metrics.ai}<div class="quota-details">
      {#each shownProviders as usage}{#each usage?.windows || [] as window}
        <div class="quota-line"><span>{usage?.surface === 'codex' ? 'Codex' : 'Claude'} · {window.label}</span><div class="meter"><span use:displayMeter={{value: remaining(window.usedPercent), reduced}} style={`background:${stressGradient(remaining(window.usedPercent), true)}`}></span></div><b>{remaining(window.usedPercent).toFixed(0)}% left</b><small>{countdown(window.resetsAt, now)}</small></div>
        <div class="drain-detail">{drainText(drainTracker.reading(usage!, window, now))} · average of up to 30 min</div>
      {/each}{/each}
    </div>{/if}
  {/if}
  <footer class="hud-footer">{#if preview}<span>Preview only · no account connection</span>{:else}<button class="resource-chip" class:saving={resources.quiet} data-hint="resources" aria-label={`Resource strategy: ${resources.mode}`} aria-describedby={hoverKey === 'resources' ? 'metric-tooltip' : undefined} onclick={onExpand}><Icon name="orbit" size={9}/>{settings.resources.adaptive ? `AUTO · ${resources.mode === 'normal' ? 'READY' : resources.mode === 'critical' ? 'SAVE++' : 'SAVE'}` : 'AUTO OFF'}</button>{/if}<span>Hover details · Right-click controls</span></footer>
  {#if hoverLines.length && !contextOpen}<div id="metric-tooltip" class="hover-info" role="tooltip">{#each hoverLines as line}<span>{line}</span>{/each}</div>{/if}
  {#if contextOpen}<div class="hud-context" bind:this={contextRoot} role="menu" aria-label="HUD quick controls"><span class="context-label">QUICK MISCHIEF <button role="menuitem" aria-label="Close quick controls" onclick={dismissMenu}><Icon name="close" size={13}/></button></span><div class="context-grid">
    {#if !preview}<button role="menuitem" onclick={() => run(onExpand)}><Icon name="expand" size={15}/>{expanded ? 'Go tiny' : 'Expand'}</button>
    <button role="menuitem" onclick={() => run(onPause)}><Icon name="pause" size={15}/>{paused ? 'Resume readings' : 'Pause readings'}</button>
    <button role="menuitem" onclick={() => run(onPin)}><Icon name="pin" size={15}/>{settings.alwaysOnTop ? 'Unpin HUD' : 'Keep on top'}</button>{/if}
    <button role="menuitem" onclick={() => run(onTheme)}><Icon name="star" size={15}/>Next theme</button>
    <button role="menuitem" onclick={() => run(onMotion)}><Icon name="orbit" size={15}/>{settings.motion === 'quiet' ? 'Bring the chaos' : 'Quiet motion'}</button>
    {#if !preview}<button role="menuitem" onclick={() => run(onRefresh)}><Icon name="network" size={15}/>Refresh now</button>
    <button role="menuitem" onclick={() => run(onSettings)}><Icon name="settings" size={15}/>Settings & drives</button>
    <button role="menuitem" onclick={() => run(onHide)}><Icon name="close" size={15}/>Hide to tray</button>{/if}
  </div><small>Esc to close · Shift+F10 to open</small></div>{/if}
</section>
