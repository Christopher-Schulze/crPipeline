<script lang="ts">
  import { createEventDispatcher } from 'svelte';

  export let orgId: string;
  export let userId: string;
  export let pipelineId: string | null = null;

  const dispatch = createEventDispatcher();

  async function handleUpload(e: Event) {
    const input = e.target as HTMLInputElement;
    if (!input.files?.length) return;
    const file = input.files[0];
    const form = new FormData();
    form.append('file', file);
    const params = new URLSearchParams({
      org_id: orgId,
      owner_id: userId,
    });
    if (pipelineId) params.append('pipeline_id', pipelineId);
    const res = await fetch(`/api/upload?${params.toString()}`, { method: 'POST', body: form });
    if (res.ok) {
      dispatch('uploaded');
    }
  }
</script>

<div class="space-y-2">
  <input class="glass-input w-full" type="file" accept="application/pdf" on:change={handleUpload} />
</div>
