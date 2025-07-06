<script lang="ts">
  import { onMount, onDestroy, getContext } from 'svelte';
  import { sessionStore } from '$lib/stores/session';
  import DataTable, { type TableHeader } from '$lib/components/DataTable.svelte';
  import Button from '$lib/components/Button.svelte';
  import GlassCard from '$lib/components/GlassCard.svelte';
import { apiFetch } from '$lib/utils/apiUtils';
import { errorStore } from '$lib/utils/errorStore';
  import { fade } from 'svelte/transition';

  // Define Pipeline type consistent with PipelineEditor and backend
  // Assuming Pipeline type from +layout.svelte or a shared types definition
  // For now, defining it locally for clarity.
  interface Pipeline {
    id: string;
    name: string;
    org_id: string;
    stages: Array<{type: string, [key: string]: any}>; // Simple stage type
    created_at?: string; // Optional, if backend provides
    updated_at?: string; // Optional, if backend provides
  }

  let pipelines: Pipeline[] = [];
  let isLoading: boolean = true;
  let error: string | null = null;

  let orgId: string | null = null;
  let loggedIn = false;
  $: ({ orgId, loggedIn } = { orgId: $sessionStore.org, loggedIn: $sessionStore.loggedIn });

  // Get context function to open pipeline editor
  const managePipeline = getContext<(pipelineData?: Pipeline | null) => void>('managePipeline');

  async function loadPipelines() {
    if (!orgId) {
      error = "Organization context not available. Please ensure you are part of an organization.";
      isLoading = false;
      pipelines = []; // Clear any stale data
      return;
    }
    isLoading = true;
    error = null;
    try {
      const response = await apiFetch(`/api/pipelines/${orgId}`);

      if (!response.ok) {
        const errText = await response.text();
        const errData = JSON.parse(errText || "{}");
        throw new Error(errData.error || `Failed to fetch pipelines for organization ${orgId}: ${response.statusText} - ${errText}`);
      }
      pipelines = await response.json();
    } catch (e: any) {
      error = e.message;
      pipelines = [];
      console.error("Error loading pipelines:", e);
    } finally {
      isLoading = false;
    }
  }

  let pipelinesUpdatedHandler: () => void;

  onMount(() => {
    if (orgId) {
        loadPipelines();
    } else if (loggedIn) {
        // User is logged in but no orgId in session, which is unexpected for pipeline management.
        error = "Organization ID is missing from your session. Unable to load pipelines.";
        isLoading = false;
    } else {
        // Not logged in, message will be handled by the main conditional block in markup
        isLoading = false;
    }

    pipelinesUpdatedHandler = () => {
        loadPipelines();
    };
    document.body.addEventListener('pipelinesUpdated', pipelinesUpdatedHandler);
  });

  onDestroy(() => {
    if (pipelinesUpdatedHandler) {
        document.body.removeEventListener('pipelinesUpdated', pipelinesUpdatedHandler);
    }
  });

  const pipelineTableHeaders: TableHeader[] = [
    { key: 'name', label: 'Pipeline Name', sortable: true, cellClass: 'font-medium !text-gray-100 group-hover:!text-accent-lighter' },
    { key: 'stage_count', label: 'Stages', sortable: true, headerClass: 'text-center', cellClass: 'text-center !text-gray-300' },
    { key: 'created_at', label: 'Created', sortable: true, cellClass: '!text-gray-400 text-xs' },
    { key: 'updated_at', label: 'Last Modified', sortable: true, cellClass: '!text-gray-400 text-xs' },
    { key: 'actions', label: 'Actions', headerClass: 'text-right', cellClass: 'text-right', sortable: false }
  ];

  $: tablePipelines = pipelines.map(p => ({
      ...p,
      stage_count: p.stages?.length || 0,
      // Basic date formatting, can be enhanced with a utility
      created_at: p.created_at ? new Date(p.created_at).toLocaleDateString('en-CA') : 'N/A',
      updated_at: p.updated_at ? new Date(p.updated_at).toLocaleDateString('en-CA') : 'N/A',
  }));

  function handleEditPipeline(pipelineToEdit: Pipeline) {
      // Find the original pipeline object from `pipelines` to pass full data if needed
      const originalPipeline = pipelines.find(p => p.id === pipelineToEdit.id);
      if (managePipeline && originalPipeline) {
          managePipeline(originalPipeline);
      } else {
          console.error("Could not find original pipeline data or managePipeline context is unavailable.", pipelineToEdit);
          errorStore.show("Error: Could not initiate pipeline editing.");
      }
  }

  function handleCreateNewPipeline() {
      if (managePipeline) {
          managePipeline(null); // Pass null for new pipeline
      } else {
          console.error("managePipeline context is unavailable.");
          errorStore.show("Error: Could not open pipeline editor.");
      }
  }

  async function handleDeletePipeline(p: Pipeline) {
      if (!confirm('Delete this pipeline?')) return;
      try {
          const res = await apiFetch(`/api/pipelines/${p.id}`, { method: 'DELETE' });
          if (res.ok) {
              alert('Pipeline deleted.');
              document.body.dispatchEvent(new CustomEvent('pipelinesUpdated'));
          } else {
              const err = await res.json().catch(() => ({ error: res.statusText }));
              errorStore.show(`Error deleting pipeline: ${err.error}`);
          }
      } catch (e: any) {
          errorStore.show(`Network error while deleting pipeline: ${e.message}`);
      }
  }

  async function handleClonePipeline(p: Pipeline) {
      try {
          const res = await apiFetch(`/api/pipelines/${p.id}/clone`, { method: 'POST' });
          if (res.ok) {
              alert('Pipeline cloned.');
              document.body.dispatchEvent(new CustomEvent('pipelinesUpdated'));
          } else {
              const err = await res.json().catch(() => ({ error: res.statusText }));
              errorStore.show(`Error cloning pipeline: ${err.error}`);
          }
      } catch (e: any) {
          errorStore.show(`Network error while cloning pipeline: ${e.message}`);
      }
  }

