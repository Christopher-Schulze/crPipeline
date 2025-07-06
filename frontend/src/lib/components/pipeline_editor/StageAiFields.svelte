<script lang="ts">
  import type { Stage } from '$lib/types/api';
  import type { EditorPromptTemplate } from './types';
  export let stage: Stage;
  export let availablePromptTemplates: EditorPromptTemplate[] = [];
  export let isLoadingOrgSettings: boolean = false;
  export let onChange: () => void = () => {};
</script>

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
      on:change={onChange}
      class="glass-input w-full text-sm !bg-neutral-600/50 !border-neutral-500/70 !text-gray-100"
    >
      <option value={undefined}>Default (No specific template)</option>
      {#each availablePromptTemplates as template (template.name)}
        <option value={template.name}>{template.name}</option>
      {/each}
    </select>
  {:else}
    <p class="text-sm font-light text-gray-400 dark:text-gray-500 py-2">
      No prompt templates defined. AI uses default behavior.
    </p>
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
