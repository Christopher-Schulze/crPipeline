<script lang="ts">
  export let variant: 'primary' | 'secondary' | 'ghost' = 'primary';
  export let type: 'button' | 'submit' | 'reset' = 'button';
  export let disabled: boolean = false;
  export let customClass: string = '';
  export let href: string | undefined = undefined;

  $: variantClass =
    variant === 'primary'
      ? 'btn-primary'
      : variant === 'secondary'
      ? 'btn-secondary'
      : 'btn-ghost';
</script>

{#if href}
  <a
    href={href}
    class="btn {variantClass} {customClass}"
    aria-disabled={disabled}
    on:click
  >
    <slot />
  </a>
{:else}
  <button
    type={type}
    class="btn {variantClass} {customClass}"
    {disabled}
    on:click
  >
    <slot />
  </button>
{/if}

