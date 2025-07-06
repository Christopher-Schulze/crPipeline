<script lang="ts">
  import type { Stage } from '$lib/types/api';
  export let stage: Stage;
  export let onChange: () => void = () => {};
</script>

<div class="form-group mt-3 pt-3 border-t border-neutral-700/50 space-y-3">
  <div>
    <label for={`stage-report-template-${stage.id}`} class="block text-xs font-medium text-gray-300 mb-1">
      Report Markdown Template
    </label>
    <textarea
      id={`stage-report-template-${stage.id}`}
      bind:value={stage.config.template}
      on:input={onChange}
      rows={8}
      class="glass-input w-full !text-sm custom-scrollbar !bg-neutral-600/50 !border-neutral-500/70 !text-gray-100"
      placeholder="Enter Markdown template. Use \u007B\u007Bplaceholder.path\u007D\u007D for data. e.g., \u007B\u007Bdocument_name\u007D\u007D, \u007B\u007Bai_result.summary\u007D\u007D"
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
        onChange();
      }}
      class="glass-input w-full !text-sm !bg-neutral-600/50 !border-neutral-500/70 !text-gray-100"
      placeholder="e.g., previous_stage_output.field.name, ai_result.sentiment"
    />
    <p class="text-sm font-light text-gray-400 dark:text-gray-500 mt-1">
      Enter JSONPath expressions (e.g., `previous_stage_output.field.name`) relative to the data available to the report stage. Each path creates a key in the summary JSON.
    </p>
  </div>
</div>
