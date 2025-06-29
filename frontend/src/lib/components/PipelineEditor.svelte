<script lang="ts">
  import Button from './Button.svelte';
  import { onMount, createEventDispatcher } from 'svelte'; // Import onMount and createEventDispatcher
  import { apiFetch } from '$lib/utils/apiUtils';

  // Define Stage interface for clarity and new fields
  export interface Stage { // Exporting if it's useful for other components, otherwise keep local
    id: string; // Client-side unique ID
    type: string;
    command?: string | null;
    prompt_name?: string | null;

    // New fields for OCR stage configuration
    ocr_engine?: 'default' | 'external';
    ocr_stage_endpoint?: string | null;
    ocr_stage_key?: string | null;

    config?: {
      strategy?: string;
      parameters?: any;
      template?: string;
      summaryFields?: string[];
      _summaryFieldsString?: string;
    } | null;
  }

  interface RegexPatternConfig {
    id: string;
    name: string;
    regex: string;
    captureGroupIndex?: number;
  }

  // Define Pipeline structure, matching +layout.svelte and backend expectations
  export interface Pipeline {
    id?: string;
    name: string;
    org_id: string;
    stages: Stage[];
  }

  export let orgId: string;
  export let initialPipeline: Pipeline | null = null;

  // Internal reactive state for the pipeline being edited
  let pipeline: Pipeline;

  function resetPipeline() {
    pipeline = {
      id: undefined,
      org_id: orgId,
      name: '',
      stages: [],
    };
  }

  // State for available prompt templates
  interface EditorPromptTemplate {
    name: string;
    text: string;
  }
  let availablePromptTemplates: EditorPromptTemplate[] = [];
  let isLoadingOrgSettings: boolean = false;
  let promptTemplatesError: string | null = null;

  async function loadPromptTemplates() {
    if (!orgId) {
      console.warn("OrgID not provided to PipelineEditor, cannot load prompt templates.");
      availablePromptTemplates = [];
      return;
    }
    isLoadingOrgSettings = true;
    promptTemplatesError = null;
    try {
      const response = await apiFetch(`/api/settings/${orgId}`);
      if (!response.ok) {
        throw new Error(`Failed to fetch organization settings: ${response.statusText}`);
      }
      const settingsData = await response.json();
      if (settingsData.prompt_templates && Array.isArray(settingsData.prompt_templates)) {
        availablePromptTemplates = settingsData.prompt_templates.map((pt: any) => ({
          name: pt.name || 'Unnamed Template',
          text: pt.text || ''
        }));
      } else {
        availablePromptTemplates = [];
      }
      promptTemplatesError = null;
    } catch (e: any) {
      console.error("Error loading prompt templates:", e);
      availablePromptTemplates = [];
      promptTemplatesError = 'Failed to load prompt templates.';
    } finally {
      isLoadingOrgSettings = false;
    }
  }

  onMount(() => {
    // Initialize based on initialPipeline prop or reset
    if (initialPipeline) {
      loadPipelineFromProp(initialPipeline);
    } else {
      resetPipeline();
    }
    loadPromptTemplates();
  });

  function initializeStageUiFields(stage: Stage) {
    if (!stage.id) { // Ensure client-side ID
      stage.id = `stage-${Date.now()}-${Math.random().toString(36).substring(2,9)}`;
    }
    // Initialize UI helper for Report stage summaryFields
    if (stage.type?.toLowerCase() === 'report' && stage.config) {
      if (typeof stage.config._summaryFieldsString === 'undefined') {
        stage.config._summaryFieldsString = (stage.config.summaryFields || []).join(', ');
      }
    }
    // Initialize UI helpers for Parse stage SimpleTableExtraction
    if (stage.type?.toLowerCase() === 'parse' && stage.config?.strategy === 'SimpleTableExtraction' && stage.config.parameters) {
      if (typeof stage.config.parameters._headerKeywordsString === 'undefined') {
        stage.config.parameters._headerKeywordsString = (stage.config.parameters.headerKeywords || []).join(', ');
      }
      if (typeof stage.config.parameters._stopKeywordsString === 'undefined') {
        stage.config.parameters._stopKeywordsString = (stage.config.parameters.stopKeywords || []).join(', ');
      }
    }
    // Initialize client-side IDs and default captureGroupIndex for RegexExtraction patterns
    if (stage.type?.toLowerCase() === 'parse' && stage.config?.strategy === 'RegexExtraction' && stage.config.parameters?.patterns) {
      stage.config.parameters.patterns = stage.config.parameters.patterns.map((p: RegexPatternConfig) => ({
        ...p,
        id: p.id || `pattern-${Date.now()}-${Math.random().toString(36).substring(2,9)}`,
        captureGroupIndex: p.captureGroupIndex === undefined ? 1 : p.captureGroupIndex,
      }));
    }
  }

  function loadPipelineFromProp(sourcePipeline: Pipeline) {
    pipeline = JSON.parse(JSON.stringify(sourcePipeline)); // Deep clone
    pipeline.org_id = orgId; // Ensure it's set to the current org context
    pipeline.stages.forEach(initializeStageUiFields);
  }

  // Reactive statement to handle prop changes if the editor is already mounted
  // This is important if the initialPipeline prop changes while the component is still active.
  $: if (initialPipeline && pipeline && initialPipeline.id !== pipeline.id) { // Check if it's a different pipeline
    loadPipelineFromProp(initialPipeline);
  } else if (!initialPipeline && pipeline && (pipeline.id || pipeline.name || pipeline.stages.length > 0) ) {
    // If initialPipeline becomes null (e.g. creating new after editing),
    // and current pipeline isn't already a pristine new one.
    resetPipeline();
  }

  let newStageType = '';
  let newCommand = ''; // This might become less relevant for stages with structured 'config'

  function addStage() { // Modified to use component-level newStageType, newCommand
    if (newStageType.trim()) {
      const type = newStageType.trim().toLowerCase(); // Use lowercase for consistent checks
      let initialConfig: Stage['config'] = undefined;
      let commandValue: string | null = newCommand.trim() || null;

      if (type === 'parse') {
        initialConfig = { strategy: 'Passthrough', parameters: {} };
        commandValue = null;
      } else if (type === 'report') {
        initialConfig = {
            template: `## Report for {{document_name}}\n\nDate: {{job_created_at_formatted}}\n\n### AI Summary\n{{ai_result.summary}}\n\n### Parsed Data Overview\n{{parse_result.overview}}`,
            summaryFields: [],
            _summaryFieldsString: ''
        };
        commandValue = null;
      }

      const newStage: Stage = {
        id: `newstage-${Date.now()}-${Math.random().toString(36).substring(2,9)}`,
        type: newStageType.trim(),
        command: commandValue,
        prompt_name: type === 'ai' ? undefined : undefined,
        ocr_engine: type === 'ocr' ? 'default' : undefined,
        ocr_stage_endpoint: undefined,
        ocr_stage_key: undefined,
        config: initialConfig,
      };
      pipeline.stages = [...pipeline.stages, newStage];
      newStageType = '';
      newCommand = '';
    }
  }

  function initializeParseStrategyParameters(stage: Stage) {
    if (stage.type.toLowerCase() !== 'parse') {
        // This function should only be called for parse stages, but as a safeguard:
        if (stage.config) stage.config = undefined; // Clear config if type changed away from parse
        pipeline.stages = [...pipeline.stages];
        return;
    }

    // Ensure config object exists for a parse stage
    if (!stage.config) {
        stage.config = { strategy: 'Passthrough', parameters: {} };
    }
    // stage.config is now guaranteed to exist

    switch (stage.config.strategy) {
        case 'KeywordExtraction':
            stage.config.parameters = { keywords: [''], caseSensitive: false };
            break;
        case 'RegexExtraction':
            stage.config.parameters = { patterns: [{ id: Date.now().toString() + Math.random().toString(36).substring(2,9), name: '', regex: '' }] };
            break;
        case 'SimpleTableExtraction':
            stage.config.parameters = { _headerKeywordsString: '', _stopKeywordsString: '', headerKeywords: [], stopKeywords: [] };
            break;
        case 'RegexExtraction': // Ensure this also initializes with captureGroupIndex
            stage.config.parameters = {
                patterns: [{
                  id: Date.now().toString() + Math.random().toString(36).substring(2,9),
                  name: '',
                  regex: '',
                  captureGroupIndex: 1 // Default for new patterns in UI
                }]
            };
            break;
        case 'Passthrough':
        default:
            stage.config.parameters = {};
            break;
    }
    // Force reactivity for the entire stages array to ensure UI updates for nested changes
    // Force reactivity for the entire stages array to ensure UI updates for nested changes
    pipeline.stages = [...pipeline.stages];
  }

  // This function prepares the pipeline data for saving (e.g., removing temporary UI fields)
  function getSanitizedPipelineForSave(): Pipeline {
    const pipelineToSave = JSON.parse(JSON.stringify(pipeline));
    pipelineToSave.org_id = orgId;
    if (!pipeline.id) {
      delete pipelineToSave.id;
    }

    pipelineToSave.stages.forEach((stage: Stage) => {
        if (stage.type.toLowerCase() === 'parse' && stage.config?.strategy === 'SimpleTableExtraction' && stage.config.parameters) {
            delete stage.config.parameters._headerKeywordsString;
            delete stage.config.parameters._stopKeywordsString;
            if (stage.config.parameters.stopKeywords && stage.config.parameters.stopKeywords.length === 0) {
                stage.config.parameters.stopKeywords = null;
            }
        }
        if (stage.type.toLowerCase() === 'parse' && stage.config?.strategy === 'RegexExtraction' && stage.config.parameters?.patterns) {
          stage.config.parameters.patterns = stage.config.parameters.patterns.map((pattern: RegexPatternConfig) => {
            const newPattern: Partial<RegexPatternConfig> = { ...pattern };
            if (newPattern.captureGroupIndex === null || newPattern.captureGroupIndex === undefined || isNaN(parseInt(String(newPattern.captureGroupIndex)))) {
              delete newPattern.captureGroupIndex;
            } else {
              newPattern.captureGroupIndex = parseInt(String(newPattern.captureGroupIndex), 10);
            }
            return newPattern as RegexPatternConfig;
          });
        }
        if (stage.type.toLowerCase() === 'report' && stage.config) {
            delete stage.config._summaryFieldsString;
            if (stage.config.summaryFields && stage.config.summaryFields.length === 0) {
                stage.config.summaryFields = null;
            }
        }
    });
    return pipelineToSave;
  }

  async function savePipeline() {
    // Basic validation (name is required)
    if (!pipeline.name.trim()) {
      alert("Pipeline name is required.");
      return;
    }
    if (pipeline.stages.length === 0) {
      alert("Pipeline must have at least one stage.");
      return;
    }

    const isEdit = !!pipeline.id;
    const finalPipelineData = getSanitizedPipelineForSave();
    const url = isEdit ? `/api/pipelines/${pipeline.id}` : '/api/pipelines';
    const method = isEdit ? 'PUT' : 'POST';

    try {
      const response = await apiFetch(url, {
        method: method,
        body: JSON.stringify(finalPipelineData),
      });

      if (response.ok) {
        alert('Pipeline saved successfully!');
        dispatch('saved'); // Notify parent to close the editor
        // Emit global event so other pages can refresh pipeline lists
        document.body.dispatchEvent(new CustomEvent('pipelinesUpdated'));
      } else {
        const errorData = await response.json().catch(() => ({ error: "Unknown error during save." }));
        console.error('Failed to save pipeline:', errorData);
        alert(`Error saving pipeline: ${errorData.error || response.statusText}`);
      }
    } catch (e: any) {
      console.error('Network or other error saving pipeline:', e);
      alert(`Network error while saving pipeline: ${e.message}`);
    }
  }

  async function deletePipeline() {
    if (!pipeline.id) return;
    if (!confirm('Delete this pipeline?')) {
      return;
    }
    try {
      const response = await apiFetch(`/api/pipelines/${pipeline.id}`, { method: 'DELETE' });
      if (response.ok) {
        alert('Pipeline deleted.');
        dispatch('saved');
        document.body.dispatchEvent(new CustomEvent('pipelinesUpdated'));
      } else {
        const errorData = await response.json().catch(() => ({ error: 'Unknown error' }));
        alert(`Error deleting pipeline: ${errorData.error || response.statusText}`);
      }
    } catch (e: any) {
      alert(`Network error while deleting pipeline: ${e.message}`);
    }
  }

  const dispatch = createEventDispatcher(); // For 'saved' and 'cancel' events

  /*
  // Helper function to be called by parent component before saving the pipeline
  // This is now effectively replaced by getSanitizedPipelineForSave for internal use.
  // If it were needed as an export for other reasons, it would need to be maintained.
  export function preparePipelineForSave(pipelineData: { stages: Stage[] }): { stages: Stage[] } {
    const cleanedStages = pipelineData.stages.map(stage => {
        const newStage = { ...stage };
        if (newStage.type.toLowerCase() === 'parse' && newStage.config?.strategy === 'SimpleTableExtraction' && newStage.config.parameters) {
            const params = { ...newStage.config.parameters };
            delete params._headerKeywordsString;
            delete params._stopKeywordsString;
            if (params.stopKeywords && params.stopKeywords.length === 0) {
                params.stopKeywords = null;
            }
            if (newStage.config) {
                 newStage.config = { ...newStage.config, parameters: params };
            }
        }
        if (newStage.type.toLowerCase() === 'parse' && newStage.config?.strategy === 'RegexExtraction' && newStage.config.parameters?.patterns) {
          const patterns = newStage.config.parameters.patterns.map((pattern: RegexPatternConfig) => {
            const newPattern = { ...pattern };
            if (newPattern.captureGroupIndex === null || newPattern.captureGroupIndex === undefined || isNaN(parseInt(String(newPattern.captureGroupIndex)))) {
              delete newPattern.captureGroupIndex;
            } else {
              newPattern.captureGroupIndex = parseInt(String(newPattern.captureGroupIndex), 10);
            }
            return newPattern;
          });
          if (newStage.config && newStage.config.parameters) {
            newStage.config.parameters = { ...newStage.config.parameters, patterns: patterns };
          }
        }
        if (newStage.type.toLowerCase() === 'report' && newStage.config) {
            const cfg = { ...newStage.config };
            delete cfg._summaryFieldsString;
            if (cfg.summaryFields && cfg.summaryFields.length === 0) {
                cfg.summaryFields = null;
            }
            newStage.config = cfg;
        }
        return newStage;
    });
    return { ...pipelineData, stages: cleanedStages };
  }
  */

  function removeStage(idToRemove: string) {
    pipeline.stages = pipeline.stages.filter(stage => stage.id !== idToRemove);
  }

  // State for drag and drop (using itemId based logic)
  let draggedItemId: string | null = null;
  let draggedOverIndex: number | null = null; // Visual index for hover effect
  // draggingIndex (numeric index of dragged item) is not strictly needed if using ID, but can be kept for visuals if preferred
  let draggingVisualIndex: number | null = null;


  function handleDragStart(event: DragEvent, stageId: string, index: number) {
    draggedItemId = stageId;
    draggingVisualIndex = index; // For visual feedback like 'dragging' class
    event.dataTransfer!.setData('text/plain', stageId);
    event.dataTransfer!.effectAllowed = 'move';
  }

  function handleDragOver(event: DragEvent, targetIndex: number) {
    event.preventDefault();
    if (draggedItemId === null) return;

    const targetStageId = pipeline.stages[targetIndex]?.id;
    if (draggedItemId !== targetStageId) { // Don't highlight if dragging over itself based on ID
      draggedOverIndex = targetIndex;
    } else {
      draggedOverIndex = null;
    }
    event.dataTransfer!.dropEffect = 'move';
  }

  function handleDragEnter(event: DragEvent, targetIndex: number) {
    event.preventDefault();
    if (draggedItemId === null) return;
    const targetStageId = pipeline.stages[targetIndex]?.id;
    if (draggedItemId !== targetStageId) {
      draggedOverIndex = targetIndex;
    }
  }

  function handleDragLeave(event: DragEvent) {
    const currentTarget = event.currentTarget as HTMLElement;
    const relatedTarget = event.relatedTarget as HTMLElement | null;
    if (!relatedTarget || !currentTarget.contains(relatedTarget)) {
      // Cleared by dragover on new item or dragend
    }
  }

  function handleDrop(event: DragEvent, targetItemIndex: number) {
    event.preventDefault();
    if (!draggedItemId) return;

    const draggedItemOriginalIndex = pipeline.stages.findIndex(s => s.id === draggedItemId);
    if (draggedItemOriginalIndex === -1) return; // Should not happen

    // If dropped on itself effectively (same index after potential shifts)
    // This logic can be tricky if targetItemIndex is the final visual index.
    // It's often simpler to just re-construct the array based on splice/insert.

    const items = [...pipeline.stages];
    const [draggedItem] = items.splice(draggedItemOriginalIndex, 1); // Get the dragged item

    // Insert at the target drop index
    items.splice(targetItemIndex, 0, draggedItem);

    pipeline.stages = items;

    draggedItemId = null;
    draggingIndex = null; // Reset visual index if used
    draggedOverIndex = null;
  }

  function handleDragEnd() {
    draggedItemId = null;
    draggingVisualIndex = null; // Reset visual index
    draggedOverIndex = null;
  }
