<script lang="ts">
  import { errorStore, ErrorItem } from '$lib/utils/errorStore';
  import { fly } from 'svelte/transition';
</script>

<div class="fixed top-4 right-4 z-50 space-y-2" aria-live="assertive">
  {#each $errorStore as err (err.id)}
    <div
      class="bg-red-600 text-white px-4 py-2 rounded shadow flex items-start"
      transition:fly={{ x: 16, duration: 150 }}
    >
      <span class="flex-1">{err.message}</span>
      <button
        class="ml-2 font-bold focus:outline-none"
        on:click={() => errorStore.remove(err.id)}
        aria-label="close"
      >
        &times;
      </button>
    </div>
  {/each}
</div>
