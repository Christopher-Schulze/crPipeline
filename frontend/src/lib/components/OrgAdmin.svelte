<script lang="ts">
  import GlassCard from './GlassCard.svelte';
  import Button from './Button.svelte';
  import { onMount } from 'svelte';


  interface Org { id: string; name: string; api_key: string; }

  let orgs: Org[] = [];
  let name = '';

  async function loadOrgs() {
    const res = await fetch('/api/orgs');
    if (res.ok) orgs = await res.json();
  }

  async function createOrg() {
    const res = await fetch('/api/orgs', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ name })
    });
    if (res.ok) {
      name = '';
      await loadOrgs();
    }
  }

  onMount(loadOrgs);
</script>

<GlassCard class="p-4 space-y-4" depth={2}>
  <h2 class="text-xl font-semibold">Organizations</h2>
  <div class="space-x-2">
    <input class="glass-input px-2 py-1" placeholder="Name" bind:value={name} />
    <Button on:click={createOrg}>Create</Button>
  </div>
  <table class="w-full text-left">
    <thead>
      <tr><th>Name</th><th>API Key</th></tr>
    </thead>
    <tbody>
      {#each orgs as o}
        <tr class="hover:bg-white/10">
          <td class="py-1">{o.name}</td>
          <td class="py-1 font-mono text-sm">{o.api_key}</td>
        </tr>
      {/each}
    </tbody>
  </table>
</GlassCard>
