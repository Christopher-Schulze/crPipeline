<script lang="ts">
  import { createEventDispatcher, onMount } from 'svelte';
  import Button from './Button.svelte';
  import GlassCard from './GlassCard.svelte';
  import ConfirmationModal from './ConfirmationModal.svelte';
import { apiFetch } from '$lib/utils/apiUtils'; // Import apiFetch
import { errorStore } from '$lib/utils/errorStore';

  export let orgId: string;

  // Define interfaces for settings
  interface PromptTemplate {
    id?: string;
    name: string;
    text: string;
  }

  interface Header {
    id: string; // Client-side unique ID for {#each} key
    name: string;
    value: string;
  }

  interface OrgSettings {
    org_id: string;
    monthly_upload_quota: number;
    monthly_analysis_quota: number;
    accent_color: string;
    ai_api_endpoint?: string | null;
    ai_api_key?: string | null;
    ocr_api_endpoint?: string | null;
    ocr_api_key?: string | null;
    prompt_templates?: PromptTemplate[] | null;
    ai_custom_headers?: Header[] | null; // New field
  }

  let settings: OrgSettings = {
    org_id: orgId, // Initialize with passed orgId
    monthly_upload_quota: 100,
    monthly_analysis_quota: 100,
    accent_color: '#30D5C8',
    ai_api_endpoint: null,
    ai_api_key: null,
    ocr_api_endpoint: null,
    ocr_api_key: null,
    prompt_templates: [],
    ai_custom_headers: [], // Initialize new field
  };

  const dispatch = createEventDispatcher();

  async function loadSettings() {
    if (!orgId) return;
    try {
      // Use apiFetch for GET requests too (ensures credentials, etc.)
      const res = await apiFetch(`/api/settings/${orgId}`);
      if (res.ok) {
        const data = await res.json();
        settings = {
          ...settings,
          ...data,
          prompt_templates: (data.prompt_templates && Array.isArray(data.prompt_templates))
            ? data.prompt_templates.map((pt: any, i: number) => ({
                id: `pt-${Date.now()}-${i}`,
                name: pt.name || '',
                text: pt.text || ''
              }))
            : [],
          ai_custom_headers: (data.ai_custom_headers && Array.isArray(data.ai_custom_headers))
            ? data.ai_custom_headers.map((h: any, i: number) => ({
                id: `header-${Date.now()}-${i}`,
                name: h.name || '',
                value: h.value || ''
              }))
            : [],
        };
      } else {
        console.error('Failed to load settings:', await res.text());
      }
    } catch (error) {
      console.error('Error loading settings:', error);
    }
  }

  onMount(loadSettings);

  async function saveSettings() {
    // Ensure numbers are correctly formatted if they were bound to text inputs that became strings
    settings.monthly_upload_quota = +settings.monthly_upload_quota;
    settings.monthly_analysis_quota = +settings.monthly_analysis_quota;

    // Prepare payload, stripping client-side IDs from headers
    const payloadForBackend = { ...settings };
    if (payloadForBackend.ai_custom_headers) {
      payloadForBackend.ai_custom_headers = payloadForBackend.ai_custom_headers.map(h => ({
        name: h.name,
        value: h.value
      }));
    }
    // Ensure prompt_templates also strip client-side 'id' if it was added during editing
    // (though current PromptTemplate interface doesn't enforce 'id' to be sent to backend)
    if (payloadForBackend.prompt_templates) {
        payloadForBackend.prompt_templates = payloadForBackend.prompt_templates.map(pt => ({
            name: pt.name,
            text: pt.text
        }));
    }


    try {
      const res = await apiFetch('/api/settings', { // Use apiFetch here
        method: 'POST',
        // headers: { 'Content-Type': 'application/json' }, // apiFetch handles this
        body: JSON.stringify(payloadForBackend)
      });
      if (res.ok) {
        const updatedSettingsFromServer = await res.json();
        // Re-map headers and prompts from server to add client-side IDs for UI consistency
        settings = {
          ...settings, // Keep current client state for potentially unsaved parts
          ...updatedSettingsFromServer,
          prompt_templates: (updatedSettingsFromServer.prompt_templates && Array.isArray(updatedSettingsFromServer.prompt_templates))
            ? updatedSettingsFromServer.prompt_templates.map((pt: any, i: number) => ({
                id: `pt-reloaded-${Date.now()}-${i}`,
                name: pt.name || '',
                text: pt.text || ''
              }))
            : [],
          ai_custom_headers: (updatedSettingsFromServer.ai_custom_headers && Array.isArray(updatedSettingsFromServer.ai_custom_headers))
            ? updatedSettingsFromServer.ai_custom_headers.map((h: any, i: number) => ({
                id: `header-reloaded-${Date.now()}-${i}`,
                name: h.name || '',
                value: h.value || ''
              }))
            : [],
        };
        dispatch('saved', { accentColor: settings.accent_color });
        alert('Settings saved successfully!');
      } else {
        errorStore.show('Failed to save settings: ' + (await res.text()));
      }
    } catch (error) {
      console.error('Error saving settings:', error);
      errorStore.show('Error saving settings.');
    }
  }

  function addPromptTemplate() {
    if (!settings.prompt_templates) {
      settings.prompt_templates = [];
    }
    // Add a unique temporary ID for Svelte's keyed {#each} block
    const newId = `temp_${Date.now()}_${Math.random().toString(36).substring(2, 9)}`;
    settings.prompt_templates = [
      ...settings.prompt_templates,
      { id: newId, name: '', text: '' },
    ];
  }

  function removePromptTemplate(templateIdToRemove: string | undefined, indexToRemove: number) {
     if (templateIdToRemove) {
        settings.prompt_templates = settings.prompt_templates?.filter(pt => pt.id !== templateIdToRemove) || [];
     } else {
        settings.prompt_templates = settings.prompt_templates?.filter((_, i) => i !== indexToRemove) || [];
     }
  }

  function addAiCustomHeader() {
    if (!settings.ai_custom_headers) {
      settings.ai_custom_headers = [];
    }
    settings.ai_custom_headers = [
      ...settings.ai_custom_headers,
      { id: `header-new-${Date.now()}-${Math.random().toString(36).substring(2,9)}`, name: '', value: '' }
    ];
  }

  function removeAiCustomHeader(headerId: string) {
    if (settings.ai_custom_headers) {
      settings.ai_custom_headers = settings.ai_custom_headers.filter(h => h.id !== headerId);
    }
  }

  // --- Confirmation Modal State & Logic ---
  let showConfirmationModal = false;
  let confirmationTitle = '';
  let confirmationMessage = '';
  let confirmAction: (() => void) | null = null;
  let confirmButtonText = 'Confirm';
  let confirmVariant: 'primary' | 'danger' = 'primary';

  function requestRemovePromptTemplate(index: number, template: PromptTemplate) {
    confirmationTitle = 'Remove Prompt Template';
    // Use template.id for a stable reference if available and actually unique, otherwise index is fine for client-side list.
    // The current removePromptTemplate uses index if id is not found.
    confirmationMessage = `Are you sure you want to remove the prompt template "${template.name || 'Unnamed Template'}"? This action cannot be undone.`;
    confirmButtonText = 'Remove Template';
    confirmVariant = 'danger';
    confirmAction = () => {
      // Pass both id (if exists) and index to be robust
      removePromptTemplate(template.id, index);
    };
    showConfirmationModal = true;
  }

  function requestRemoveAiCustomHeader(headerId: string, headerName: string) {
    confirmationTitle = 'Remove Custom Header';
    confirmationMessage = `Are you sure you want to remove the custom AI header "${headerName || 'Unnamed Header'}"?`;
    confirmButtonText = 'Remove Header';
    confirmVariant = 'danger';
    confirmAction = () => {
      removeAiCustomHeader(headerId);
    };
    showConfirmationModal = true;
  }

  function handleConfirmation() {
    if (confirmAction) {
      confirmAction();
    }
    closeConfirmationModal();
  }

  function closeConfirmationModal() {
    showConfirmationModal = false;
    confirmAction = null; // Reset action
  }

