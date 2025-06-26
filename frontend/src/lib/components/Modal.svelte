<script lang="ts">
  import { createEventDispatcher, tick } from 'svelte'; // Added tick for focus management
  import { fade } from 'svelte/transition';
  import { quintOut } from 'svelte/easing'; // Or other preferred easing
  import GlassCard from './GlassCard.svelte';

  export let isOpen: boolean = false;
  export let title: string = 'Modal Title';
  export let maxWidth: string = 'max-w-md';

  const dispatch = createEventDispatcher();

  // Custom Svelte transition for fade & scale effect
  function fadeScale(node: HTMLElement, { delay = 0, duration = 200, startOpacity = 0, startScale = 0.95, easing = quintOut }) {
    const style = getComputedStyle(node);
    const originalOpacity = parseFloat(style.opacity);
    const originalTransform = style.transform === 'none' ? '' : style.transform;

    return {
      delay,
      duration,
      easing,
      css: (t: number) => { // t is eased, 0 -> 1 for IN, and 1 -> 0 for OUT
        const opacity = startOpacity + t * (originalOpacity - startOpacity);
        const scale = startScale + t * (1 - startScale);
        return `
          opacity: ${opacity};
          transform: ${originalTransform} scale(${scale});
        `;
      }
    };
  }

  // Focus management
  let panelElement: HTMLDivElement | null = null;
  $: if (isOpen && panelElement) {
    tick().then(() => {
      const firstFocusable = panelElement?.querySelector(
        'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])'
      ) as HTMLElement | null;
      firstFocusable?.focus();
    });
  }

  function closeModal() {
    dispatch('close');
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape' && isOpen) {
      closeModal();
    }
  }
</script>

<svelte:window on:keydown={handleKeydown} />

{#if isOpen}
  <!-- Backdrop -->
  <div
    transition:fade={{ duration: 200 }}
    class="fixed inset-0 bg-black/50 backdrop-blur-sm z-40 flex items-center justify-center p-4"
    on:click={closeModal}
  >
    <!-- Modal Panel -->
    <div
      bind:this={panelElement}
      role="dialog"
      aria-modal="true"
      aria-labelledby="modal-title-text"
      in:fadeScale={{ duration: 200, delay: 50, startScale: 0.95, startOpacity: 0 }}
      out:fadeScale={{ duration: 200, startScale: 0.95, startOpacity: 0 }}
      class="z-50 w-full {maxWidth} transform outline-none" <!-- Added transform and outline-none -->
      on:click|stopPropagation
    >
      <GlassCard
        padding="p-0"
        shadow="shadow-2xl"
        borderRadius="rounded-xl md:rounded-2xl"
        bgOpacity="!bg-neutral-800/85" <!-- Darker theme for modal panel -->
        borderStyle="!border-neutral-700/70"
        customClass="flex flex-col overflow-hidden" <!-- Ensure flex column for structure -->
      >
        <!-- Header -->
        {#if title}
          <div class="flex justify-between items-center p-4 border-b border-neutral-700/50 flex-shrink-0">
            <h2 id="modal-title-text" class="text-xl font-semibold text-gray-100">{title}</h2>
            <button
              on:click={closeModal}
              aria-label="Close modal"
              class="p-1 rounded-full hover:bg-white/10 text-gray-400 hover:text-gray-200 transition-colors"
            >
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-6 h-6">
                <path stroke-linecap="round" stroke-linejoin="round" d="M6 18L18 6M6 6l12 12" />
              </svg>
            </button>
          </div>
        {/if}

        <!-- Content Slot -->
        <div class="p-5 sm:p-6 flex-grow overflow-y-auto space-y-4 custom-scrollbar">
          <slot name="content" />
        </div>

        <!-- Footer Slot (Optional) -->
        {#if $$slots.footer}
          <div class="p-4 border-t border-neutral-700/50 flex-shrink-0">
            <slot name="footer" />
          </div>
        {/if}
      </GlassCard>
    </div>
  </div>
{/if}

<style lang="postcss">
  /* Ensure dialog is above other content if z-index issues arise with other fixed/absolute elements */
  /* Tailwind's z-40 and z-50 should generally be sufficient. */
</style>
