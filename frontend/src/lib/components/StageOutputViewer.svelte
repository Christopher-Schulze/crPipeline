<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import Modal from './Modal.svelte';
  import Button from './Button.svelte';
  export let isOpen = false;
  export let title = '';
  export let content: string | null = null;
  export let isLoading = false;
  const dispatch = createEventDispatcher();
  function close() { dispatch('close'); }
</script>
<Modal {isOpen} {title} on:close={close} maxWidth="max-w-3xl">
  <div slot="content">
    {#if isLoading}
      <div class="flex justify-center items-center min-h-[200px]">
        <p class="text-gray-300 text-lg">Loading output content...</p>
      </div>
    {:else if content}
      <pre class="whitespace-pre-wrap break-all p-2 text-xs text-gray-200 bg-neutral-900/60 max-h-[70vh] overflow-y-auto rounded custom-scrollbar">{content}</pre>
    {:else}
      <div class="flex justify-center items-center min-h-[200px]">
        <p class="text-gray-400">No content to display or an error occurred.</p>
      </div>
    {/if}
  </div>
  <div slot="footer" class="flex justify-end">
    <Button variant="secondary" on:click={close}>Close Viewer</Button>
  </div>
</Modal>
