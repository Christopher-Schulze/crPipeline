<script lang="ts">
  import { derived } from 'svelte/store';
  import { loadingStore } from '$lib/utils/loadingStore';

  const isLoading = derived(loadingStore, (count) => count > 0);
</script>

{#if $isLoading}
  <div class="fixed top-0 left-0 right-0 h-1 z-[200]" role="progressbar" aria-valuetext="loading">
    <div class="h-full bg-accent animate-pulse rounded-r-full" style="width: 75%;"></div>
  </div>
{/if}
