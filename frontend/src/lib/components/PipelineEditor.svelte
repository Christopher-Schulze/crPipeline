<script lang="ts">
  import Button from './Button.svelte';
  import StageList from './pipeline_editor/StageList.svelte';
  import { onMount, createEventDispatcher } from 'svelte';
import { apiFetch } from '$lib/utils/apiUtils';
import { errorStore } from '$lib/utils/errorStore';
  import type { Stage, Pipeline } from '$lib/types/api';
  import type { EditorPromptTemplate, RegexPatternConfig } from './pipeline_editor/types';

  export let orgId: string;
  export let initialPipeline: Pipeline | null = null;

  // Internal reactive state for the pipeline being edited
  let pipeline: Pipeline = {
    id: undefined,
    org_id: orgId,
    name: '',
    stages: [],
  };

  function resetPipeline() {
    pipeline = {
      id: undefined,
      org_id: orgId,
      name: '',
      stages: [],
    };
  }

  // State for available prompt templates
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
      if (typeof stage.config.parameters._delimiterRegex === 'undefined') {
        stage.config.parameters._delimiterRegex = stage.config.parameters.delimiterRegex || '';
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
  $: if (initialPipeline && pipeline && initialPipeline.id !== pipeline.id) { // Check if it's a different pipeline
    loadPipelineFromProp(initialPipeline);
  } else if (!initialPipeline && pipeline && (pipeline.id || pipeline.name || pipeline.stages.length > 0) ) {
    resetPipeline();
  }

  let newStageType = '';
  let newCommand = '';

  function addStage() {
    if (newStageType.trim()) {
      const type = newStageType.trim().toLowerCase();
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
            delete stage.config.parameters._delimiterRegex;
            if (stage.config.parameters.stopKeywords && stage.config.parameters.stopKeywords.length === 0) {
                stage.config.parameters.stopKeywords = null;
            }
            if (!stage.config.parameters.delimiterRegex) {
                delete stage.config.parameters.delimiterRegex;
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
    if (!pipeline.name.trim()) {
      errorStore.show("Pipeline name is required.");
      return;
    }
    if (pipeline.stages.length === 0) {
      errorStore.show("Pipeline must have at least one stage.");
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
        dispatch('saved');
        document.body.dispatchEvent(new CustomEvent('pipelinesUpdated'));
      } else {
        const errorData = await response.json().catch(() => ({ error: "Unknown error during save." }));
        console.error('Failed to save pipeline:', errorData);
        errorStore.show(`Error saving pipeline: ${errorData.error || response.statusText}`);
      }
    } catch (e: any) {
      console.error('Network or other error saving pipeline:', e);
      errorStore.show(`Network error while saving pipeline: ${e.message}`);
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
        errorStore.show(`Error deleting pipeline: ${errorData.error || response.statusText}`);
      }
    } catch (e: any) {
      errorStore.show(`Network error while deleting pipeline: ${e.message}`);
    }
  }

  const dispatch = createEventDispatcher();
</script>

<style>
</style>

<div class="space-y-4 text-gray-200">
  <input class="input input-bordered w-full" bind:value={pipeline.name} placeholder="Pipeline name" />
  {#if promptTemplatesError}
    <div class="alert alert-error text-sm">
      {promptTemplatesError}
    </div>
  {/if}
  <div class="space-y-3">
    <StageList
      bind:stages={pipeline.stages}
      {availablePromptTemplates}
      {isLoadingOrgSettings}
    />
    <div class="flex gap-2 mt-3 p-3 border-t border-neutral-700/50">
      <input class="input input-bordered flex-1" bind:value={newStageType} placeholder="New Stage Type" />
      <input class="input input-bordered flex-1" bind:value={newCommand} placeholder="Command (optional)" />
      <Button variant="primary" customClass="!px-3 !py-1.5" on:click={addStage}>Add Stage</Button>
    </div>

    <div class="flex items-center justify-between mt-6">
      {#if pipeline.id}
        <Button variant="danger" customClass="!px-3 !py-1.5" on:click={deletePipeline}>Delete</Button>
      {/if}
      <div class="ml-auto space-x-2">
        <Button variant="secondary" on:click={() => dispatch('cancel')}>Cancel</Button>
        <Button variant="primary" on:click={savePipeline}>Save</Button>
      </div>
    </div>
  </div>
</div>
