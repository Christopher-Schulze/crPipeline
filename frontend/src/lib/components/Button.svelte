<script lang="ts">
  export let variant: 'primary' | 'secondary' | 'ghost' | 'danger' = 'primary';
  export let type: 'button' | 'submit' | 'reset' = 'button';
  export let disabled: boolean = false;
  export let customClass: string = ''; // Renamed from 'class' to avoid conflict if used directly
  export let href: string | undefined = undefined;

  let baseClasses: string = "btn"; // Common disabled behavior

  let currentVariantClasses: string = '';

  // Reactive statement to update classes based on variant and disabled state
  $: {
    switch (variant) {
      case 'primary':
        currentVariantClasses = "btn-primary";
        break;
      case 'secondary':
        currentVariantClasses = "btn-secondary";
        break;
      case 'ghost':
        currentVariantClasses = "btn-ghost";
        break;
      case 'danger':
        currentVariantClasses = "btn-error";
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