</script>

<div class="container mx-auto px-4 py-8 space-y-6" in:fade={{ duration: 200 }}>
    <div class="flex justify-between items-center">
        <h1 class="text-3xl font-semibold text-gray-100 dark:text-gray-50">Pipelines</h1>
        {#if loggedIn && orgId}
            <Button variant="primary" on:click={handleCreateNewPipeline} customClass="flex items-center">
                <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="w-5 h-5 mr-2">
                    <path d="M10.75 4.75a.75.75 0 00-1.5 0v4.5h-4.5a.75.75 0 000 1.5h4.5v4.5a.75.75 0 001.5 0v-4.5h4.5a.75.75 0 000-1.5h-4.5v-4.5z" />
                </svg>
                Create New Pipeline
            </Button>
        {/if}
    </div>

    {#if !loggedIn}
        <GlassCard padding="p-8 text-center">
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-12 h-12 text-primary-500 mx-auto mb-4">
              <path stroke-linecap="round" stroke-linejoin="round" d="M16.5 10.5V6.75a4.5 4.5 0 10-9 0v3.75m-.75 11.25h10.5a2.25 2.25 0 002.25-2.25v-6.75a2.25 2.25 0 00-2.25-2.25H6.75a2.25 2.25 0 00-2.25 2.25v6.75a2.25 2.25 0 002.25 2.25z" />
            </svg>
            <p class="text-xl font-medium text-gray-300 mb-2">Access Denied</p>
            <p class="text-gray-400 dark:text-gray-500 mb-6">
                Please log in to manage pipelines.
            </p>
            <Button href="/login" variant="primary">Go to Login</Button>
        </GlassCard>
    {:else if !orgId && loggedIn}
         <GlassCard padding="p-8 text-center">
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-12 h-12 text-warning-500 mx-auto mb-4">
              <path stroke-linecap="round" stroke-linejoin="round" d="M12 9v3.75m-9.303 3.376c-.866 1.5.217 3.374 1.948 3.374h14.71c1.73 0 2.813-1.874 1.948-3.374L13.949 3.378c-.866-1.5-3.032-1.5-3.898 0L2.697 16.126zM12 15.75h.007v.008H12v-.008z" />
            </svg>
            <p class="text-xl font-medium text-gray-300 mb-2">Organization Context Missing</p>
            <p class="text-gray-400 dark:text-gray-500">
                Your account does not seem to be associated with an organization. Pipelines are managed per organization.
            </p>
        </GlassCard>
    {:else if isLoading}
        <GlassCard padding="p-8"><div class="flex justify-center items-center"><div class="w-8 h-8 border-4 border-accent border-t-transparent rounded-full animate-spin mr-3"></div><p class="text-gray-400">Loading pipelines...</p></div></GlassCard>
    {:else if error}
        <GlassCard title="Error Loading Pipelines" padding="p-6">
            <p class="text-red-400 text-center py-4">{error}</p>
            <div class="text-center mt-4">
                <Button variant="secondary" on:click={loadPipelines}>Try Again</Button>
            </div>
        </GlassCard>
    {:else}
        <DataTable
            headers={pipelineTableHeaders}
            items={tablePipelines}
            keyField="id"
            tableSortable={true}
            emptyStateMessage="No pipelines configured yet for this organization."
            emptyStateIconPath="M10.5 6h9.75M10.5 6a1.5 1.5 0 11-3 0m3 0a1.5 1.5 0 10-3 0M3.75 6H7.5m3 12h9.75m-9.75 0a1.5 1.5 0 01-3 0m3 0a1.5 1.5 0 00-3 0m-3.75 0H7.5m9-6h3.75m-3.75 0a1.5 1.5 0 01-3 0m3 0a1.5 1.5 0 00-3 0m-3.75 0H16.5m-6 6h3.75m-3.75 0a1.5 1.5 0 01-3 0m3 0a1.5 1.5 0 00-3 0m-3.75 0H10.5"
            tableContainerClass="bg-neutral-800/40 backdrop-blur-sm shadow-lg rounded-xl border border-neutral-700/50 overflow-hidden"
            tableClass="min-w-full divide-y divide-neutral-700/30"
            thClass="px-4 py-3 text-left text-xs font-medium text-gray-400 uppercase tracking-wider bg-neutral-700/20 whitespace-nowrap"
            trClass="group transition-colors duration-150 hover:bg-neutral-700/50"
            tdClass="px-4 py-3 whitespace-nowrap text-sm text-gray-300"
        >
            <div slot="cell-actions" let:item class="flex justify-end items-center space-x-1">
                <Button variant="ghost" customClass="!px-2 !py-1 text-xs hover:!bg-accent/20" on:click={() => handleEditPipeline(item)}>
                    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="w-4 h-4 mr-1"><path d="M2.695 14.763l-1.262 3.154a.5.5 0 00.65.65l3.155-1.262a4 4 0 001.343-.885L17.5 5.5a2.121 2.121 0 00-3-3L3.58 13.42a4 4 0 00-.885 1.343z" /></svg>
                    Edit
                </Button>
                <Button variant="ghost" customClass="!px-2 !py-1 text-xs hover:!bg-accent/20" on:click={() => handleClonePipeline(item)}>Clone</Button>
                <Button variant="ghost" customClass="!px-2 !py-1 text-xs text-red-400 hover:text-red-300" on:click={() => handleDeletePipeline(item)}>Delete</Button>
            </div>
            <span slot="cell-name" let:item title={item.name} class="block truncate max-w-md group-hover:text-accent-lighter transition-colors">
                {item.name}
            </span>
            <span slot="cell-created_at" let:item class="text-xs text-gray-400">{item.created_at}</span>
            <span slot="cell-updated_at" let:item class="text-xs text-gray-400">{item.updated_at}</span>
        </DataTable>
    {/if}
</div>
