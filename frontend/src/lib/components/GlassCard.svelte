<script lang="ts">
  /**
   * Shadow depth. Higher values use a larger drop shadow.
   */
  export let depth: 1 | 2 | 3 = 1;
  /**
   * Glass opacity from 0 (fully transparent) to 1.
   */
  export let opacity = 0.6;
  /**
   * Blur radius in pixels.
   */
  export let blur = 20;
</script>

<div
  class="glass rounded-2xl p-6"
  class:depth-2={depth === 2}
  class:depth-3={depth === 3}
  style="--glass-opacity:{opacity}; --glass-blur:{blur}px"
>
  <slot />
</div>

<style>
  .glass {
    background-color: rgba(255, 255, 255, var(--glass-opacity));
    backdrop-filter: blur(var(--glass-blur));
    border: 1px solid rgba(255, 255, 255, 0.4);
    box-shadow: 0 4px 10px rgba(0, 0, 0, 0.05);
  }
  .depth-2 {
    box-shadow: 0 8px 20px rgba(0, 0, 0, 0.08);
  }
  .depth-3 {
    box-shadow: 0 15px 30px rgba(0, 0, 0, 0.1);
  }
</style>
