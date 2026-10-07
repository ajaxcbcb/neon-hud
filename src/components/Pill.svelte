<script lang="ts">
  import { onDestroy } from 'svelte';
  import Icon from './Icon.svelte';
  import type { Settings, SystemSnapshot, ProviderSnapshot, Surface } from '../lib/model';
  import { selectedNetwork, selectedDrives, drivePercent, gib, rate, remaining, countdown, windowStatus } from '../lib/model';
  import { PressureTracker } from '../lib/pressure';
  import { DrainTracker, drainText } from '../lib/drain';
  import { PILL_HEIGHT } from '../lib/layout';
  export let settings: Settings;
  export let snapshot: SystemSnapshot | null = null;
  export let providers: ProviderSnapshot = { usages: [], attention: [] };
  export let now = Date.now() / 1000;
  export let preview = false;
  export let paused = false;
  export let drainTracker = new DrainTracker();
  export let onSettings = () => {};
  export let onExpand = () => {};
  export let onHide = () => {};
  export let onPause = () => {};
  export let onRefresh = () => {};
  export let onTheme = () => {};
  export let onMotion = () => {};
  export let onPin = () => {};
  export let onProvider = (_surface: string) => {};
  export let onSpace = (_height: number) => {};
  const tracker = new PressureTracker();
  let active = '';
  let menu = false;
  let closeTimer: ReturnType<typeof setTimeout>;
  let shell: HTMLElement;
  let menuButton: HTMLButtonElement;
  $: tracker.observe(snapshot, settings);
  $: pressure = tracker.reading(snapshot, settings, now);
  $: network = selectedNetwork(snapshot, settings.interface);
  $: drives = selectedDrives(snapshot, settings.storageDriveIds);
  $: hottestDrive = [...drives].sort((a, b) => (drivePercent(b) || 0) - (drivePercent(a) || 0))[0];
  $: memory = snapshot?.memory.total ? snapshot.memory.used / snapshot.memory.total * 100 : null;
  $: systemStale = !snapshot || now - snapshot.sampledAt > 12;
  $: cells = [
    ...(settings.metrics.cpu ? [{ id: 'cpu', name: 'CPU', value: systemStale ? '—' : `${snapshot?.cpu?.toFixed(0) ?? '—'}%`, stress: systemStale ? 0 : snapshot?.cpu || 0, warning: pressure.cpu.length > 0, detail: systemStale ? 'Waiting for current CPU readings.' : `${snapshot?.cpu?.toFixed(1) ?? 'Unavailable'}% load · ${snapshot?.cores.length || 0} cores\n${snapshot?.temperatures.length ? snapshot.temperatures.map(t => `${t.label}: ${t.celsius.toFixed(1)} °C`).join(' · ') : 'Temperature sensor unavailable'}\n${pressure.cpu.join(' · ') || 'Click for core meters and graphs.'}` }] : []),
    ...(settings.metrics.ram ? [{ id: 'ram', name: 'Memory', value: systemStale ? '—' : `${memory?.toFixed(0) ?? '—'}%`, stress: systemStale ? 0 : memory || 0, warning: pressure.memory.length > 0, detail: snapshot && !systemStale ? `${gib(snapshot.memory.used)} / ${gib(snapshot.memory.total)} GiB used\n${gib(snapshot.memory.available)} GiB available\n${pressure.memory.join(' · ') || 'Click for memory details.'}` : 'Waiting for current memory readings.' }] : []),
    ...(settings.metrics.network ? [{ id: 'network', name: 'Network', value: network && !systemStale ? rate(network.down) : '—', stress: 0, warning: false, detail: network && !systemStale ? `${network.name}\n↓ ${rate(network.down)} MiB/s · ↑ ${rate(network.up)} MiB/s\nReceived ${gib(network.received)} GiB · sent ${gib(network.transmitted)} GiB\nClick for the traffic graph.` : 'Selected network interface unavailable.' }] : []),
    ...(settings.metrics.storage ? [{ id: 'storage', name: 'Storage', value: hottestDrive && !systemStale ? `${drivePercent(hottestDrive)?.toFixed(0) ?? '—'}%` : '—', stress: hottestDrive && !systemStale ? drivePercent(hottestDrive) || 0 : 0, warning: drives.some(d => pressure.drives[d.id]), detail: drives.length && !systemStale ? `Most full selected drive shown · ${drives.length} selected\n${drives.map(d => `${d.name || d.mount}: ${gib(d.usedBytes)} / ${gib(d.totalBytes)} GiB · ${gib(d.availableBytes)} GiB free`).join('\n')}\n${drives.map(d => pressure.drives[d.id]).filter(Boolean).join(' · ')}` : 'Selected drives unavailable.' }] : []),
  ];
  $: aiCells = settings.metrics.ai ? (['codex', 'claude'] as Surface[]).map(surface => {
    const usage = providers.usages.find(u => u.surface === surface);
    const quota = usage?.windows.find(w => w.minutes === 300) || usage?.windows[0];
    const current = usage?.state === 'connected' && quota && windowStatus(quota, usage.fetchedAt, now) === 'Current';
    const drain = usage && quota ? drainTracker.reading(usage, quota, now) : null;
    const tokens = usage ? drainTracker.tokenReading(usage, now) : null;
    const attention = providers.attention.filter(a => surface === 'claude' ? ['claude', 'claude-code'].includes(a.surface) : a.surface === surface);
    return { id: surface, name: surface === 'codex' ? 'Codex allowance' : 'Claude allowance', value: current && quota ? `${remaining(quota.usedPercent).toFixed(0)}%` : '—', stress: current && quota ? quota.usedPercent : 0, warning: current && quota ? remaining(quota.usedPercent) <= settings.warning : false, question: attention.length > 0, attentionKey: attention.map(a => a.id).join(','), fast: !!(drain?.fast || tokens?.fast), detail: `${surface === 'codex' ? 'Codex account · ChatGPT chat quota is separate' : 'Claude shared account'}\n${current && quota ? `${remaining(quota.usedPercent).toFixed(1)}% left · ${quota.label}\n${countdown(quota.resetsAt, now)} · ${drainText(drain)}` : usage?.message || 'No supported reading'}\n${usage?.source || 'Local source'}${attention.length ? '\nQuestion needs you · click to view' : ''}` };
  }) : [];
  $: detail = [...cells, ...aiCells].find(cell => cell.id === active);
  $: onSpace(menu ? 328 : detail ? 208 : PILL_HEIGHT);
  function hover(id: string) { clearTimeout(closeTimer); if (!menu) active = id; }
  function leave() { clearTimeout(closeTimer); if (!menu) closeTimer = setTimeout(() => active = '', 130); }
  function openMenu() { clearTimeout(closeTimer); active = ''; menu = !menu; if (menu) queueMicrotask(() => shell.querySelector<HTMLButtonElement>('.pill-menu button')?.focus()); }
  function action(callback: () => void) { menu = false; active = ''; callback(); }
  function keys(event: KeyboardEvent) {
    if (event.key === 'Escape') { menu = false; active = ''; menuButton.focus(); event.preventDefault(); }
    if (event.key === 'F10' && event.shiftKey) { event.preventDefault(); if (!menu) openMenu(); }
  }
  onDestroy(() => clearTimeout(closeTimer));
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions (Group delegates context shortcuts for its metric buttons.) -->
<div class="pill-shell" style={`height:${menu ? 328 : detail ? 208 : PILL_HEIGHT}px`} class:bottom-dock={settings.corner.startsWith('bottom')} class:sample={preview} bind:this={shell} onpointerleave={leave} onpointerenter={() => clearTimeout(closeTimer)} onfocusout={(event) => { if (!shell.contains(event.relatedTarget as Node)) { menu = false; leave(); } }} onkeydown={keys} oncontextmenu={(event) => { event.preventDefault(); if (!menu) openMenu(); }} role="group" aria-label={preview ? 'Sample floating HUD' : 'Floating Neon HUD'}>
  <div class="neon-pill" class:paused>
    <button class="pill-mascot" aria-label="Open meters and graphs" onclick={() => action(onExpand)} onpointerenter={() => hover('')}><span aria-hidden="true">✦</span><i></i><i></i></button>
    <div class="pill-metrics">
      {#each cells as cell}<button class="pill-cell" class:has-warning={cell.warning} style={`--stress:${Math.round(160 - cell.stress * 1.6)}`} aria-label={`${cell.name}: ${cell.value}. Open details`} onpointerenter={() => hover(cell.id)} onfocus={() => hover(cell.id)} onclick={() => action(onExpand)}><Icon name={cell.id} size={17}/><b>{cell.value}</b>{#if cell.warning}<span class="pill-badge">!</span>{/if}<span class="pill-track"><i style={`width:${Math.max(4, cell.stress)}%`}></i></span></button>{/each}
      {#each aiCells as cell}<button class="pill-cell ai-cell" class:has-warning={cell.warning} style={`--stress:${Math.round(160 - cell.stress * 1.6)}`} aria-label={`${cell.name}: ${cell.value} left${cell.question ? '. Question needs attention' : ''}`} onpointerenter={() => hover(cell.id)} onfocus={() => hover(cell.id)} onclick={() => action(() => onProvider(cell.id))}>{#key cell.attentionKey}<span class:question-jolt={cell.question}><Icon name={cell.id} size={17}/></span>{/key}<b>{cell.value}</b>{#if cell.question || cell.warning || cell.fast}<span class="pill-badge">{cell.question ? '?' : cell.fast ? 'ϟ' : '!'}</span>{/if}<span class="pill-track"><i style={`width:${Math.max(4, cell.stress)}%`}></i></span></button>{/each}
      {#if !cells.length && !aiCells.length}<span class="pill-empty">tiny chaos ✦</span>{/if}
    </div>
    <button class="pill-more" bind:this={menuButton} aria-label="Quick controls" aria-expanded={menu} onclick={openMenu} onpointerenter={() => hover('')}><span aria-hidden="true">•••</span></button>
  </div>
  {#if menu}<div class="pill-menu"><header>POCKET CHAOS <span>✦</span></header><button onclick={() => action(onExpand)}><Icon name="expand"/>Meters & graphs</button><button onclick={() => action(onSettings)}><Icon name="settings"/>Settings</button><button onclick={() => action(onPause)}><Icon name="pause"/>{paused ? 'Resume readings' : 'Pause readings'}</button><button onclick={() => action(onRefresh)}><Icon name="network"/>Refresh readings</button><div class="pill-menu-row"><button onclick={onTheme}>Theme ✦</button><button onclick={onMotion}>{settings.motion === 'quiet' ? 'Chaos!' : 'Quiet'}</button><button aria-pressed={settings.alwaysOnTop} onclick={onPin}><Icon name="pin"/>Pin</button></div><button onclick={() => action(onHide)}><Icon name="close"/>Hide to tray</button><small>Escape closes · right click opens</small></div>
  {:else if detail}<div class="pill-peek" role="status"><header><Icon name={detail.id} size={18}/>{detail.name}<span>↗</span></header><p>{detail.detail}</p>{#if preview}<small>SAMPLE DATA</small>{/if}</div>{/if}
</div>
