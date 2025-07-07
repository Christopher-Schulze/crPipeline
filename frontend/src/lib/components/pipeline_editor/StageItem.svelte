<script lang="ts">
  import Button from '../Button.svelte';
  import ParseConfigEditor from './ParseConfigEditor.svelte';
  import StageAiFields from './StageAiFields.svelte';
  import StageOcrFields from './StageOcrFields.svelte';
  import StageReportFields from './StageReportFields.svelte';
  import type { Stage } from '$lib/types/api';
  import type { EditorPromptTemplate } from './types';
  import { createEventDispatcher } from 'svelte';

  export let stage: Stage;
  export let index: number;
  export let availablePromptTemplates: EditorPromptTemplate[] = [];
  export let isLoadingOrgSettings: boolean = false;
  export let dragState: { draggedItemId: string | null; draggedOverIndex: number | null; draggingVisualIndex: number | null; };
  export let handleDragStart: (e: DragEvent, id: string, idx: number) => void;
  export let handleDragOver: (e: DragEvent, idx: number) => void;
  export let handleDragEnter: (e: DragEvent, idx: number) => void;
  export let handleDragLeave: (e: DragEvent) => void;
  export let handleDrop: (e: DragEvent, idx: number) => void;
  export let handleDragEnd: () => void;
  export let initializeParseStrategyParameters: (stage: Stage) => void;

  const dispatch = createEventDispatcher();

  function remove() {
    dispatch('remove');
  }

  function changed() {
    dispatch('update');
  }
</script>

<div
  class={`stage-item card glass p-4 cursor-grab border ${
    dragState.draggingVisualIndex === index ? 'dragging border-accent' : 'border-neutral-700/70'
  } ${
    dragState.draggedOverIndex === index && dragState.draggedItemId !== stage.id
      ? 'drag-over-highlight border-accent'
      : 'hover:border-neutral-600'
  }`}
  draggable="true"
  on:dragstart={(event) => handleDragStart(event, stage.id, index)}
  on:dragover={(event) => handleDragOver(event, index)}
  on:dragenter={(event) => handleDragEnter(event, index)}
  on:dragleave={handleDragLeave}
  on:drop={(event) => handleDrop(event, index)}
  on:dragend={handleDragEnd}
>
  <div class="flex items-center gap-2 mb-2">
    <input
      class="glass-input flex-1 !bg-neutral-600/50 !border-neutral-500/70 !text-gray-100"
      bind:value={stage.type}
      on:input={changed}
      placeholder="Stage Type (e.g., ocr, ai)"
    />
    <input
      class="glass-input flex-1 !bg-neutral-600/50 !border-neutral-500/70 !text-gray-100"
      bind:value={stage.command}
      on:input={changed}
      placeholder="Command / Config (optional)"
    />
    <Button variant="ghost" customClass="!text-error hover:!text-error-content !p-1" on:click={remove}>
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="w-5 h-5">
        <path
          fill-rule="evenodd"
          d="M8.75 1A2.75 2.75 0 006 3.75v.443c-.795.077-1.58.176-2.365.298a.75.75 0 10.23 1.482l.149-.022.841 10.518A2.75 2.75 0 007.596 19h4.807a2.75 2.75 0 002.742-2.53l.841-10.52.149.023a.75.75 0 00.23-1.482A41.03 41.03 0 0014 4.193V3.75A2.75 2.75 0 0011.25 1h-2.5zM10 4c.84 0 1.673.025 2.5.075V3.75c0-.69-.56-1.25-1.25-1.25h-2.5c-.69 0-1.25.56-1.25 1.25V4c.827-.05 1.66-.075 2.5-.075zM8.088 15.816a1.25 1.25 0 01-1.24-1.22L6.003 6.51a.75.75 0 111.494-.138l.84 8.088a1.25 1.25 0 01-1.24 1.221zM11.912 15.816a1.25 1.25 0 01-1.24-1.221l-.84-8.088a.75.75 0 111.494.138l.84 8.088a1.25 1.25 0 01-1.24 1.22z"
          clip-rule="evenodd"
        />
      </svg>
    </Button>
  </div>

  {#if stage.type === 'ai'}
    <StageAiFields {stage} {availablePromptTemplates} {isLoadingOrgSettings} onChange={changed} />
  {/if}

  {#if stage.type.toLowerCase() === 'ocr'}
    <StageOcrFields {stage} onChange={changed} />
  {/if}

  {#if stage.type.toLowerCase() === 'parse'}
    <ParseConfigEditor {stage} {initializeParseStrategyParameters} />
  {/if}

  {#if stage.type.toLowerCase() === 'report'}
    <StageReportFields {stage} onChange={changed} />
  {/if}
</div>

<style>
  .stage-item {
    transition: background-color 0.2s ease-out, border 0.2s ease-out;
    border: 2px solid transparent;
  }
  .dragging {
    opacity: 0.5;
    background-color: rgba(255, 255, 255, 0.1);
  }
  .drag-over-highlight {
    border-color: #3b82f6;
    background-color: rgba(59, 130, 246, 0.1);
  }
</style>
