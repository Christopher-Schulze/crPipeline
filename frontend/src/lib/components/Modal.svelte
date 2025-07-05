<script lang="ts">
  import { createEventDispatcher, tick } from 'svelte';
  export let isOpen: boolean = false;
  export let title: string = 'Modal Title';

  const dispatch = createEventDispatcher();

  let panelElement: HTMLDivElement | null = null;
  $: if (isOpen && panelElement) {
    tick().then(() => panelElement?.focus());
  }

  function closeModal() {
    dispatch('close');
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape' && isOpen) closeModal();
  }
</script>

<svelte:window on:keydown={handleKeydown} />

{#if isOpen}
  <div class="modal modal-open" on:click|self={closeModal}>
    <div class="modal-box" bind:this={panelElement} tabindex="-1">
      {#if title}
        <h3 class="font-bold text-lg mb-4" id="modal-title-text">{title}</h3>
      {/if}
      <slot name="content" />
      {#if $$slots.footer}
        <div class="modal-action">
          <slot name="footer" />
        </div>
      {/if}
    </div>
  </div>
{/if}
