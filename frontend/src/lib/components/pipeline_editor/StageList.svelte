<script lang="ts">
  import Button from '../Button.svelte';
  import RegexPatternEditor from './RegexPatternEditor.svelte';
  import type { Stage, EditorPromptTemplate } from './types';
  import { createEventDispatcher } from 'svelte';

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

  let draggedItemId: string | null = null;
  let draggedOverIndex: number | null = null;
  let draggingVisualIndex: number | null = null;

  function handleDragStart(event: DragEvent, stageId: string, index: number) {
    draggedItemId = stageId;
    draggingVisualIndex = index;
    event.dataTransfer!.setData('text/plain', stageId);
    event.dataTransfer!.effectAllowed = 'move';
  }

  function handleDragOver(event: DragEvent, targetIndex: number) {
    event.preventDefault();
    if (draggedItemId === null) return;
    const targetStageId = stages[targetIndex]?.id;
    if (draggedItemId !== targetStageId) {
      draggedOverIndex = targetIndex;
    } else {
      draggedOverIndex = null;
    }
    event.dataTransfer!.dropEffect = 'move';
  }

  function handleDragEnter(event: DragEvent, targetIndex: number) {
    event.preventDefault();
    if (draggedItemId === null) return;
    const targetStageId = stages[targetIndex]?.id;
    if (draggedItemId !== targetStageId) {
      draggedOverIndex = targetIndex;
    }
  }

  function handleDragLeave(event: DragEvent) {
    const currentTarget = event.currentTarget as HTMLElement;
    const relatedTarget = event.relatedTarget as HTMLElement | null;
    if (!relatedTarget || !currentTarget.contains(relatedTarget)) {
      /* nothing */
    }
  }

  function handleDrop(event: DragEvent, targetItemIndex: number) {
    event.preventDefault();
    if (!draggedItemId) return;
    const draggedItemOriginalIndex = stages.findIndex((s) => s.id === draggedItemId);
    if (draggedItemOriginalIndex === -1) return;
    const items = [...stages];
    const [draggedItem] = items.splice(draggedItemOriginalIndex, 1);
    items.splice(targetItemIndex, 0, draggedItem);
    stages = items;
    draggedItemId = null;
    draggingVisualIndex = null;
    draggedOverIndex = null;
    dispatch('update', stages);
  }

  function handleDragEnd() {
    draggedItemId = null;
    draggingVisualIndex = null;
    draggedOverIndex = null;
  }

  export function initializeParseStrategyParameters(stage: Stage) {
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
          headerKeywords: [],
          stopKeywords: [],
        };
        break;
      case 'Passthrough':
      default:
        stage.config.parameters = {};
        break;
    }
    stages = [...stages];
  }
</script>

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

