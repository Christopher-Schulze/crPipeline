<script lang="ts">
  import type { Stage } from '$lib/types/api';
  export let stage: Stage;
  export let onChange: () => void = () => {};
</script>

<div class="form-group mt-2 pt-2 border-t border-neutral-700/50 space-y-2">
  <div>
    <label for={`stage-ocr-engine-${stage.id}`} class="block text-xs font-light text-gray-300 mb-1">OCR Engine</label>
    <select
      bind:value={stage.ocr_engine}
      id={`stage-ocr-engine-${stage.id}`}
      on:change={onChange}
      class="glass-input w-full text-sm !bg-neutral-600/50 !border-neutral-500/70 !text-gray-100"
    >
      <option value="default">Default (Tesseract/Local)</option>
      <option value="external">External API</option>
    </select>
  </div>
  {#if stage.ocr_engine === 'external'}
    <div class="mt-2 space-y-2 pl-2 border-l-2 border-neutral-700/40 ml-1">
      <div class="pt-1">
        <label for={`stage-ocr-endpoint-${stage.id}`} class="block text-xs font-light text-gray-300 mb-1" title="Endpoint for external OCR service">
          Stage OCR API Endpoint
        </label>
        <input
          type="text"
          id={`stage-ocr-endpoint-${stage.id}`}
          bind:value={stage.ocr_stage_endpoint}
          on:input={onChange}
          class="glass-input w-full text-sm !bg-neutral-600/50 !border-neutral-500/70 !text-gray-100"
          title="Leave empty to use the global OCR endpoint"
          placeholder="Overrides global OCR endpoint"
        />
      </div>
      <div>
        <label for={`stage-ocr-key-${stage.id}`} class="block text-xs font-light text-gray-300 mb-1" title="API key for the external OCR service">
          Stage OCR API Key
        </label>
        <input
          type="password"
          id={`stage-ocr-key-${stage.id}`}
          bind:value={stage.ocr_stage_key}
          on:input={onChange}
          class="glass-input w-full text-sm !bg-neutral-600/50 !border-neutral-500/70 !text-gray-100"
          title="Leave empty to use the global OCR key"
          placeholder="Overrides global OCR key"
        />
      </div>
    </div>
  {/if}
</div>
