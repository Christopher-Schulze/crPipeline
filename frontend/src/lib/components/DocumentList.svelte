<script lang="ts">
import GlassCard from './GlassCard.svelte';
export let docs: { id: string; filename: string }[] = [];

async function download(id: string) {
  const res = await fetch(`/api/download/${id}`, { credentials: 'include' });
  if (res.ok) {
    const { url } = await res.json();
    window.open(url, '_blank');
  }
}
</script>

<ul class="space-y-2">
  {#each docs as doc}
    <GlassCard depth={1} class="p-4 flex justify-between">
      <span>{doc.filename}</span>
      <a
        class="text-accent underline cursor-pointer"
        on:click={() => download(doc.id)}
        >Download</a>
    </GlassCard>
  {/each}
</ul>
