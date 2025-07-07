<script lang="ts">
  export let size: 'xs' | 'sm' | 'md' | 'lg' = 'md';
  $: sizeClass = size === 'xs' ? 'loading-xs' : size === 'sm' ? 'loading-sm' : size === 'lg' ? 'loading-lg' : '';
</script>
<div class="flex justify-center py-4" aria-busy="true">
  <span class="loading loading-spinner {sizeClass}"></span>
</div>