</script>

<div class="space-y-6">
  <GlassCard title="Quotas & Appearance" padding="p-4" titleClass="text-xl font-semibold text-gray-100 mb-3" bgOpacity="!bg-neutral-700/30" borderStyle="!border-neutral-600/50">
    <div class="space-y-4 p-2">
      <label class="block">
        <span class="text-sm font-medium text-gray-300">Monthly Upload Quota</span>
        <input type="number" min="0" class="glass-input w-full mt-1 !bg-neutral-600/50 !border-neutral-500/70 !text-gray-100" bind:value={settings.monthly_upload_quota} />
      </label>
      <label class="block">
        <span class="text-sm font-medium text-gray-300">Monthly Analysis Quota</span>
        <input type="number" min="0" class="glass-input w-full mt-1 !bg-neutral-600/50 !border-neutral-500/70 !text-gray-100" bind:value={settings.monthly_analysis_quota} />
      </label>
      <label class="block">
        <span class="text-sm font-medium text-gray-300">Accent Color</span>
        <input class="glass-input mt-1 !bg-neutral-600/50 !border-neutral-500/70" type="color" bind:value={settings.accent_color} />
      </label>
    </div>
  </GlassCard>

  <GlassCard title="AI Configuration" padding="p-4" titleClass="text-xl font-semibold text-gray-100 mb-3" bgOpacity="!bg-neutral-700/30" borderStyle="!border-neutral-600/50">
    <div class="space-y-4 p-2">
      <div>
        <label for="ai_api_endpoint" class="block text-sm font-medium text-gray-300">AI API Endpoint</label>
        <input type="text" id="ai_api_endpoint" class="glass-input w-full mt-1 !bg-neutral-600/50 !border-neutral-500/70 !text-gray-100" placeholder="e.g., https://openrouter.ai/api/v1/chat/completions" bind:value={settings.ai_api_endpoint} />
        <p class="text-sm font-light text-gray-400 dark:text-gray-500 mt-0.5">Compatible with OpenRouter.ai and other OpenAI-compatible APIs.</p>
      </div>
      <div>
        <label for="ai_api_key" class="block text-sm font-medium text-gray-300">AI API Key</label>
          <input
            type="password"
            id="ai_api_key"
            bind:value={settings.ai_api_key}
            class="glass-input w-full mt-1 !bg-neutral-600/50 !border-neutral-500/70 !text-gray-100"
            placeholder={settings.ai_api_key === "********" ? "Key is set. Type to change." : "Enter API Key (e.g., OpenRouter Key)"}
          />
        <p class="text-sm font-light text-gray-400 dark:text-gray-500 mt-1">
          If a key shows '********', it means a key is currently set.
          To change it, type the new key. To clear it, delete all text and save.
          Saving with '********' visible will be ignored by the backend to preserve the original key.
        </p>
      </div>

      <div class="mt-4 pt-3 border-t border-neutral-600/70"> <!-- Adjusted border color -->
        <h4 class="text-sm font-semibold text-gray-200 mb-2">Custom AI HTTP Headers (Optional)</h4>
        {#if settings.ai_custom_headers && settings.ai_custom_headers.length > 0}
          <div class="space-y-2">
            {#each settings.ai_custom_headers as header (header.id)}
              <div class="flex items-center space-x-2 p-2 bg-black/20 rounded-md border border-neutral-600/50"> <!-- Darker bg and border for items -->
                <input
                  type="text"
                  bind:value={header.name}
                  placeholder="Header Name (e.g., HTTP-Referer)"
                  class="glass-input flex-1 text-sm !bg-neutral-700/60 !border-neutral-600 !text-gray-100"
                />
                <input
                  type="text"
                  bind:value={header.value}
                  placeholder="Header Value"
                  class="glass-input flex-1 text-sm !bg-neutral-700/60 !border-neutral-600 !text-gray-100"
                />
                <Button variant="ghost" customClass="!px-1.5 !py-1 text-xs !text-error hover:!text-error-content hover:!bg-error/10" on:click={() => requestRemoveAiCustomHeader(header.id, header.name)}>
                  <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="2" stroke="currentColor" class="w-4 h-4"><path stroke-linecap="round" stroke-linejoin="round" d="M6 18L18 6M6 6l12 12" /></svg>
                </Button>
              </div>
            {/each}
          </div>
        {/if}
        <Button variant="secondary" customClass="text-xs mt-2 !py-1 !px-2" on:click={addAiCustomHeader}>
          Add Custom Header
        </Button>
        <p class="text-sm font-light text-gray-400 dark:text-gray-500 mt-1">e.g., for `X-Title` or `HTTP-Referer` used by some OpenRouter models.</p>
      </div>
    </div>
  </GlassCard>

  <GlassCard title="OCR Configuration (Future Use)" padding="p-4" titleClass="text-xl font-semibold text-gray-100 mb-3" bgOpacity="!bg-neutral-700/30" borderStyle="!border-neutral-600/50">
     <div class="space-y-4 p-2">
      <label class="block">
        <span class="text-sm font-medium text-gray-300">OCR API Endpoint</span>
        <input type="text" class="glass-input w-full mt-1 !bg-neutral-600/50 !border-neutral-500/70 !text-gray-100" placeholder="OCR API Endpoint" bind:value={settings.ocr_api_endpoint} />
      </label>
      <label class="block">
        <span class="text-sm font-medium text-gray-300">OCR API Key</span>
        <input
          type="password"
          id="ocr_api_key"
          bind:value={settings.ocr_api_key}
          class="glass-input w-full mt-1 !bg-neutral-600/50 !border-neutral-500/70 !text-gray-100"
          placeholder={settings.ocr_api_key === "********" ? "Key is set (masked). Type to change." : "Enter OCR API Key"}
        />
        <p class="text-sm font-light text-gray-400 dark:text-gray-500 mt-1">
          If a key shows '********', it means a key is currently set.
          To change it, type the new key. To clear it, delete all text from the field and save.
          Saving with '********' visible in the field will be ignored by the backend to preserve the original key.
        </p>
      </label>
    </div>
  </GlassCard>

  <GlassCard title="Prompt Templates" padding="p-4" titleClass="text-xl font-semibold text-gray-100 mb-3" bgOpacity="!bg-neutral-700/30" borderStyle="!border-neutral-600/50">
    <div class="space-y-4 p-2">
      {#if settings.prompt_templates && settings.prompt_templates.length > 0}
        {#each settings.prompt_templates as template, index (template.id || index)}
          <div class="p-3 border border-neutral-600/50 rounded-lg bg-black/20 space-y-2"> <!-- Darker item bg and border -->
            <label class="block">
              <span class="text-sm font-medium text-gray-300">Template Name</span>
              <input type="text" class="glass-input w-full mt-1 !bg-neutral-600/50 !border-neutral-500/70 !text-gray-100" placeholder="e.g., Document Summary" bind:value={template.name} />
            </label>
            <label class="block">
              <span class="text-sm font-medium text-gray-300">Template Text</span>
              <textarea class="glass-input w-full h-24 mt-1 !bg-neutral-600/50 !border-neutral-500/70 !text-gray-100" placeholder="Enter prompt text. Use {{content}} or {{json_input}} for previous stage data." bind:value={template.text}></textarea>
            </label>
            <div class="text-right">
              <Button variant="ghost" on:click={() => requestRemovePromptTemplate(index, template)} customClass="text-error hover:text-error-content hover:bg-error/10 !px-2 !py-1 text-xs">Remove</Button>
            </div>
          </div>
        {/each}
      {/if}
      <Button variant="secondary" on:click={addPromptTemplate} customClass="text-sm">Add Prompt Template</Button>
    </div>
  </GlassCard>

  <div class="mt-6">
    <Button variant="primary" on:click={saveSettings} customClass="w-full md:w-auto">Save All Settings</Button>
  </div>

  <ConfirmationModal
    isOpen={showConfirmationModal}
    title={confirmationTitle}
    message={confirmationMessage}
    confirmText={confirmButtonText}
    confirmButtonVariant={confirmVariant}
    on:confirm={handleConfirmation}
    on:cancel={closeConfirmationModal}
  />
</div>
