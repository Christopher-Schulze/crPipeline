<script lang="ts">
  export let variant: 'primary' | 'secondary' | 'ghost' = 'primary';
  export let type: 'button' | 'submit' | 'reset' = 'button';
  export let disabled: boolean = false;
  export let customClass: string = ''; // Renamed from 'class' to avoid conflict if used directly
  export let href: string | undefined = undefined;

  let baseClasses: string =
    "px-4 py-2 rounded-lg font-semibold focus:outline-none focus:ring-2 focus:ring-offset-2 \
    transition-colors duration-150 ease-in-out inline-flex items-center justify-center \
    disabled:cursor-not-allowed"; // Common disabled behavior

  let currentVariantClasses: string = '';

  // Reactive statement to update classes based on variant and disabled state
  $: {
    switch (variant) {
      case 'primary':
        currentVariantClasses = `
          bg-accent text-white
          hover:bg-accent/80
          focus:ring-accent
          disabled:bg-gray-300 disabled:text-gray-500`;
        break;
      case 'secondary':
        currentVariantClasses = `
          bg-white/70 backdrop-blur-sm text-accent border border-accent/50
          hover:bg-white/90 hover:border-accent
          focus:ring-accent
          disabled:bg-gray-200/50 disabled:text-gray-400 disabled:border-gray-300/50`;
        break;
      case 'ghost':
        currentVariantClasses = `
          bg-transparent text-accent
          hover:bg-accent/10
          focus:ring-accent focus:bg-accent/10
          disabled:text-gray-400`;
        break;
    }
  }
</script>

{#if href}
  <a
    href={href}
    class="{baseClasses} {currentVariantClasses} {customClass}"
    aria-disabled={disabled}
    on:click
  >
    <slot />
  </a>
{:else}
  <button
    type={type}
    class="{baseClasses} {currentVariantClasses} {customClass}"
    {disabled}
    on:click
  >
    <slot />
  </button>
{/if}

<style lang="postcss">
  /* Ensure --color-accent is available globally for bg-accent to work */
  /* e.g., in your app.css or global style block: */
  /* :root { --color-accent: #30D5C8; } */
  /* Tailwind JIT should pick up bg-accent if --color-accent is defined in a way it understands,
     or if 'accent' is configured as a color in tailwind.config.js */

  /* Opacity modifiers like /80, /10 for bg-accent assume that 'accent' is a color
     that Tailwind can apply alpha to (e.g. using CSS variables with R,G,B components or a hex code).
     If --color-accent is just a hex string, Tailwind CSS v3+ usually handles this.
   */
</style>
