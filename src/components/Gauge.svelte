<script lang="ts">
  export let value: number | null = null;
  export let label = '';
  export let suffix = '%';
  export let tone = 'normal';
  $: bounded = value === null || !Number.isFinite(value) ? null : Math.max(0, Math.min(100, value));
</script>
<div class="gauge {tone}" role="img" aria-label={`${label}: ${bounded === null ? 'unavailable' : bounded.toFixed(0) + suffix}`}>
  <svg viewBox="0 0 132 88" aria-hidden="true">
    <path class="gauge-track" d="M12 74 A54 54 0 0 1 120 74" pathLength="100"/>
    {#if bounded !== null}<path class="gauge-fill" d="M12 74 A54 54 0 0 1 120 74" pathLength="100" stroke-dasharray={`${bounded} 100`}/>{/if}
    <path class="gauge-ticks" d="M12 73h6m3-28 5 3m22-24 2 6m34-6-2 6m26 15-5 3m11 25h6"/>
    <text x="66" y="63" class="gauge-value" text-anchor="middle">{bounded === null ? '—' : bounded.toFixed(0)}<tspan class="gauge-unit">{bounded === null ? '' : suffix}</tspan></text>
    <text x="66" y="82" class="gauge-label" text-anchor="middle">{label}</text>
  </svg>
</div>
