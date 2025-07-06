<script lang="ts">
  import { onMount, createEventDispatcher } from 'svelte';
  import { sessionStore } from '$lib/stores/session';
  import GlassCard from '$lib/components/GlassCard.svelte';
  import Button from '$lib/components/Button.svelte';
import { apiFetch } from '$lib/utils/apiUtils';
import { errorStore } from '$lib/utils/errorStore';

  interface Header { id: string; name: string; value: string; }

  let ai_api_endpoint = '';
  let ai_api_key = '';
  let ocr_api_endpoint = '';
  let ocr_api_key = '';
  let ai_custom_headers: Header[] = [];

  const dispatch = createEventDispatcher();

  let orgId: string | null = null;
  $: orgId = $sessionStore.org;

  onMount(async () => {
    if (!orgId) return;
    try {
      const res = await apiFetch(`/api/settings/${orgId}`);
      if (res.ok) {
        const data = await res.json();
        ai_api_endpoint = data.ai_api_endpoint || '';
        ai_api_key = data.ai_api_key || '';
        ocr_api_endpoint = data.ocr_api_endpoint || '';
        ocr_api_key = data.ocr_api_key || '';
        ai_custom_headers = Array.isArray(data.ai_custom_headers)
          ? data.ai_custom_headers.map((h: any, i: number) => ({ id: `hdr-${i}`, name: h.name || '', value: h.value || '' }))
          : [];
      }
    } catch (e) {
      console.error('Failed to load settings', e);
    }
  });

  function addHeader() {
    ai_custom_headers = [...ai_custom_headers, { id: `new-${Date.now()}`, name: '', value: '' }];
  }

  function removeHeader(id: string) {
    ai_custom_headers = ai_custom_headers.filter(h => h.id !== id);
  }

  async function save() {
    if (!orgId) return;
    const payload = {
      ai_api_endpoint: ai_api_endpoint || null,
      ai_api_key: ai_api_key || null,
      ocr_api_endpoint: ocr_api_endpoint || null,
      ocr_api_key: ocr_api_key || null,
      ai_custom_headers: ai_custom_headers.map(h => ({ name: h.name, value: h.value }))
    };
    try {
      const res = await apiFetch(`/api/settings/${orgId}`, {
        method: 'PUT',
        body: JSON.stringify(payload)
      });
      if (res.ok) {
        dispatch('settingsUpdated');
        alert('Settings saved.');
      } else {
        errorStore.show('Failed to save settings: ' + (await res.text()));
      }
    } catch (e: any) {
      errorStore.show('Error saving settings: ' + e.message);
    }
  }
</script>

<div class="space-y-6">
  <h1 class="text-3xl font-semibold text-gray-100">Organization Settings</h1>

  <GlassCard title="AI Configuration" padding="p-4" bgOpacity="!bg-neutral-700/30" borderStyle="!border-neutral-600/50">
    <div class="space-y-4 p-2">
      <label class="block">
        <span class="text-sm font-medium text-gray-300">AI API Endpoint</span>
        <input type="text" class="glass-input w-full mt-1 !bg-neutral-600/50 !border-neutral-500/70 !text-gray-100" bind:value={ai_api_endpoint} placeholder="https://openrouter.ai/api/v1/chat/completions" />
      </label>
      <label class="block">
        <span class="text-sm font-medium text-gray-300">AI API Key</span>
        <input type="password" class="glass-input w-full mt-1 !bg-neutral-600/50 !border-neutral-500/70 !text-gray-100" bind:value={ai_api_key} />
      </label>
      <div class="mt-4 pt-3 border-t border-neutral-600/70">
        <h4 class="text-sm font-semibold text-gray-200 mb-2">Custom AI HTTP Headers</h4>
        {#each ai_custom_headers as header (header.id)}
          <div class="flex items-center space-x-2 mb-2">
            <input type="text" bind:value={header.name} class="glass-input flex-1 text-sm !bg-neutral-700/60 !border-neutral-600 !text-gray-100" placeholder="Header Name" />
            <input type="text" bind:value={header.value} class="glass-input flex-1 text-sm !bg-neutral-700/60 !border-neutral-600 !text-gray-100" placeholder="Header Value" />
            <Button variant="ghost" customClass="!px-1.5 !py-1 text-xs !text-red-400 hover:!text-red-300 hover:!bg-red-500/10" on:click={() => removeHeader(header.id)}>
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="2" stroke="currentColor" class="w-4 h-4"><path stroke-linecap="round" stroke-linejoin="round" d="M6 18L18 6M6 6l12 12" /></svg>
            </Button>
          </div>
        {/each}
        <Button variant="secondary" customClass="text-xs mt-2 !py-1 !px-2" on:click={addHeader}>Add Header</Button>
      </div>
    </div>
  </GlassCard>

  <GlassCard title="OCR Configuration" padding="p-4" bgOpacity="!bg-neutral-700/30" borderStyle="!border-neutral-600/50">
    <div class="space-y-4 p-2">
      <label class="block">
        <span class="text-sm font-medium text-gray-300">OCR API Endpoint</span>
        <input type="text" class="glass-input w-full mt-1 !bg-neutral-600/50 !border-neutral-500/70 !text-gray-100" bind:value={ocr_api_endpoint} />
      </label>
      <label class="block">
        <span class="text-sm font-medium text-gray-300">OCR API Key</span>
        <input type="password" class="glass-input w-full mt-1 !bg-neutral-600/50 !border-neutral-500/70 !text-gray-100" bind:value={ocr_api_key} />
      </label>
    </div>
  </GlassCard>

  <div class="text-right">
    <Button variant="primary" on:click={save}>Save Settings</Button>
  </div>
</div>
