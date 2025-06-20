<script lang="ts">
  import { onDestroy } from 'svelte';
  import GlassCard from './GlassCard.svelte';

  export let jobs: { id: string; status: string }[] = [];

  let sources: EventSource[] = [];

  $: if (jobs.length) {
    sources.forEach((s) => s.close());
    sources = jobs.map((job) => {
      const es = new EventSource(`/api/jobs/${job.id}/events`);
      es.onmessage = (e) => {
        job.status = e.data;
        if (e.data === 'completed' || e.data === 'failed') {
          es.close();
        }
      };
      return es;
    });
  }

  onDestroy(() => {
    sources.forEach((s) => s.close());
  });
</script>

<ul class="space-y-2">
  {#each jobs as job}
    <GlassCard depth={1} class="p-4 flex justify-between">
      <span>{job.id}</span>
      <span class="font-medium" class:!text-success={job.status==='completed'} class:!text-error={job.status==='failed'}>{job.status}</span>
    </GlassCard>
  {/each}
</ul>
