<script lang="ts">
  export let message: string = 'Loading...';
  export let size: 'xs' | 'sm' | 'md' | 'lg' = 'md';
</script>

<div class="flex flex-col items-center justify-center py-4" role="status" aria-live="polite">
  <span class="loading loading-spinner text-primary" class:loading-xs={size==='xs'} class:loading-sm={size==='sm'} class:loading-md={size==='md'} class:loading-lg={size==='lg'}></span>
  {#if message}
    <p class="mt-2 text-gray-300 text-sm">{message}</p>
  {/if}
</div>

