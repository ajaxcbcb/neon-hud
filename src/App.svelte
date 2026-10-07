<script lang="ts">

  import { onMount } from 'svelte';

  import { invoke } from '@tauri-apps/api/core';
  import { getVersion } from '@tauri-apps/api/app';
  import { check as checkUpdate } from '@tauri-apps/plugin-updater';
  import { UpdateController, type UpdateStatus } from './lib/updates';

  import { listen } from '@tauri-apps/api/event';

  import { getCurrentWindow, availableMonitors } from '@tauri-apps/api/window';

  import { LogicalSize, PhysicalPosition } from '@tauri-apps/api/dpi';

  import Hud from './components/Hud.svelte';
  import Pill from './components/Pill.svelte';
  import { windowSize, dockPosition, PILL_HEIGHT } from './lib/layout';
  import { idleAttempt, runConnection, connectionView } from './lib/connections';
  import { createRefreshQueue } from './lib/refresh';
  import { createPoller } from './lib/polling';

  import Icon from './components/Icon.svelte';

  import { DrainTracker, drainText } from './lib/drain';
  import { mischief } from './lib/mischief';
  import { ResourceGovernor } from './lib/resources';

  import { defaults, normalizeSettings, sampleSystem, sampleProviders, selectedNetwork, selectedDrives, gib, drivePercent, remaining, countdown, windowStatus } from './lib/model';

  import type { Settings, SystemSnapshot, ProviderSnapshot, Surface } from './lib/model';

  import { native, loadSettings, saveSettings, systemSnapshot, providerSnapshot, openLink } from './lib/backend';

  let settings: Settings = structuredClone(defaults);

  let loaded = false;
  let appVersion = '0.1.2';
  let updateStatus: UpdateStatus = { phase: native ? 'idle' : 'preview', message: native ? 'Updates are checked automatically.' : 'Updates are available in the installed app.' };
  const updater = new UpdateController(() => checkUpdate({ timeout: 10000 }), value => updateStatus = value);
  let lastUpdateCheck = 0;
  async function checkUpdates(automatic = false) {
    if (!native || (automatic && (!settings.autoUpdates || resources.mode !== 'normal'))) return;
    lastUpdateCheck = Date.now();
    await updater.check(automatic);
  }
  async function installUpdate() {
    await updater.install();
    if (updateStatus.phase === 'restart') {
      try { await invoke('restart_app'); }
      catch { notice = 'Update installed. Quit and reopen Neon HUD to load it.'; }
    }
  }

  let configure = true;

  let snapshot: SystemSnapshot | null = null;

  let providers: ProviderSnapshot = { usages: [], attention: [] };

  let now = Date.now() / 1000;

  let history: { time: number; down: number; up: number }[] = [];

  let monitors: string[] = ['Current display'];

  let busy = false;
  let paused = false;
  let systemBusy = false;
  let providerBusy = false;
  const queuedProviderRead = createRefreshQueue(readProviders);
  let codexAttempt = idleAttempt();
  let bridgeAttempt = idleAttempt();
  $: connecting = codexAttempt.phase === 'pending' || bridgeAttempt.phase === 'pending';
  let pillHeight = PILL_HEIGHT;
  let windowQueue: Promise<void> = Promise.resolve();
  let windowVisible = true;
  let documentHidden = document.hidden;
  let systemPoller: ReturnType<typeof createPoller> | undefined;
  let providerPoller: ReturnType<typeof createPoller> | undefined;
  $: {
    const enabled = loaded && !paused && windowVisible && !documentHidden;
    systemPoller?.setEnabled(enabled);
    providerPoller?.setEnabled(enabled);
  }
  const governor = new ResourceGovernor();
  $: resources = governor.update(snapshot, settings, now);
  $: effectiveMotion = resources.quiet ? 'quiet' : settings.motion;

  let error = '';

  let notice = '';

  let providerDetail: Surface | null = null;

  let demoQuestion = 0;
  let demoPressure = false;

  let seenAlerts = new Set<string>();

  let historyInterface = '';

  const drainTracker = new DrainTracker();

  let stop = false;

  let saveTimer: ReturnType<typeof setTimeout>;

  $: previewSnapshot = demoPressure ? { ...sampleSystem, sampledAt: now, gpus: sampleSystem.gpus?.map(gpu => ({ ...gpu, sampledAt: now })), cpu: 98, memory: { total: 32 * 1073741824, used: 31 * 1073741824, available: 1073741824 }, temperatures: [{ label: 'CPU Package (sample)', celsius: 95 }], drives: sampleSystem.drives.map((drive, index) => index ? drive : { ...drive, usedBytes: 502 * 1073741824, availableBytes: 10 * 1073741824 }) } : { ...sampleSystem, sampledAt: now, gpus: sampleSystem.gpus?.map(gpu => ({ ...gpu, sampledAt: now })) };
  $: previewProviders = { ...sampleProviders(now), attention: demoQuestion ? [{ id: `demo-${demoQuestion}`, surface: 'claude-code' as Surface, reason: 'Sample question', occurredAt: now, sessionId: 'preview' }] : [] };

  $: if (loaded) { settings; scheduleSave(); }

  function scheduleSave() {

    clearTimeout(saveTimer);

    saveTimer = setTimeout(() => saveSettings(normalizeSettings(settings)).catch(() => { error = 'Settings could not be saved. Check write access to the app data folder.'; }), 350);

  }

  async function applyWindow() {
    if (!native) return;
    windowQueue = windowQueue.catch(() => {}).then(resizeWindow);
    return windowQueue;
  }
  async function resizeWindow() {
    const win = getCurrentWindow();
    const dimensions = windowSize(settings, configure, !!providerDetail, pillHeight);
    await win.setAlwaysOnTop(configure ? false : settings.alwaysOnTop);
    await win.setSkipTaskbar(!configure);
    await win.setResizable(configure);
    await win.setMinSize(new LogicalSize(dimensions.minWidth, dimensions.minHeight));
    await win.setSize(new LogicalSize(dimensions.width, dimensions.height));
    if (configure) { await win.center(); return; }

    const displays = await availableMonitors();

    const monitor = displays[settings.monitor] || displays[0];

    if (!monitor) return;

    const area = monitor.workArea;

    const size = await win.outerSize();

    const position = dockPosition(settings.corner, { x: area.position.x, y: area.position.y, width: area.size.width, height: area.size.height }, size, monitor.scaleFactor, settings.size === 'compact' && !providerDetail, settings.textScale);
    await win.setPosition(new PhysicalPosition(position.x, position.y));
  }
  function resizePill(height: number) { if (pillHeight === height) return; pillHeight = height; if (!configure && !providerDetail && settings.size === 'compact') void applyWindow().catch(() => error = 'The HUD window could not be resized. Reopen it from the tray.'); }

  async function openConfiguration() { configure = true; providerDetail = null; settings.step = 1; await applyWindow(); }

  async function finish() {

    busy = true; error = '';

    try {

      settings = normalizeSettings({ ...settings, completed: true, step: 1 });

      if (native) await invoke('set_preferences', { launchAtLogin: settings.launchAtLogin, notifications: settings.notifications });

      await saveSettings(settings);

      configure = false;

      providerDetail = null;

      await applyWindow();

    } catch { error = 'Configuration could not be applied. Check app permissions and try again.'; }

    finally { busy = false; }

  }

  async function refreshSystem(force = false) {

    if (document.hidden || !windowVisible || stop || (paused && !force) || systemBusy) return;
    systemBusy = true;

    try {

      snapshot = await systemSnapshot(resources.mode, settings.resources.samplingMs);

      const network = selectedNetwork(snapshot, settings.interface);

      if (network) {

        if (historyInterface !== network.name) { history = []; historyInterface = network.name; }

        if (history.at(-1)?.time !== snapshot!.sampledAt) history = [...history.filter(point => snapshot && point.time >= snapshot.sampledAt - 60), { time: snapshot!.sampledAt, down: network.down, up: network.up }].slice(-240);

      } else history = [];

    } catch { snapshot = null; }
    finally { systemBusy = false; }

  }

  async function refreshProviders(force = false) {
    if (stop || (!force && (document.hidden || !windowVisible || paused))) return;
    await queuedProviderRead(force);
  }
  async function readProviders(force: boolean) {
    providerBusy = true;

    try {

      providers = await providerSnapshot();

      drainTracker.observe(providers.usages, Date.now() / 1000);

      const fresh = providers.attention.filter(a => !seenAlerts.has(a.id));

      if (fresh.length && settings.notifications && native) await invoke('notify_attention', { surfaces: [...new Set(fresh.map(a => a.surface))] });

      seenAlerts = new Set(providers.attention.map(a => a.id));

    } catch { error = 'AI sources could not be refreshed. System monitoring can continue.'; if (force) throw new Error('Source refresh failed'); }
    finally { providerBusy = false; }

  }

  async function connectCodex() {
    if (connecting) return;
    error = ''; notice = '';
    await runConnection(async () => {
      if (!native) return 'Install Neon HUD to connect the local Codex CLI.';
      settings.codexEnabled = true;
      await saveSettings(settings);
      const message = await invoke<string>('connect_codex');
      await refreshProviders(true);
      return message;
    }, value => codexAttempt = value, 'Connecting to Codex… finish sign-in in your browser if prompted.', 'Codex could not connect. Install the Codex CLI and retry; an existing sign-in is reused.');
  }

  async function connectClaude() {
    if (connecting) return;
    error = ''; notice = '';
    await runConnection(async () => {
      if (!native) return 'Install Neon HUD to enable the Claude Code bridge.';
      const message = await invoke<string>('install_claude_bridge');
      await refreshProviders(true);
      return message;
    }, value => bridgeAttempt = value, 'Enabling the Claude Code bridge…', 'The Claude bridge could not be enabled. Check access to Claude Code settings and retry.');
  }

  async function disconnectCodex() {
    if (connecting) return;
    await runConnection(async () => {
      settings.codexEnabled = false;
      await saveSettings(settings);
      if (native) await invoke('disconnect_codex');
      await refreshProviders(true);
      return 'Codex disconnected.';
    }, value => codexAttempt = value, 'Disconnecting Codex…', 'Codex could not disconnect. Retry or restart Neon HUD.');
  }

  async function removeClaudeBridge() {
    if (connecting) return;
    await runConnection(async () => {
      if (!native) return 'Install Neon HUD to manage the local Claude bridge.';
      const message = await invoke<string>('remove_claude_bridge');
      await refreshProviders(true);
      return message;
    }, value => bridgeAttempt = value, 'Removing the Claude Code bridge…', 'The Claude bridge could not be removed. See the connection guide for recovery.');
  }

  function toggleDrive(id: string, checked: boolean) {
    const available = snapshot?.drives || [];
    const ids = new Set(settings.storageDriveIds.length ? settings.storageDriveIds : available.map(d => d.id));
    if (checked) ids.add(id); else ids.delete(id);
    if (!ids.size) { notice = 'Keep at least one drive selected, or turn off the storage metric.'; return false; }
    settings.storageDriveIds = [...ids];
    return true;
  }
  function cycleTheme() {
    const themes: Settings['theme'][] = ['circuit', 'cyberpunk', 'aurora'];
    settings.theme = themes[(themes.indexOf(settings.theme) + 1) % themes.length];
  }
  async function togglePin() { settings.alwaysOnTop = !settings.alwaysOnTop; if (native) await getCurrentWindow().setAlwaysOnTop(settings.alwaysOnTop); }
  function toggleMotion() { settings.motion = settings.motion === 'quiet' ? 'chaotic' : 'quiet'; }
  function togglePause() { paused = !paused; }
  function refreshNow() { void refreshSystem(true); void refreshProviders(true); }

  async function toggleSize() { settings.size = settings.size === 'compact' ? 'expanded' : 'compact'; await applyWindow(); }

  function setVisible(visible: boolean) { windowVisible = visible; window.dispatchEvent(new CustomEvent('neon-visibility', { detail: visible })); }
  async function hide() { if (native) { await getCurrentWindow().hide(); setVisible(false); } else notice = 'The desktop app hides to the system tray or menu bar.'; }

  async function showProvider(surface: string) {

    providerDetail = surface as Surface;

    if (!configure) await applyWindow();

  }

  async function closeProvider() { providerDetail = null; await applyWindow(); }

  onMount(() => {

    const cleanups: (() => void)[] = [];
    systemPoller = createPoller(() => refreshSystem(), () => resources.systemSeconds * 1000);
    providerPoller = createPoller(() => refreshProviders(), () => resources.providerSeconds * 1000);

    (async () => {

      settings = await loadSettings();

      configure = !settings.completed;

      loaded = true;

      if (native) {
        appVersion = await getVersion();

        monitors = (await availableMonitors()).map((m, i) => m.name || `Display ${i + 1}`);

        cleanups.push(await listen('open-settings', openConfiguration));
        cleanups.push(await listen<boolean>('hud-visible', event => setVisible(event.payload)));

        cleanups.push(await getCurrentWindow().onCloseRequested(async event => { event.preventDefault(); await hide(); }));

        if (settings.codexEnabled) invoke('connect_codex', { login: false }).catch(() => {});

        await applyWindow();

        await getCurrentWindow().show();

      }

      void checkUpdates(true);

    })().catch(() => { loaded = true; error = 'Some startup settings could not be loaded. You can continue configuring the HUD.'; });

    const timer = setInterval(() => {
      if (document.hidden || !windowVisible || !loaded) return;
      now = Date.now() / 1000;
    }, 1000);
    // Works from the tray too; defer automatic network work during pressure.
    const updatesTimer = setInterval(() => {
      if (loaded && Date.now() - lastUpdateCheck >= 6 * 60 * 60 * 1000) void checkUpdates(true);
    }, 60000);

    const recover = setInterval(async () => {

      if (!native || configure || document.hidden || !windowVisible) return;

      try {

        const displays = await availableMonitors();

        const win = getCurrentWindow();

        const p = await win.outerPosition();

        const visible = displays.some(m => p.x >= m.workArea.position.x && p.y >= m.workArea.position.y && p.x < m.workArea.position.x + m.workArea.size.width - 40 && p.y < m.workArea.position.y + m.workArea.size.height - 40);

        if (!visible) await applyWindow();

      } catch { /* A display may disappear during enumeration. Retry next interval. */ }

    }, 15000);

    const resume = () => { documentHidden = document.hidden; };

    document.addEventListener('visibilitychange', resume);

    return () => { stop = true; systemPoller?.destroy(); providerPoller?.destroy(); clearTimeout(saveTimer); [timer, recover, updatesTimer].forEach(clearInterval); void updater.dispose(); cleanups.forEach(fn => fn()); document.removeEventListener('visibilitychange', resume); };

  });

