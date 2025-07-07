<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import Modal from './Modal.svelte';
  import Button from './Button.svelte';
  import { apiFetch } from '$lib/utils/apiUtils';

  export interface Org {
    id: string;
    name: string;
    api_key: string;
  }

  export let isOpen = false;
  export let org: Org | null = null;

  const dispatch = createEventDispatcher();

  let name = '';
  let errorMsg: string | null = null;
  let isLoading = false;

  $: if (isOpen && org) {
    name = org.name;
    errorMsg = null;
  }

  async function save() {
    if (!org) return;
    const trimmed = name.trim();
    if (!trimmed) {
      errorMsg = 'Organization name cannot be empty.';
      return;
    }
    isLoading = true;
    try {
      const res = await apiFetch(`/api/orgs/${org.id}`, {
        method: 'PUT',
        body: JSON.stringify({ name: trimmed })
      });
      if (!res.ok) {
        const data = await res.json().catch(() => ({}));
        throw new Error(data.error || 'Failed to update organization');
      }
      dispatch('updated');
      dispatch('close');
    } catch (e: any) {
      errorMsg = e.message;
    } finally {
      isLoading = false;
    }
  }

  function close() {
    dispatch('close');
  }
</script>

<Modal {isOpen} title="Edit Organization" on:close={close} maxWidth="max-w-md">
  <div slot="content" class="space-y-4 py-2">
    {#if errorMsg}
      <p class="text-sm text-error bg-error/20 p-2 rounded-md">{errorMsg}</p>
    {/if}
    <div>
      <label for="org-name" class="block text-sm font-medium text-gray-300 mb-1">Organization Name</label>
      <input id="org-name" type="text" bind:value={name} class="glass-input w-full" required />
    </div>
  </div>
  <div slot="footer" class="flex justify-end space-x-2">
    <Button variant="secondary" on:click={close} disabled={isLoading}>Cancel</Button>
    <Button variant="primary" on:click={save} disabled={isLoading}>{isLoading ? 'Saving...' : 'Save'}</Button>
  </div>
</Modal>
