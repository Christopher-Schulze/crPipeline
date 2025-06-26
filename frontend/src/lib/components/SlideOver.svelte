<script lang="ts">
  import { createEventDispatcher, tick } from 'svelte';
  import { fly, fade } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing'; // Import a standard easing function
  import GlassCard from './GlassCard.svelte';

  export let isOpen: boolean = false;
  export let title: string = 'Panel Title';
  export let position: 'left' | 'right' = 'right';
  export let maxWidth: string = 'max-w-md sm:max-w-lg'; // Responsive max width

  const dispatch = createEventDispatcher();

  function closePanel() {
    dispatch('close');
  }

  // Ensure focus is managed when panel opens/closes for accessibility
  let panelElement: HTMLDivElement | null = null;
  $: if (isOpen && panelElement) {
    tick().then(() => { // Wait for DOM update
      const focusable = panelElement?.querySelector(
        'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])'
      ) as HTMLElement | null;
      focusable?.focus();
    });
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape' && isOpen) {
      closePanel();
    }
    // Basic focus trapping (can be enhanced with a more robust solution if needed)
    if (event.key === 'Tab' && isOpen && panelElement) {
      const focusableElements = Array.from(
        panelElement.querySelectorAll('button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])')
      ) as HTMLElement[];
      const firstElement = focusableElements[0];
      const lastElement = focusableElements[focusableElements.length - 1];

      if (event.shiftKey) { // Shift + Tab
        if (document.activeElement === firstElement) {
          lastElement.focus();
          event.preventDefault();
        }
      } else { // Tab
        if (document.activeElement === lastElement) {
          firstElement.focus();
          event.preventDefault();
        }
      }
    }
  }

  // Fly transition parameters based on position
  let flyParams = {};
  $: {
    if (position === 'right') {
      flyParams = { x: 300, duration: 300, easing: cubicOut };
    } else {
      flyParams = { x: -300, duration: 300, easing: cubicOut };
    }
  }

  // Panel position classes
  let panelPositionClasses = '';
  $: panelPositionClasses = position === 'right' ? 'right-0' : 'left-0';

</script>

<svelte:window on:keydown={handleKeydown} />

{#if isOpen}
  <!-- Backdrop -->
  <div
    transition:fade={{ duration: 200 }}
    class="fixed inset-0 bg-black/50 backdrop-blur-sm z-30"
    on:click={closePanel}
    aria-hidden="true"
  ></div>

  <!-- SlideOver Panel -->
  <div
    bind:this={panelElement}
    role="dialog"
    aria-modal="true"
    aria-labelledby="slideover-title-text"
    transition:fly={flyParams}
    class={`fixed top-0 bottom-0 ${panelPositionClasses} w-full ${maxWidth} z-40 flex flex-col outline-none`}
    on:click|stopPropagation
  >
    <GlassCard
        bgOpacity="!bg-neutral-800/85" <!-- Slightly more opacity for side panel -->
        customClass="h-full flex flex-col !shadow-2xl !border-neutral-700/70"
        borderRadius="!rounded-none"
        padding="p-0"
    >
        <!-- Header -->
        <div class="flex justify-between items-center p-4 border-b border-neutral-700 flex-shrink-0">
          <h2 id="slideover-title-text" class="text-xl font-semibold text-gray-100 truncate pr-2">{title}</h2>
          <button
            on:click={closePanel}
            aria-label="Close panel"
            class="p-1 rounded-full hover:bg-white/10 text-gray-300 hover:text-gray-100 transition-colors"
          >
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-6 h-6">
              <path stroke-linecap="round" stroke-linejoin="round" d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>

        <!-- Content Slot (scrollable) -->
        <div class="flex-grow p-5 sm:p-6 overflow-y-auto space-y-4">
          <slot name="content"></slot>
        </div>

        <!-- Footer Slot (Optional) -->
        {#if $$slots.footer}
          <div class="p-4 border-t border-neutral-700 flex-shrink-0">
            <slot name="footer"></slot>
          </div>
        {/if}
    </GlassCard>
  </div>
{/if}

<style lang="postcss">
  /* Add any specific global styles for SlideOver if needed, but prefer Tailwind utilities */
  /* Ensure the backdrop-blur works well */
  /* Consider more robust focus trapping for complex forms within the slide over */
</style>