</script>

<main use:mischief={{ mode: effectiveMotion, reduced: settings.reducedMotion }} data-theme={settings.theme} data-motion={effectiveMotion} data-resource-mode={resources.mode} class:pill-window={loaded && !configure && !providerDetail && settings.size === 'compact'} class:reduced-motion={settings.reducedMotion} class:configuration={configure} style={`--panel-opacity:${settings.opacity};--text-scale:${settings.textScale}`}>

  {#if !loaded}<div class="loading">Starting Neon HUD…</div>

  {:else if configure}

    <header class="setup-header" data-tauri-drag-region><div class="hud-brand"><span class="brand-mark">N</span><span>NEON <b>HUD</b></span></div><span class="setup-tag">PERSONAL COMMAND CENTER</span><button class="icon-button" aria-label="Hide configuration" onclick={hide}><Icon name="close"/></button></header>

    <nav class="steps" aria-label="Configuration progress">

      {#each ['Appearance', 'Connections', 'Preferences'] as label, index}<button class:active={settings.step === index + 1} aria-current={settings.step === index + 1 ? 'step' : undefined} onclick={() => settings.step = index + 1}><span>{index + 1}</span>{label}</button>{/each}

    </nav>

    {#key settings.step}<div class="setup-content">

      {#if settings.step === 1}

        <div class="page-intro appearance-intro"><span class="eyebrow">STEP 01 / TUNE YOUR ORBIT</span><h1>Tiny HUD.<br/><em>Big energy<span>✦</span></em></h1><p>Your machine. Your AI. Your tiny neon playground.</p><span class="orbit-sticker" aria-hidden="true">SMALL<br/>BUT LOUD ↗</span></div>

        <div class="appearance-layout"><div class="appearance-controls">

          <fieldset class="theme-picker"><legend>Choose your atmosphere</legend>

            {#each [{ id: 'circuit', name: 'Neon Circuit', caption: 'Acid lime + candy pink. Electric.' }, { id: 'cyberpunk', name: 'Cyberpunk Night', caption: 'Hot pink + yellow. After hours.' }, { id: 'aurora', name: 'Aurora', caption: 'Mint + lavender. A softer glow.' }] as theme}

              <button class="theme-option {theme.id}" class:selected={settings.theme === theme.id} aria-pressed={settings.theme === theme.id} onclick={() => settings.theme = theme.id as Settings['theme']}><span class="theme-art"><Icon name={theme.id === 'circuit' ? 'cpu' : theme.id === 'cyberpunk' ? 'star' : 'orbit'} size={29}/></span><span class="theme-swatches"><i></i><i></i><i></i></span><span><b>{theme.name}</b><small>{theme.caption}</small></span><span class="theme-check">{#if settings.theme === theme.id}<Icon name="check" size={16}/>{/if}</span></button>

            {/each}

          </fieldset>

          <fieldset class="motion-picker"><legend>How much mischief?</legend><div class="motion-options">{#each [{id:'quiet',label:'Quiet',icon:'orbit'},{id:'playful',label:'Playful',icon:'star'},{id:'chaotic',label:'Chaos!',icon:'spark'}] as mode}<button aria-pressed={settings.motion === mode.id} class:chosen={settings.motion === mode.id} onclick={() => settings.motion = mode.id as Settings['motion']}><Icon name={mode.icon} size={19}/><span>{mode.label}</span></button>{/each}</div></fieldset><label for="size">Size<select id="size" bind:value={settings.size}><option value="compact">Pill · 280 × 56</option><option value="expanded">Expanded · 680 × 500</option></select></label><details class="advanced-settings"><summary>Placement & readability</summary><label for="corner">Position<select id="corner" bind:value={settings.corner}><option value="middle-right">Right side</option><option value="middle-left">Left side</option><option value="bottom-right">Bottom right</option><option value="bottom-left">Bottom left</option><option value="top-right">Top right</option><option value="top-left">Top left</option></select></label>

          <label for="monitor">Display<select id="monitor" bind:value={settings.monitor}>{#each monitors as monitor, index}<option value={index}>{monitor}</option>{/each}</select></label>

          <div class="form-grid"><label for="opacity">Panel opacity <b>{(settings.opacity * 100).toFixed(0)}%</b><input id="opacity" type="range" min="0.85" max="1" step="0.01" bind:value={settings.opacity}/></label><label for="text-scale">Text size <b>{(settings.textScale * 100).toFixed(0)}%</b><input id="text-scale" type="range" min="0.9" max="1.15" step="0.05" bind:value={settings.textScale}/></label></div>

          <label class="toggle" for="always-top"><input id="always-top" type="checkbox" bind:checked={settings.alwaysOnTop}/><span>Keep HUD above other windows</span></label></details>

        </div><div class="preview-area"><div class="preview-heading"><span>LIVE PREVIEW</span><span class="sample-pill">SAMPLE DATA</span></div><div class="preview-stage"><Pill settings={{...settings, size: 'compact'}} snapshot={previewSnapshot} providers={previewProviders} {now} preview onTheme={cycleTheme} onMotion={toggleMotion}/></div><p class="preview-caption">The preview shows sample readings. Your real metrics appear after setup.</p><div class="preview-feature"><Icon name="cpu"/><span>System metrics stay on your device.</span></div><div class="preview-feature"><Icon name="question"/><span>AI icons signal when a connected session needs you.</span></div><button class="text-button demo-question" onclick={() => demoQuestion += 1}><Icon name="question" size={14}/> Test a sample question shake</button><button class="text-button demo-question" aria-pressed={demoPressure} onclick={() => demoPressure = !demoPressure}><Icon name="cpu" size={14}/>{demoPressure ? 'Stop sample pressure' : 'Test sample pressure badges'}</button></div></div>

      {:else if settings.step === 2}

        <div class="page-intro"><span class="eyebrow">STEP 02 / YOUR SOURCES</span><h1>Connect your AI<span>.</span></h1><p>Your providers manage sign-in. Neon HUD receives supported usage readings, never your passwords.</p></div>

        <div class="connection-grid">{#each ['chatgpt', 'codex', 'claude', 'claude-code'] as surface}

          {@const usage = providers.usages.find(p => p.surface === surface)}
          {@const attempt = surface === 'codex' ? codexAttempt : surface === 'claude-code' ? bridgeAttempt : idleAttempt()}
          {@const status = connectionView(surface as Surface, usage, attempt, !!providers.claudeBridgeEnabled)}

          <article class="connection-card" aria-busy={attempt.phase === 'pending'}><div class="connection-title"><Icon name={surface} size={26}/><h2>{surface === 'chatgpt' ? 'ChatGPT' : surface === 'codex' ? 'Codex' : surface === 'claude' ? 'Claude' : 'Claude Code'}</h2><span class="connection-state" data-tone={status.tone}>{status.label}</span></div>

            <p>{surface === 'chatgpt' ? 'Chat quotas are separate from Codex. No supported automatic chat quota source is available in this version.' : surface === 'codex' ? 'Read the account allowance through the official Codex app-server. Existing Codex sign-in can be reused.' : surface === 'claude' ? 'Claude and Claude Code share account limits when signed in to the same account. Readings come from the Code bridge.' : 'Add a local statusline and question observer. Existing settings are backed up and preserved.'}</p>

            <div class="source-note" data-tone={status.tone} role="status" aria-live="polite">{#if attempt.phase === 'pending'}<span class="connection-progress" aria-hidden="true"></span>{/if}{status.message}{#if usage?.fetchedAt && attempt.phase !== 'pending'} · Updated {new Date(usage.fetchedAt * 1000).toLocaleTimeString()}{/if}</div>

            <div class="connection-actions">{#if surface === 'codex'}<button class="secondary" disabled={connecting} onclick={connectCodex}>{codexAttempt.phase === 'pending' ? 'Working…' : status.tone === 'connected' ? 'Reconnect Codex ✓' : codexAttempt.phase === 'error' ? 'Retry connection' : 'Connect Codex'}</button>{#if settings.codexEnabled}<button class="text-button" disabled={connecting} onclick={disconnectCodex}>Disconnect</button>{/if}<button class="text-button" onclick={() => openLink('codex-install')}>Install CLI ↗</button>

            {:else if surface === 'claude-code'}<button class="secondary" disabled={connecting} onclick={connectClaude}>{bridgeAttempt.phase === 'pending' ? 'Working…' : providers.claudeBridgeEnabled ? 'Bridge enabled ✓' : bridgeAttempt.phase === 'error' ? 'Retry bridge' : 'Enable bridge'}</button><button class="text-button" disabled={connecting} onclick={removeClaudeBridge}>Remove bridge</button><button class="text-button" onclick={() => openLink(surface)}>Install CLI ↗</button>

            {:else}<button class="secondary" onclick={() => openLink(surface)}>Open provider ↗</button>{/if}</div>

          </article>

        {/each}</div><p class="connection-footnote">Optional connections. Claude readings become available after an assistant response. Question detection applies to connected Claude Code sessions; existing Codex desktop conversations are not automatically observed.</p>

      {:else}

        <div class="page-intro"><span class="eyebrow">STEP 03 / YOUR RHYTHM</span><h1>Keep it useful<span>.</span></h1><p>Pick what stays visible and when your HUD should get your attention.</p></div>

        <div class="preferences-grid"><section class="preferences-panel"><h2>Visible metrics</h2>{#each [{key:'cpu',label:'CPU activity'},{key:'gpu',label:'GPU activity'},{key:'ram',label:'Memory usage'},{key:'network',label:'Network traffic'},{key:'storage',label:'Drive capacity'},{key:'ai',label:'AI allowance'}] as metric}<label class="toggle" for={`metric-${metric.key}`}><input id={`metric-${metric.key}`} type="checkbox" bind:checked={settings.metrics[metric.key as keyof Settings['metrics']]}/><Icon name={metric.key === 'ai' ? 'codex' : metric.key}/><span>{metric.label}</span></label>{/each}<label for="interface">Network interface<select id="interface" bind:value={settings.interface}><option value="auto">Automatic · default route</option>{#each snapshot?.networks || [] as network}<option value={network.name}>{network.name}</option>{/each}</select></label><p class="field-hint">One interface at a time avoids counting VPN traffic twice. CPU, memory and network follow your sampling choice. Smart resource mode slows checks under pressure.</p><label for="gpu-choice">GPU<select id="gpu-choice" bind:value={settings.gpuId}><option value="auto">Automatic · busiest reported GPU</option>{#each snapshot?.gpus || [] as gpu}<option value={gpu.id}>{gpu.name}</option>{/each}{#if settings.gpuId !== 'auto' && !snapshot?.gpus?.some(gpu => gpu.id === settings.gpuId)}<option value={settings.gpuId}>Saved GPU · disconnected</option>{/if}</select></label><p class="field-hint">Windows reports activity and dedicated memory where available. Mac reports GPU identity; unsupported readings stay unavailable.</p><label for="sampling">System sampling<select id="sampling" bind:value={settings.resources.samplingMs}><option value={250}>Fast · 250 ms</option><option value={500}>Balanced · 500 ms</option><option value={1000}>Light · 1 second</option><option value={2000}>Quiet · 2 seconds</option></select></label><p class="field-hint">Meters animate at your display's refresh rate between readings. GPU checks run every second, sensors every 2 seconds, and AI every 5 seconds. Hidden monitoring pauses; pressure reduces these rates.</p><fieldset class="drive-picker"><legend><Icon name="storage" size={14}/> Choose drives</legend>
          <label class="toggle"><input type="checkbox" checked={!settings.storageDriveIds.length} onchange={(event) => settings.storageDriveIds = event.currentTarget.checked ? [] : (snapshot?.drives || []).map(d => d.id)}/><span>All detected drives · include new ones</span></label>
          {#each snapshot?.drives || [] as drive}<label class="drive-choice"><input type="checkbox" checked={!settings.storageDriveIds.length || settings.storageDriveIds.includes(drive.id)} onchange={(event) => { if (!toggleDrive(drive.id, event.currentTarget.checked)) event.currentTarget.checked = true; }}/><span><b>{drive.name || drive.mount}</b><small>{drive.mount} · {gib(drive.availableBytes)} GiB free / {gib(drive.totalBytes)} GiB</small></span><strong>{drivePercent(drive)?.toFixed(0) ?? '—'}%</strong></label>{/each}
          {#each settings.storageDriveIds.filter(id => !snapshot?.drives.some(d => d.id === id)) as id}<div class="drive-missing"><span>Disconnected drive · selection saved</span><button class="text-button" onclick={() => settings.storageDriveIds = settings.storageDriveIds.filter(saved => saved !== id)}>Forget</button></div>{/each}
          {#if !snapshot?.drives.length}<p class="field-hint">The desktop app detects mounted drives. Capacity refreshes about every 30 seconds.</p>{/if}
          <p class="field-hint">Hover a drive for exact used/free/total measurements. Capacity is per volume; shared storage pools can appear on more than one volume.</p>
        </fieldset></section>

        <section class="preferences-panel"><h2>Attention & behaviour</h2><div class="form-grid"><label for="warning">Warn at % remaining<input id="warning" type="number" min="1" max="100" bind:value={settings.warning}/></label><label for="critical">Critical at % remaining<input id="critical" type="number" min="0" max={settings.warning} bind:value={settings.critical}/></label></div><fieldset class="pressure-settings"><legend>Performance pressure thresholds</legend><div class="form-grid"><label for="cpu-pressure">CPU load %<input id="cpu-pressure" type="number" min="50" max="100" bind:value={settings.performance.cpuPercent}/></label><label for="gpu-pressure">GPU load %<input id="gpu-pressure" type="number" min="50" max="100" bind:value={settings.performance.gpuPercent}/></label><label for="ram-pressure">RAM used %<input id="ram-pressure" type="number" min="50" max="100" bind:value={settings.performance.memoryPercent}/></label><label for="temp-pressure">Temperature °C<input id="temp-pressure" type="number" min="40" max="120" bind:value={settings.performance.temperatureCelsius}/></label><label for="drive-pressure">Drive used %<input id="drive-pressure" type="number" min="50" max="100" bind:value={settings.performance.storagePercent}/></label></div><p class="field-hint">! flags sustained CPU/GPU/RAM pressure (10 seconds), high reported temperatures, or low drive space. Hover for the cause. These signals suggest possible slowdown; thermal throttling is not measured. Temperature is unavailable when no sensor is reported.</p></fieldset><label class="toggle" for="adaptive-resources"><input id="adaptive-resources" type="checkbox" bind:checked={settings.resources.adaptive}/><span>Smart resource mode · adapt to system pressure</span></label><p class="field-hint">CPU/GPU/RAM pressure or high reported temperature slows system checks to 4–8 seconds, AI checks to 10–15 seconds, and quiets motion. Recovery needs 20 healthy seconds per step. Drive and route checks are cached longer. Your motion choice returns automatically.</p><label class="toggle" for="reduce-motion"><input id="reduce-motion" type="checkbox" bind:checked={settings.reducedMotion}/><span>Reduce motion · use a static question badge</span></label><label class="toggle" for="notifications"><input id="notifications" type="checkbox" bind:checked={settings.notifications}/><span>Desktop notifications for questions</span></label><label class="toggle" for="autostart"><input id="autostart" type="checkbox" bind:checked={settings.launchAtLogin}/><span>Launch Neon HUD at login</span></label><p class="field-hint">No automatic approvals. A brief shake signals a real question or permission request. System reduced-motion preferences are also respected.</p></section>
        <section class="preferences-panel update-panel"><h2>App updates <small>v{appVersion}</small></h2>
          <label class="toggle" for="auto-updates"><input id="auto-updates" type="checkbox" bind:checked={settings.autoUpdates}/><span>Automatically check and download updates</span></label>
          <p class="field-hint">Checks GitHub on startup and every 6 hours. Downloads wait during high system pressure. Installation and restart require your click.</p>
          <p class="update-status" role="status" aria-live="polite">{updateStatus.message}</p>
          {#if updateStatus.phase === 'downloading'}<progress aria-label="Update download" max={updateStatus.total || undefined} value={updateStatus.total ? updateStatus.received || 0 : undefined}></progress><small>{((updateStatus.received || 0) / 1048576).toFixed(1)} MiB{updateStatus.total ? ` / ${(updateStatus.total / 1048576).toFixed(1)} MiB` : ' downloaded'}</small>{/if}
          <div class="update-actions">
            <button class="secondary" disabled={!native || ['checking','downloading','installing'].includes(updateStatus.phase)} onclick={() => checkUpdates()}>Check updates</button>
            {#if updateStatus.phase === 'available' || (updateStatus.phase === 'error' && updater.canDownload)}<button class="primary" onclick={() => updater.download()}>Download update</button>{/if}
            {#if updateStatus.phase === 'ready' || (updateStatus.phase === 'error' && updater.canInstall)}<button class="primary" onclick={installUpdate}>Install &amp; restart</button>{/if}
            {#if updateStatus.phase === 'restart'}<button class="primary" onclick={() => invoke('restart_app')}>Restart Neon HUD</button>{/if}
          </div>
        </section></div>

      {/if}

      {#if error}<div class="feedback error" role="alert">{error}</div>{/if}

      {#if notice}<div class="feedback" role="status">{notice}</div>{/if}

    </div>{/key}

    <footer class="setup-footer"><span>LOCAL FIRST <i>·</i> SMALL BY DESIGN</span><div>{#if settings.step > 1}<button class="secondary" onclick={() => settings.step -= 1}>Back</button>{/if}{#if settings.completed}<button class="text-button" onclick={finish}>Return to HUD</button>{/if}{#if settings.step < 3}<button class="primary" onclick={() => settings.step += 1}>{settings.step === 1 ? 'Next: Connect sources' : 'Next: Preferences'}<Icon name="arrow" size={17}/></button>{:else}<button class="primary" disabled={busy} onclick={finish}>Open my HUD<Icon name="arrow" size={17}/></button>{/if}</div></footer>

  {:else if providerDetail}

    <section class="provider-dialog"><header><h1>{providerDetail === 'codex' ? 'ChatGPT & Codex' : 'Claude'} usage</h1><button class="icon-button" aria-label="Close usage details" onclick={closeProvider}><Icon name="close"/></button></header>

      {#each providers.usages.filter(u => providerDetail === 'codex' ? ['chatgpt','codex'].includes(u.surface) : ['claude','claude-code'].includes(u.surface)) as usage}<article><h2>{usage.surface === 'chatgpt' ? 'ChatGPT chat' : usage.surface === 'codex' ? 'Codex account' : usage.surface === 'claude' ? 'Claude shared account' : 'Claude Code bridge'}</h2><p>{usage.message}</p>{#each usage.windows as quota}<div class="quota-line"><span>{quota.label}</span><div class="meter"><span style={`width:${remaining(quota.usedPercent)}%`}></span></div><b>{remaining(quota.usedPercent).toFixed(0)}% left</b><small>{countdown(quota.resetsAt,now)} · {windowStatus(quota,usage.fetchedAt,now)}</small></div><p class="drain-detail">{drainText(drainTracker.reading(usage, quota, now))} · time-weighted average, up to 30 min</p>{/each}{#if usage.tokenUsage}{@const tokenRate = drainTracker.tokenReading(usage, now)}<p class="token-reading">{usage.tokenUsage.total.toLocaleString()} reported tokens this session · {tokenRate ? `${tokenRate.perMinute.toFixed(0)} tokens/min average over ${(tokenRate.observedSeconds / 60).toFixed(0)} min${tokenRate.fast ? " · Fast token drain" : ""}` : "Collecting ≥2 min of token readings"}</p>{/if}<small class="source-note">Source: {usage.source} · {usage.fetchedAt ? `Last reading ${new Date(usage.fetchedAt * 1000).toLocaleString()}` : 'No reading'}</small></article>{/each}

      {#each providers.attention.filter(a => providerDetail === 'codex' ? a.surface === 'codex' : a.surface === 'claude-code' || a.surface === 'claude') as attention}<div class="feedback"><Icon name="question"/> {attention.reason} · Return to your {attention.surface} session to answer. <button class="text-button" onclick={async () => { if (native) await invoke('dismiss_attention', { id: attention.id }); await refreshProviders(); }}>Dismiss badge</button></div>{/each}

      <button class="secondary" onclick={() => openLink(providerDetail || 'chatgpt')}>Open provider usage ↗</button>

      {#if error}<div class="feedback error" role="alert">{error}</div>{/if}

    </section>

  {:else}<div class="hud-window" style={`zoom:${settings.textScale}`}>{#if settings.size === 'compact'}<Pill reduced={settings.reducedMotion || effectiveMotion === 'quiet'} {settings} {snapshot} {providers} {now} {drainTracker} onSpace={resizePill} onSettings={openConfiguration} onExpand={toggleSize} onHide={hide} onProvider={showProvider} {paused} onPause={togglePause} onRefresh={refreshNow} onTheme={cycleTheme} onMotion={toggleMotion} onPin={togglePin}/>{:else}<Hud {settings} {snapshot} {providers} {now} {history} {drainTracker} {resources} onSettings={openConfiguration} onExpand={toggleSize} onHide={hide} onProvider={showProvider} {paused} onPause={togglePause} onRefresh={refreshNow} onTheme={cycleTheme} onMotion={toggleMotion} onPin={togglePin}/>{/if}</div>{/if}

</main>