<div class="space-y-3">
  {#each stages as stage, i (stage.id)}
    <div
      class={`stage-item p-4 rounded-lg cursor-grab border-2 ${
        draggingVisualIndex === i ? 'dragging !border-accent' : 'border-neutral-700/70'
      } ${draggedOverIndex === i && draggedItemId !== stage.id ? 'drag-over-highlight !border-accent' : 'hover:border-neutral-600'}`}
      draggable="true"
      on:dragstart={(event) => handleDragStart(event, stage.id, i)}
      on:dragover={(event) => handleDragOver(event, i)}
      on:dragenter={(event) => handleDragEnter(event, i)}
      on:dragleave={handleDragLeave}
      on:drop={(event) => handleDrop(event, i)}
      on:dragend={handleDragEnd}
    >
      <div class="flex items-center gap-2 mb-2">
        <input
          class="glass-input flex-1 !bg-neutral-600/50 !border-neutral-500/70 !text-gray-100"
          bind:value={stage.type}
          placeholder="Stage Type (e.g., ocr, ai)"
        />
        <input
          class="glass-input flex-1 !bg-neutral-600/50 !border-neutral-500/70 !text-gray-100"
          bind:value={stage.command}
          placeholder="Command / Config (optional)"
        />
        <Button
          variant="ghost"
          customClass="!text-red-500 hover:!text-red-400 !p-1"
          on:click={() => removeStage(stage.id)}
        >
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
        <div class="form-group pt-2 border-t border-neutral-700/50">
          <label for={`stage-prompt-${stage.id}`} class="block text-xs font-light text-gray-300 mb-1">
            Prompt Template
          </label>
          {#if isLoadingOrgSettings}
            <p class="text-sm font-light text-gray-400 dark:text-gray-500 py-2">Loading prompt templates...</p>
          {:else if availablePromptTemplates.length > 0}
            <select
              id={`stage-prompt-${stage.id}`}
              bind:value={stage.prompt_name}
              class="glass-input w-full text-sm !bg-neutral-600/50 !border-neutral-500/70 !text-gray-100"
            >
              <option value={undefined}>Default (No specific template)</option>
              {#each availablePromptTemplates as template (template.name)}
                <option value={template.name}>{template.name}</option>
              {/each}
            </select>
          {:else}
            <p class="text-sm font-light text-gray-400 dark:text-gray-500 py-2">No prompt templates defined. AI uses default behavior.</p>
          {/if}
          {#if stage.prompt_name && availablePromptTemplates.find((p) => p.name === stage.prompt_name)}
            {@const selectedTemplateText = availablePromptTemplates.find((p) => p.name === stage.prompt_name)?.text}
            {#if selectedTemplateText}
              <div class="mt-1.5 p-1.5 bg-black/30 rounded text-xs text-gray-400 max-h-24 overflow-y-auto custom-scrollbar border border-neutral-600/50">
                <strong class="text-gray-300">Preview:</strong>
                <pre class="whitespace-pre-wrap font-mono text-[0.7rem] leading-snug">
                  {selectedTemplateText.substring(0, 150)}{selectedTemplateText.length > 150 ? '...' : ''}
                </pre>
              </div>
            {/if}
          {/if}
        </div>
      {/if}

      {#if stage.type.toLowerCase() === 'ocr'}
        <div class="form-group mt-2 pt-2 border-t border-neutral-700/50 space-y-2">
          <div>
            <label for={`stage-ocr-engine-${stage.id}`} class="block text-xs font-light text-gray-300 mb-1">OCR Engine</label>
            <select
              bind:value={stage.ocr_engine}
              id={`stage-ocr-engine-${stage.id}`}
              class="glass-input w-full text-sm !bg-neutral-600/50 !border-neutral-500/70 !text-gray-100"
            >
              <option value="default">Default (Tesseract/Local)</option>
              <option value="external">External API</option>
            </select>
          </div>

          {#if stage.ocr_engine === 'external'}
            <div class="mt-2 space-y-2 pl-2 border-l-2 border-neutral-700/40 ml-1">
              <div class="pt-1">
                <label for={`stage-ocr-endpoint-${stage.id}`} class="block text-xs font-light text-gray-300 mb-1">
                  Stage OCR API Endpoint
                </label>
                <input
                  type="text"
                  id={`stage-ocr-endpoint-${stage.id}`}
                  bind:value={stage.ocr_stage_endpoint}
                  class="glass-input w-full text-sm !bg-neutral-600/50 !border-neutral-500/70 !text-gray-100"
                  placeholder="Overrides global OCR endpoint"
                />
              </div>
              <div>
                <label for={`stage-ocr-key-${stage.id}`} class="block text-xs font-light text-gray-300 mb-1">
                  Stage OCR API Key
                </label>
                <input
                  type="password"
                  id={`stage-ocr-key-${stage.id}`}
                  bind:value={stage.ocr_stage_key}
                  class="glass-input w-full text-sm !bg-neutral-600/50 !border-neutral-500/70 !text-gray-100"
                  placeholder="Overrides global OCR key"
                />
              </div>
            </div>
          {/if}
        </div>
      {/if}

      {#if stage.type.toLowerCase() === 'parse'}
        <div class="form-group mt-3 pt-3 border-t border-neutral-700/50 space-y-3">
          <div>
            <label for={`stage-parse-strategy-${stage.id}`} class="block text-xs font-medium text-gray-300 mb-1">
              Parsing Strategy
            </label>
            <select
              id={`stage-parse-strategy-${stage.id}`}
              bind:value={stage.config.strategy}
              on:change={() => initializeParseStrategyParameters(stage)}
              class="glass-input w-full !text-sm !bg-neutral-600/50 !border-neutral-500/70 !text-gray-100"
            >
              <option value="Passthrough">Passthrough (Basic Lines/Text)</option>
              <option value="KeywordExtraction">Keyword Extraction</option>
              <option value="RegexExtraction">Regex Extraction</option>
              <option value="SimpleTableExtraction">Simple Table Extraction (Basic Placeholder)</option>
            </select>
          </div>

          {#if stage.config?.strategy === 'KeywordExtraction'}
            <div class="pl-3 border-l-2 border-neutral-700 space-y-2 py-2 text-xs">
              <label class="block font-medium text-gray-300 mb-1">Keywords:</label>
              {#each stage.config.parameters.keywords as keyword, k (k)}
                <div class="flex items-center space-x-2">
                  <input
                    type="text"
                    bind:value={stage.config.parameters.keywords[k]}
                    class="glass-input flex-grow !text-xs !bg-neutral-500/40"
                    placeholder="Enter keyword"
                  />
                  <Button
                    variant="ghost"
                    customClass="!px-1.5 !py-0.5 !text-red-400 hover:!text-red-300"
                    on:click={() => (stage.config.parameters.keywords = stage.config.parameters.keywords.filter((_, idx) => idx !== k))}
                  >
                    X
                  </Button>
                </div>
              {/each}
              <Button
                variant="secondary"
                customClass="!text-xs !py-1"
                on:click={() => (stage.config.parameters.keywords = [...stage.config.parameters.keywords, ''])}
              >
                Add Keyword
              </Button>
              <label class="flex items-center space-x-2 mt-2 cursor-pointer">
                <input
                  type="checkbox"
                  bind:checked={stage.config.parameters.caseSensitive}
                  class="form-checkbox h-4 w-4 text-accent rounded !bg-neutral-700 border-neutral-600 focus:ring-accent/50"
                />
                <span class="text-gray-300">Case Sensitive</span>
              </label>
            </div>
          {/if}

          {#if stage.config?.strategy === 'RegexExtraction'}
            <div class="pl-3 border-l-2 border-neutral-700">
              <RegexPatternEditor bind:patterns={stage.config.parameters.patterns} />
            </div>
          {/if}

          {#if stage.config?.strategy === 'SimpleTableExtraction'}
            <div class="pl-3 border-l-2 border-neutral-700 space-y-2 py-2 text-xs">
              <label class="block font-medium text-gray-300 mb-0.5">Header Keywords (comma-separated):</label>
              <input
                type="text"
                bind:value={stage.config.parameters._headerKeywordsString}
                on:input={() =>
                  (stage.config.parameters.headerKeywords = (stage.config.parameters._headerKeywordsString || '')
                    .split(',')
                    .map((s) => s.trim())
                    .filter((s) => s))}
                class="glass-input w-full !text-xs !bg-neutral-500/40"
                placeholder="e.g., Item, Qty, Price"
              />
              <label class="block font-medium text-gray-300 mt-1 mb-0.5">Stop Keywords (optional, comma-separated):</label>
              <input
                type="text"
                bind:value={stage.config.parameters._stopKeywordsString}
                on:input={() =>
                  (stage.config.parameters.stopKeywords = (stage.config.parameters._stopKeywordsString || '')
                    .split(',')
                    .map((s) => s.trim())
                    .filter((s) => s))}
                class="glass-input w-full !text-xs !bg-neutral-500/40"
                placeholder="e.g., Total, Subtotal"
              />
            </div>
          {/if}
        </div>
      {/if}

      {#if stage.type.toLowerCase() === 'report'}
        <div class="form-group mt-3 pt-3 border-t border-neutral-700/50 space-y-3">
          <div>
            <label for={`stage-report-template-${stage.id}`} class="block text-xs font-medium text-gray-300 mb-1">
              Report Markdown Template
            </label>
            <textarea
              id={`stage-report-template-${stage.id}`}
              bind:value={stage.config.template}
              rows={8}
              class="glass-input w-full !text-sm custom-scrollbar !bg-neutral-600/50 !border-neutral-500/70 !text-gray-100"
              placeholder="Enter Markdown template. Use {{placeholder.path}} for data. e.g., {{document_name}}, {{ai_result.summary}}"
            ></textarea>
            <p class="text-sm font-light text-gray-400 dark:text-gray-500 mt-1">
              Available placeholders depend on data from previous stages (e.g., `ai_result`, `parse_result`) and job metadata (`document_name`, `job_id`).
            </p>
          </div>

          <div>
            <label for={`stage-report-summary-fields-${stage.id}`} class="block text-xs font-medium text-gray-300 mb-1">
              JSON Summary Fields (Optional, comma-separated JSONPaths)
            </label>
            <input
              type="text"
              id={`stage-report-summary-fields-${stage.id}`}
              bind:value={stage.config._summaryFieldsString}
              on:input={() => {
                if (!stage.config) stage.config = { template: '', summaryFields: [], _summaryFieldsString: '' };
                stage.config.summaryFields = (stage.config._summaryFieldsString || '')
                  .split(',')
                  .map((s) => s.trim())
                  .filter((s) => s);
                stages = [...stages];
              }}
              class="glass-input w-full !text-sm !bg-neutral-600/50 !border-neutral-500/70 !text-gray-100"
              placeholder="e.g., previous_stage_output.field.name, ai_result.sentiment"
            />
            <p class="text-sm font-light text-gray-400 dark:text-gray-500 mt-1">
              Enter JSONPath expressions (e.g., `previous_stage_output.field.name`) relative to the data available to the report stage. Each path creates a key in the summary JSON.
            </p>
          </div>
        </div>
      {/if}
    </div>
  {/each}
</div>

