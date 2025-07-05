<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { apiFetch } from '$lib/utils/apiUtils';

  export let orgId: string;
  export let userId: string;
  export let pipelineId: string | null = null; // This seems to be 'selectedPipeline' in the prompt, renamed for clarity.

  let isTarget: boolean = false; // Default to "Source" document

  const dispatch = createEventDispatcher();

  async function handleUpload(e: Event) {
    const input = e.target as HTMLInputElement;
    if (!input.files?.length) return;
    const file = input.files[0];
    const formData = new FormData(); // Renamed to avoid conflict with form element if one existed
    formData.append('file', file);

    const params = new URLSearchParams({
      org_id: orgId,
      owner_id: userId,
      is_target: isTarget.toString(), // Add is_target to query params
    });

    if (pipelineId) { // Use the prop pipelineId
      params.append('pipeline_id', pipelineId);
    }

    try {
      await apiFetch(`/api/upload?${params.toString()}`, {
        method: 'POST',
        body: formData,
        isFormData: true
      });
      dispatch('uploaded');
      input.value = '';
      isTarget = false;
    } catch (error) {
      console.error('Upload error:', error);
      alert('Upload failed. See console for details.');
    }
  }
</script>

<div class="space-y-3 p-1">
  <div>
    <input
      class="glass-input w-full text-sm file:mr-4 file:py-2 file:px-4 file:rounded-full file:border-0 file:text-sm file:font-semibold file:bg-accent/20 file:text-accent hover:file:bg-accent/30"
      type="file"
      accept="application/pdf,.md,.txt"
      on:change={handleUpload}
    />
    <p class="mt-1 text-xs text-gray-500">PDF, Markdown, or TXT files accepted.</p>
  </div>
  <div class="mt-2">
    <label class="flex items-center space-x-2 cursor-pointer">
      <input
        type="checkbox"
        bind:checked={isTarget}
        class="h-4 w-4 text-accent rounded border-gray-300 focus:ring-accent/50 bg-white/30"
      />
      <span class="text-sm text-gray-700">Is Target Document? (for analysis pipelines)</span>
    </label>
     {#if pipelineId}
      <p class="mt-1 text-xs text-gray-500">
        This document will be processed by the selected pipeline: '{pipelineId}'.
      </p>
    {/if}
    {#if isTarget && !pipelineId}
       <p class="mt-1 text-xs text-orange-600">
        Warning: Target documents are usually processed by a pipeline. No pipeline selected.
      </p>
    {/if}
  </div>
</div>
