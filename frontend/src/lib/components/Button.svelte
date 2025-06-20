<script lang="ts">
  export let variant: 'primary' | 'secondary' | 'ghost' = 'primary';
  const base = 'px-4 py-2 rounded-lg font-semibold transition focus:outline-none';
  $: classes = `${base} ${variant === 'primary' ? 'bg-accent/80 text-white backdrop-blur-sm shadow-md hover:bg-accent' : variant === 'secondary' ? 'bg-white/40 backdrop-blur-sm text-black shadow-md hover:bg-white/60' : 'bg-transparent text-black hover:bg-black/5'}`;
</script>

<button class={classes}>
  <slot />
</button>
