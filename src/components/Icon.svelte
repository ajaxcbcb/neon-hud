<script lang="ts">
  export let name = 'cpu';
  export let size = 18;
</script>

<svg class="metric-icon" data-icon={name} width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
  {#if name === 'cpu'}
    <rect x="5" y="5" width="14" height="14" rx="3"/><rect x="9" y="9" width="6" height="6" rx="1"/>
    <path d="M9 2v3m6-3v3M9 19v3m6-3v3M2 9h3m-3 6h3m14-6h3m-3 6h3"/>
  {:else if name === 'ram'}
    <rect x="2" y="6" width="20" height="11" rx="2"/><path d="M6 10v3m4-3v3m4-3v3m4-3v3M6 17v3m4-3v3m4-3v3m4-3v3"/>
  {:else if name === 'network'}
    <path d="M7 3v16m-4-4 4 4 4-4M17 21V5m-4 4 4-4 4 4"/>
  {:else if name === 'storage'}
    <path d="m5 3-3 11v5a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-5L19 3Z"/><path d="M2 14h20"/><circle cx="17" cy="17.5" r=".6" fill="currentColor"/><path d="M6 17.5h4"/>
  {:else if name === 'pause'}<path d="M8 4v16m8-16v16"/>
  {:else if name === 'pin'}<path d="m8 3 8 0-1 7 4 4H5l4-4ZM12 14v8"/>
  {:else if name === 'claude' || name === 'claude-code'}
    <image href="/brands/claude.svg" width="24" height="24"/>
  {:else if name === 'chatgpt' || name === 'codex'}
    <image href="/brands/openai.svg" width="24" height="24"/>
  {:else if name === 'settings'}
    <path d="m9 3-.8 2.5-2.5.5-2 3 1.3 2.3-.8 2.4 2 3 2.6-.2L11 19h3l1.8-2.5 2.6.2 2-3-.8-2.4L22 9l-2-3-2.5-.5L16 3Z"/><circle cx="12" cy="11" r="3"/>
  {:else if name === 'expand'}<path d="M8 3H3v5m13-5h5v5M3 16v5h5m13-5v5h-5"/>
  {:else if name === 'close'}<path d="m6 6 12 12M6 18 18 6"/>
  {:else if name === 'check'}<path d="m5 12 4 4L19 6"/>
  {:else if name === 'arrow'}<path d="M4 12h16m-6-6 6 6-6 6"/>
  {:else if name === 'question'}<path d="M9 8a3 3 0 0 1 6 0c0 2-3 2-3 5"/><circle cx="12" cy="17" r=".5"/><circle cx="12" cy="12" r="10"/>
  {:else if name === 'star'}<path d="m12 2 2.5 6.5 7 1-5.2 4.7 1.5 7-5.8-3.8-5.8 3.8 1.5-7L2.5 9.5l7-1Z"/>
  {:else if name === 'orbit'}<circle cx="12" cy="12" r="5"/><ellipse cx="12" cy="12" rx="11" ry="4" transform="rotate(-30 12 12)"/><circle cx="20" cy="7" r="1" fill="currentColor"/>
  {:else if name === 'spark'}<path d="m12 1 2.2 7.5L22 6l-4.5 6L23 16l-8-1.2L12 23l-2.5-8L2 18l4.5-6L1 7l8.2 1.5Z"/>
  {/if}
</svg>

<style>
  image { filter: invert(1); }
</style>
