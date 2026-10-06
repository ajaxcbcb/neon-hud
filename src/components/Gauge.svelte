<script context="module" lang="ts">
  let sequence = 0;
</script>
<script lang="ts">
  import { stressColor } from '../lib/stress';
  export let value: number | null = null;
  export let label = '';
  export let suffix = '%';
  export let tone = 'normal';
  export let inverse = false;
  const gradientId = `stress-gauge-${++sequence}`;
  $: bounded = value === null || !Number.isFinite(value) ? null : Math.max(0, Math.min(100, value));
</script>
<div class="gauge {tone}" style={`color:${tone === 'unavailable' ? 'var(--muted)' : stressColor(bounded, inverse)}`} role="img" aria-label={`${label}: ${bounded === null ? 'unavailable' : bounded.toFixed(0) + suffix}`}>
  <svg viewBox="0 0 132 88" aria-hidden="true">
    <defs><linearGradient id={gradientId} x1="12" y1="0" x2="120" y2="0" gradientUnits="userSpaceOnUse">
      {#if inverse}<stop offset="0%" stop-color={stressColor(bounded, true)}/><stop offset="100%" stop-color={stressColor(Math.max(0, (bounded ?? 0) - 15), true)}/>
      {:else}<stop offset="0%" stop-color={stressColor(0)}/><stop offset="79.4%" stop-color={stressColor(70)}/><stop offset="100%" stop-color={stressColor(100)}/>{/if}
    </linearGradient></defs>
    <path class="gauge-track" d="M12 74 A54 54 0 0 1 120 74" pathLength="100"/>
    {#if bounded !== null}<path class="gauge-fill" style={`stroke:${tone === 'unavailable' ? 'var(--muted)' : `url(#${gradientId})`}`} d="M12 74 A54 54 0 0 1 120 74" pathLength="100" stroke-dasharray={`${bounded} 100`}/>{/if}
    <path class="gauge-ticks" d="M12 73h6m3-28 5 3m22-24 2 6m34-6-2 6m26 15-5 3m11 25h6"/>
    <text x="66" y="63" class="gauge-value" text-anchor="middle">{bounded === null ? '—' : bounded.toFixed(0)}<tspan class="gauge-unit">{bounded === null ? '' : suffix}</tspan></text>
    <text x="66" y="82" class="gauge-label" text-anchor="middle">{label}</text>
  </svg>
</div>
