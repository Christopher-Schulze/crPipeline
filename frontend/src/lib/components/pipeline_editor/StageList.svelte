<script lang="ts">
  import type { Stage } from '$lib/types/api';
  import type { EditorPromptTemplate } from './types';
  import { createEventDispatcher } from 'svelte';
  import StageItem from './StageItem.svelte';
  import { createDragHandlers } from './dnd';

  export let stages: Stage[] = [];
  export let availablePromptTemplates: EditorPromptTemplate[] = [];
  export let isLoadingOrgSettings: boolean = false;

  const dispatch = createEventDispatcher();

  export function addStage(newStage: Stage) {
    stages = [...stages, newStage];
    dispatch('update', stages);
  }

  export function removeStage(id: string) {
    stages = stages.filter((s) => s.id !== id);
    dispatch('update', stages);
  }

  function updateStages() {
    stages = [...stages];
    dispatch('update', stages);
  }

  function initializeParseStrategyParameters(stage: Stage) {
    if (stage.type.toLowerCase() !== 'parse') {
      if (stage.config) stage.config = undefined;
      stages = [...stages];
      return;
    }
    if (!stage.config) {
      stage.config = { strategy: 'Passthrough', parameters: {} };
    }
    switch (stage.config.strategy) {
      case 'KeywordExtraction':
        stage.config.parameters = { keywords: [''], caseSensitive: false };
        break;
      case 'RegexExtraction':
        stage.config.parameters = {
          patterns: [
            {
              id: Date.now().toString() + Math.random().toString(36).substring(2, 9),
              name: '',
              regex: '',
              captureGroupIndex: 1,
            },
          ],
        };
        break;
      case 'SimpleTableExtraction':
        stage.config.parameters = {
          _headerKeywordsString: '',
          _stopKeywordsString: '',
          _delimiterRegex: '',
          headerKeywords: [],
          stopKeywords: [],
          delimiterRegex: '',
          numericSummary: false,
        };
        break;
      case 'Passthrough':
      default:
        stage.config.parameters = {};
        break;
    }
    stages = [...stages];
  }

  const {
    state: dragState,
    handleDragStart,
    handleDragOver,
    handleDragEnter,
    handleDragLeave,
    handleDrop,
    handleDragEnd,
  } = createDragHandlers<Stage>(
    () => stages,
    (items) => {
      stages = items;
      dispatch('update', stages);
    }
  );
</script>

<div class="space-y-3">
  {#each stages as stage, i (stage.id)}
    <StageItem
      {stage}
      index={i}
      {availablePromptTemplates}
      {isLoadingOrgSettings}
      {dragState}
      {initializeParseStrategyParameters}
      on:remove={() => removeStage(stage.id)}
      on:update={updateStages}
      on:dragstart={(e) => handleDragStart(e, stage.id, i)}
      on:dragover={(e) => handleDragOver(e, i)}
      on:dragenter={(e) => handleDragEnter(e, i)}
      on:dragleave={handleDragLeave}
      on:drop={(e) => handleDrop(e, i)}
      on:dragend={handleDragEnd}
    />
  {/each}
</div>
