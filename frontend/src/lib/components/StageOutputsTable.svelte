<script lang="ts">
  import Button from './Button.svelte';
  export interface StageOutput {
    id: string;
    job_id: string;
    stage_name: string;
    output_type: string;
    s3_bucket: string;
    s3_key: string;
    created_at: string;
  }
  export let outputs: StageOutput[] = [];
  export let documentName: string = '';
  export let jobId: string = '';
  export let onView: (output: StageOutput) => void;
  export let onDownload: (id: string, filename: string) => void;
  function download(output: StageOutput) {
    let baseName = documentName && documentName.trim() !== '' ? documentName : `job_${jobId}`;
    baseName = baseName.replace(/\.[^/.]+$/, '');
    let suggestedFilename = `${baseName}_${output.stage_name}.${output.output_type}`;
    suggestedFilename = suggestedFilename.replace(/[\s\\/:*?"<>|]+/g, '_').replace(/__+/g, '_');
    onDownload(output.id, suggestedFilename);
  }
</script>
<section>
  <h3 class="text-lg font-semibold mb-2 text-gray-200">All Stage Outputs</h3>
  {#if outputs && outputs.length > 0}
    <div class="space-y-3">
      {#each outputs as output (output.id)}
        <div class="p-3 bg-black/20 rounded-md flex justify-between items-center border border-neutral-700/50 hover:border-neutral-600/80 transition-colors">
          <div class="truncate pr-2">
            <span class="font-semibold text-gray-200 truncate block" title={output.stage_name}>{output.stage_name}</span>
            <span class="ml-1 px-2 py-0.5 inline-flex text-xs leading-4 font-semibold rounded-full bg-neutral-600/60 text-neutral-300">
              {output.output_type}
            </span>
          </div>
          <div class="flex items-center space-x-2 flex-shrink-0">
            {#if output.output_type === 'txt' || output.output_type === 'json'}
              <Button variant="ghost" customClass="text-xs !py-1 !px-2 !text-sky-400 hover:!text-sky-300 hover:!bg-sky-500/10" on:click={() => onView(output)}>
                View (Modal)
              </Button>
            {/if}
            <Button variant="secondary" customClass="text-xs !py-1 !px-2" on:click={() => download(output)} title={`S3 Path: s3://${output.s3_bucket}/${output.s3_key}`}>Download</Button>
          </div>
        </div>
      {/each}
    </div>
  {:else}
    <div class="p-4 bg-black/20 rounded-md text-center border border-white/10">
      <p class="text-gray-400">No individual stage outputs recorded for this job.</p>
    </div>
  {/if}
</section>
