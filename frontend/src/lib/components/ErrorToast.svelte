<script lang="ts">
  import { errorStore, ErrorItem } from '$lib/utils/errorStore';
  import { fly } from 'svelte/transition';
</script>

<div class="toast toast-top toast-end z-50" aria-live="assertive">
  {#each $errorStore as err (err.id)}
    <div
      class="alert alert-error shadow-lg flex"
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