</script>

<style>
  .stage-item {
    transition: background-color 0.2s ease-out, border 0.2s ease-out;
    border: 2px solid transparent; /* Reserve space for border highlight */
  }
  .dragging {
    opacity: 0.5;
    background-color: rgba(255, 255,255, 0.1); /* Slight background change */
  }
  .drag-over-highlight {
    /* Visual cue for where the item will be dropped */
    /* E.g., a border on the item it will be inserted before/after */
    border-color: #3b82f6; /* Tailwind blue-500 */
    background-color: rgba(59, 130, 246, 0.1); /* Light blue background */
  }
</style>

<div class="space-y-4 text-gray-200">
  <input class="glass-input w-full !bg-neutral-700/50 !border-neutral-600/80 !text-gray-100" bind:value={pipeline.name} placeholder="Pipeline name" />
  {#if promptTemplatesError}
    <div class="bg-error/20 border border-error/40 text-error px-3 py-2 rounded text-sm">
      {promptTemplatesError}
    </div>
  {/if}
  <div class="space-y-3">
    {#each pipeline.stages as stage, i (stage.id)}
      <div
        class="stage-item p-4 rounded-lg cursor-grab border-2"
               {draggingVisualIndex === i ? 'dragging !border-accent' : 'border-neutral-700/70'}
               {draggedOverIndex === i && draggedItemId !== stage.id ? 'drag-over-highlight !border-accent' : 'hover:border-neutral-600'}"
        draggable="true"
        on:dragstart={(event) => handleDragStart(event, stage.id, i)}
        on:dragover={(event) => handleDragOver(event, i)}
        on:dragenter={(event) => handleDragEnter(event, i)}
        on:dragleave={handleDragLeave}
        on:drop={(event) => handleDrop(event, i)}
        on:dragend={handleDragEnd}
      >
        <div class="flex items-center gap-2 mb-2">
          <input class="glass-input flex-1 !bg-neutral-600/50 !border-neutral-500/70 !text-gray-100" bind:value={stage.type} placeholder="Stage Type (e.g., ocr, ai)"/>
          <input class="glass-input flex-1 !bg-neutral-600/50 !border-neutral-500/70 !text-gray-100" bind:value={stage.command} placeholder="Command / Config (optional)" />
          <Button variant="ghost" customClass="!text-red-500 hover:!text-red-400 !p-1" on:click={() => removeStage(stage.id)}>
            <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="w-5 h-5"><path fill-rule="evenodd" d="M8.75 1A2.75 2.75 0 006 3.75v.443c-.795.077-1.58.176-2.365.298a.75.75 0 10.23 1.482l.149-.022.841 10.518A2.75 2.75 0 007.596 19h4.807a2.75 2.75 0 002.742-2.53l.841-10.52.149.023a.75.75 0 00.23-1.482A41.03 41.03 0 0014 4.193V3.75A2.75 2.75 0 0011.25 1h-2.5zM10 4c.84 0 1.673.025 2.5.075V3.75c0-.69-.56-1.25-1.25-1.25h-2.5c-.69 0-1.25.56-1.25 1.25V4c.827-.05 1.66-.075 2.5-.075zM8.088 15.816a1.25 1.25 0 01-1.24-1.22L6.003 6.51a.75.75 0 111.494-.138l.84 8.088a1.25 1.25 0 01-1.24 1.221zM11.912 15.816a1.25 1.25 0 01-1.24-1.221l-.84-8.088a.75.75 0 111.494.138l.84 8.088a1.25 1.25 0 01-1.24 1.22z" clip-rule="evenodd" /></svg>
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
            {#if stage.prompt_name && availablePromptTemplates.find(p => p.name === stage.prompt_name)}
                {@const selectedTemplateText = availablePromptTemplates.find(p => p.name === stage.prompt_name)?.text}
                {#if selectedTemplateText}
                    <div class="mt-1.5 p-1.5 bg-black/30 rounded text-xs text-gray-400 max-h-24 overflow-y-auto custom-scrollbar border border-neutral-600/50">
                        <strong class="text-gray-300">Preview:</strong>
                        <pre class="whitespace-pre-wrap font-mono text-[0.7rem] leading-snug">{selectedTemplateText.substring(0, 150)}{selectedTemplateText.length > 150 ? '...' : ''}</pre>
                    </div>
                {/if}
            {/if}
          </div>
        {/if}
        {#if stage.type.toLowerCase() === 'ocr'}
          <div class="form-group mt-2 pt-2 border-t border-neutral-700/50 space-y-2">
            <div>
              <label for={`stage-ocr-engine-${stage.id}`} class="block text-xs font-light text-gray-300 mb-1">OCR Engine</label>
              <select bind:value={stage.ocr_engine} id={`stage-ocr-engine-${stage.id}`} class="glass-input w-full text-sm !bg-neutral-600/50 !border-neutral-500/70 !text-gray-100">
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

        <!-- Parse Stage Configuration -->
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

            <!-- Parameters for KeywordExtraction -->
            {#if stage.config?.strategy === 'KeywordExtraction'}
              <div class="pl-3 border-l-2 border-neutral-700 space-y-2 py-2 text-xs">
                <label class="block font-medium text-gray-300 mb-1">Keywords:</label>
                {#each stage.config.parameters.keywords as keyword, k (k)}
                  <div class="flex items-center space-x-2">
                    <input type="text" bind:value={stage.config.parameters.keywords[k]} class="glass-input flex-grow !text-xs !bg-neutral-500/40" placeholder="Enter keyword"/>
                    <Button variant="ghost" customClass="!px-1.5 !py-0.5 !text-red-400 hover:!text-red-300" on:click={() => stage.config.parameters.keywords = stage.config.parameters.keywords.filter((_: any, idx: number) => idx !== k)}>X</Button>
                  </div>
                {/each}
                <Button variant="secondary" customClass="!text-xs !py-1" on:click={() => stage.config.parameters.keywords = [...stage.config.parameters.keywords, '']}>Add Keyword</Button>
                <label class="flex items-center space-x-2 mt-2 cursor-pointer">
                  <input type="checkbox" bind:checked={stage.config.parameters.caseSensitive} class="form-checkbox h-4 w-4 text-accent rounded !bg-neutral-700 border-neutral-600 focus:ring-accent/50"/>
                  <span class="text-gray-300">Case Sensitive</span>
                </label>
              </div>
            {/if}

            <!-- Parameters for RegexExtraction -->
            {#if stage.config?.strategy === 'RegexExtraction'}
              <div class="pl-3 border-l-2 border-neutral-700 space-y-2 py-2 text-xs">
                <label class="block font-medium text-gray-300 mb-1">Regex Patterns:</label>
                {#each stage.config.parameters.patterns as pattern, k (pattern.id)}
                  <div class="p-2 bg-black/20 rounded space-y-1.5 mb-1.5">
                      <div class="flex items-center space-x-2">
                          <input type="text" bind:value={pattern.name} class="glass-input flex-grow !text-xs !bg-neutral-500/40" placeholder="Field Name (e.g., InvoiceID)"/>
                          <Button variant="ghost" customClass="!px-1.5 !py-0.5 !text-red-400 hover:!text-red-300" on:click={() => stage.config.parameters.patterns = stage.config.parameters.patterns.filter((p: RegexPatternConfig) => p.id !== pattern.id)}>X</Button>
                      </div>
                      <input type="text" bind:value={pattern.regex} class="glass-input w-full !text-xs !bg-neutral-500/40" placeholder="Regex Pattern (e.g., INV-\d+)"/>
                      <!-- New Input for Capture Group Index -->
                      <div>
                        <label for={`pattern-group-index-${pattern.id}`} class="block text-xs font-medium text-gray-400 mb-0.5 mt-1">
                          Capture Group Index
                        </label>
                        <input
                          type="number"
                          id={`pattern-group-index-${pattern.id}`}
                          bind:value={pattern.captureGroupIndex}
                          min="0"
                          step="1"
                          class="glass-input w-full !text-xs !bg-neutral-500/40"
                          placeholder="e.g., 1 (0 for full match)"
                        />
                        <p class="text-sm font-light text-gray-500 dark:text-gray-400 mt-0.5">Default: 1 (first group). Use 0 for full match.</p>
                      </div>
                  </div>
                {/each}
                <Button variant="secondary" customClass="!text-xs !py-1" on:click={() => stage.config.parameters.patterns = [...stage.config.parameters.patterns, {id: Date.now().toString() + Math.random().toString(36).substring(2,9), name: '', regex: '', captureGroupIndex: 1}]}>Add Regex Pattern</Button>
              </div>
            {/if}

            <!-- Parameters for SimpleTableExtraction -->
            {#if stage.config?.strategy === 'SimpleTableExtraction'}
              <div class="pl-3 border-l-2 border-neutral-700 space-y-2 py-2 text-xs">
                  <label class="block font-medium text-gray-300 mb-0.5">Header Keywords (comma-separated):</label>
                  <input type="text" bind:value={stage.config.parameters._headerKeywordsString}
                         on:input={() => stage.config.parameters.headerKeywords = (stage.config.parameters._headerKeywordsString || '').split(',').map((s:string)=>s.trim()).filter((s:string)=>s)}
                         class="glass-input w-full !text-xs !bg-neutral-500/40" placeholder="e.g., Item, Qty, Price"/>
                  <label class="block font-medium text-gray-300 mt-1 mb-0.5">Stop Keywords (optional, comma-separated):</label>
                  <input type="text" bind:value={stage.config.parameters._stopKeywordsString}
                         on:input={() => stage.config.parameters.stopKeywords = (stage.config.parameters._stopKeywordsString || '').split(',').map((s:string)=>s.trim()).filter((s:string)=>s)}
                         class="glass-input w-full !text-xs !bg-neutral-500/40" placeholder="e.g., Total, Subtotal"/>
              </div>
            {/if}
          </div>
        {/if}

        <!-- Report Stage Configuration -->
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
                    if (!stage.config) stage.config = { template: '', summaryFields: [], _summaryFieldsString: '' }; // Ensure config exists
                    stage.config.summaryFields = (stage.config._summaryFieldsString || '')
                                                    .split(',')
                                                    .map(s => s.trim())
                                                    .filter(s => s);
                    pipeline.stages = [...pipeline.stages]; // Force reactivity
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
    <div class="flex gap-2 mt-3 p-3 border-t border-neutral-700/50">
      <input class="glass-input flex-1 !bg-neutral-600/50 !border-neutral-500/70 !text-gray-100" bind:value={newStageType} placeholder="New Stage Type" />
      <input class="glass-input flex-1 !bg-neutral-600/50 !border-neutral-500/70 !text-gray-100" bind:value={newCommand} placeholder="Command (optional)" />
      <Button variant="primary" customClass="!px-3 !py-1.5" on:click={addStage}>Add Stage</Button>
    </div>

    <div class="flex items-center justify-between mt-6">
      {#if pipeline.id}
        <Button variant="ghost" customClass="text-red-500 hover:text-red-400" on:click={deletePipeline}>Delete</Button>
      {/if}
      <div class="ml-auto space-x-2">
        <Button variant="secondary" on:click={() => dispatch('cancel')}>Cancel</Button>
        <Button variant="primary" on:click={savePipeline}>Save</Button>
      </div>
    </div>
  </div>
</div>
