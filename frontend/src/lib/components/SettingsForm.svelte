<script lang="ts">
  import { createEventDispatcher, onMount } from 'svelte';
  import Button from './Button.svelte';

  export let orgId: string;
  let monthlyUploadQuota = 100;
  let monthlyAnalysisQuota = 100;
  let accentColor = '#30D5C8';
  const dispatch = createEventDispatcher();

  onMount(async () => {
    const res = await fetch(`/api/settings/${orgId}`);
    if (res.ok) {
      const data = await res.json();
      monthlyUploadQuota = data.monthly_upload_quota;
      monthlyAnalysisQuota = data.monthly_analysis_quota;
      accentColor = data.accent_color;
    }
  });

  async function save() {
    const res = await fetch('/api/settings', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        org_id: orgId,
        monthly_upload_quota: +monthlyUploadQuota,
        monthly_analysis_quota: +monthlyAnalysisQuota,
        accent_color: accentColor
      })
    });
    if (res.ok) {
      dispatch('saved', { accentColor });
    }
  }
</script>

<div class="space-y-4">
  <label class="block">Monthly upload quota
    <input type="number" min="0" class="glass-input w-full" bind:value={monthlyUploadQuota} />
  </label>
  <label class="block">Monthly analysis quota
    <input type="number" min="0" class="glass-input w-full" bind:value={monthlyAnalysisQuota} />
  </label>
  <label class="block">Accent color
    <input class="glass-input" type="color" bind:value={accentColor} />
  </label>
  <Button on:click={save}>Save</Button>
</div>
