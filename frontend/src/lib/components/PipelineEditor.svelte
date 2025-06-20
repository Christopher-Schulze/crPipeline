<script lang="ts">
  import Button from './Button.svelte';
  export let pipeline = {
    name: '',
    stages: [] as { type: string; command: string }[]
  };

  let newStageType = '';
  let newCommand = '';

  function addStage() {
    if (newStageType.trim()) {
      pipeline.stages = [
        ...pipeline.stages,
        { type: newStageType.trim(), command: newCommand.trim() }
      ];
      newStageType = '';
      newCommand = '';
    }
  }

  function removeStage(index: number) {
    pipeline.stages = pipeline.stages.filter((_, i) => i !== index);
  }

  function moveUp(index: number) {
    if (index === 0) return;
    const items = [...pipeline.stages];
    [items[index - 1], items[index]] = [items[index], items[index - 1]];
    pipeline.stages = items;
  }

  function moveDown(index: number) {
    if (index === pipeline.stages.length - 1) return;
    const items = [...pipeline.stages];
    [items[index + 1], items[index]] = [items[index], items[index + 1]];
    pipeline.stages = items;
  }
</script>

<div class="space-y-4">
  <input class="glass-input w-full" bind:value={pipeline.name} placeholder="Pipeline name" />
  <div class="space-y-2">
    {#each pipeline.stages as stage, i}
      <div class="flex items-center gap-2">
        <input class="glass-input flex-1" bind:value={stage.type} />
        <input class="glass-input flex-1" bind:value={stage.command} placeholder="Command" />
        <button class="text-sm" on:click={() => moveUp(i)}>&uarr;</button>
        <button class="text-sm" on:click={() => moveDown(i)}>&darr;</button>
        <button class="text-sm text-red-500" on:click={() => removeStage(i)}>x</button>
      </div>
    {/each}
    <div class="flex gap-2">
      <input class="glass-input flex-1" bind:value={newStageType} placeholder="Stage type" />
      <input class="glass-input flex-1" bind:value={newCommand} placeholder="Command" />
      <Button class="px-2" on:click={addStage}>Add</Button>
    </div>
  </div>
</div>
