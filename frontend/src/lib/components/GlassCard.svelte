<script lang="ts">
  export let title: string | undefined = undefined;
  export let titleClass: string = "text-2xl font-semibold mb-4 text-gray-800"; // Added prop for title class

  // Props for future enhancements, matching typical Tailwind scales or direct values
  export let bgOpacity: string = 'bg-white/60'; // e.g., bg-white/60, bg-neutral-100/70
  export let backdropBlur: string = 'backdrop-blur-lg'; // e.g., backdrop-blur-sm, backdrop-blur-md, backdrop-blur-lg
  export let borderStyle: string = 'border border-white/40'; // e.g., border border-white/40, border-neutral-200/50
  export let borderRadius: string = 'rounded-2xl'; // e.g., rounded-lg, rounded-xl, rounded-2xl
  export let shadow: string = 'shadow-lg'; // e.g., shadow-md, shadow-lg, shadow-xl
  export let padding: string = 'p-6'; // e.g., p-4, p-6, p-8
  export let customClass: string = ''; // Allow passing additional classes

  // Combine all classes using DaisyUI card with glass effect
  $: cardContainerClasses = `card glass ${bgOpacity} ${backdropBlur} ${borderStyle} ${borderRadius} ${shadow} ${customClass}`;
  $: bodyClasses = `card-body ${padding}`;
</script>

<div class="{cardContainerClasses}">
  {#if title}
    <h2 class="card-title {titleClass}">{title}</h2>
  {/if}
  <div class="{bodyClasses}">
    <slot />
  </div>
</div>

<style lang="postcss">
  /* This style block is primarily for documentation or very specific overrides. */
  /* Tailwind utility classes are applied dynamically via props. */

  /*
    Ensure your project's Tailwind CSS configuration includes the necessary classes,
    especially if using arbitrary values like `bg-white/60`.
    Standard Tailwind JIT mode should handle this fine.
  */
</style>
