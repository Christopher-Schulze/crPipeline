<script lang="ts">
  import { onMount, setContext, tick, onDestroy } from 'svelte'; // tick might not be needed, added onDestroy
  import { page, navigating } from '$app/stores'; // For currentPath and global loading

  // Component Imports
  import Sidebar from '$lib/components/Sidebar.svelte';
  import SettingsForm from '$lib/components/SettingsForm.svelte';
  import PipelineEditor from '$lib/components/PipelineEditor.svelte';
  // Modal is used by AnalysisJobDetail, not directly here unless for a global layout modal
  // import Modal from '$lib/components/Modal.svelte';
  import SlideOver from '$lib/components/SlideOver.svelte';
  import AnalysisJobDetail from '$lib/components/AnalysisJobDetail.svelte';
  import Button from '$lib/components/Button.svelte'; // For Dev Toggles
  import GlobalLoadingIndicator from '$lib/components/GlobalLoadingIndicator.svelte';

  // Type Imports or Definitions
  export interface NavItem {
    id: string;
    path: string;
    label: string;
    icon?: string;
  }

  // Props from SvelteKit load function (e.g., from src/routes/+layout.ts)
  export let data: {
    session?: {
      loggedIn: boolean;
      org: string | null;
      userId: string | null;
      role: string | null;
      // Potentially other user/org specific settings like accent_color if loaded in +layout.ts
    }
  };

  // Reactive derivations from `data`
  $: loggedIn = data?.session?.loggedIn || false;
  $: org = data?.session?.org || null;
  $: userId = data?.session?.userId || null;
  $: role = data?.session?.role || null;
  // Accent color would also ideally come from `data` if loaded in +layout.ts
  // onMount(() => { if (data?.session?.accentColor) document.documentElement.style.setProperty('--color-accent', data.session.accentColor); });
  // For now, accent color setting is deferred from this layout, SettingsForm still handles its own update.

  // Global Overlay States
  let showSettingsPanel = false;
  let showPipelineEditorPanel = false;
  let currentViewedJobId: string | null = null;
  let pipelineToEdit: Pipeline | null = null;

  // Define Pipeline type (consistent with PipelineEditor's expectation)
  export interface Pipeline { // Exporting if other components might use it, or keep local
    id?: string; // Optional for new pipelines
    name: string;
    org_id: string;
    stages: any[]; // Should match PipelineEditor's Stage interface array
    // Add other fields if present from backend Pipeline model (e.g., description)
  }

  // Context for viewJobDetails
  setContext('viewJobDetails', (jobId: string) => {
    currentViewedJobId = jobId;
    // Close other full-context overlays when opening job details
    showSettingsPanel = false;
    showPipelineEditorPanel = false;
  });

  function closeJobDetails() {
    currentViewedJobId = null;
  }

  // Settings Panel Logic
  function toggleSettingsPanel() {
    if (showSettingsPanel) {
      showSettingsPanel = false;
    } else {
      showPipelineEditorPanel = false;
      currentViewedJobId = null;
      pipelineToEdit = null; // Clear pipeline edit state
      showSettingsPanel = true;
    }
  }
  function settingsSaved() {
    showSettingsPanel = false;
  }

  // Pipeline Editor Panel Logic
  function openPipelineEditor(pipelineData?: Pipeline | null) {
    pipelineToEdit = pipelineData || null;
    currentViewedJobId = null;
    showSettingsPanel = false;
    showPipelineEditorPanel = true;
  }

  function closePipelineEditor() {
    showPipelineEditorPanel = false;
    // Consider clearing pipelineToEdit after transition for cleanliness, handled by SlideOver's on:close if needed
    // setTimeout(() => pipelineToEdit = null, 300); // Example: clear after transition
  }

  setContext('managePipeline', (pipelineData?: Pipeline | null) => {
    openPipelineEditor(pipelineData);
  });

  // Updated dev toggle for Pipeline Editor
  function devTogglePipelineEditor() {
      if (showPipelineEditorPanel) {
          closePipelineEditor();
      } else {
          openPipelineEditor(null); // Open for new pipeline
      }
  }

  // Global loading indicator state
  let globalLoading = false;
  const unsubscribeNavigating = navigating.subscribe(value => {
    globalLoading = !!value; // true if navigating, false otherwise
  });

  onDestroy(() => {
    unsubscribeNavigating();
  });

  // Sidebar Navigation Items
  // These could also be dynamically built in +layout.ts based on role/features
  const baseNavItems: NavItem[] = [
    { id: 'dashboard', path: '/dashboard', label: 'Dashboard', icon: 'M3.75 13.5l10.5-11.25L12 10.5h8.25L9.75 21.75 12 13.5H3.75z' }, // Bolt icon
    { id: 'documents', path: '/documents', label: 'Documents', icon: 'M19.5 14.25v-2.625a3.375 3.375 0 00-3.375-3.375h-1.5A1.125 1.125 0 0113.5 7.125v-1.5a3.375 3.375 0 00-3.375-3.375H8.25m0 12.75h7.5m-7.5 3H12M10.5 2.25H5.625c-.621 0-1.125.504-1.125 1.125v17.25c0 .621.504 1.125 1.125 1.125h12.75c.621 0 1.125-.504 1.125-1.125V11.25a9 9 0 00-9-9z' }, // Document icon
    {
      id: 'pipelines',
      path: '/pipelines',
      label: 'Pipelines',
      icon: 'M3.75 3v11.25A2.25 2.25 0 006 16.5h12A2.25 2.25 0 0020.25 14.25V3m-16.5 0h16.5m-16.5 0H2.25m1.5 0H2.25m15 0H21.75m-1.5 0H21.75m0 0V2.25m0 12V15M3.75 15V2.25m0 0V1.5m0 0H2.25m19.5 0H3.75M3.75 6H20.25M3.75 9H20.25M3.75 12H20.25M3.75 15H20.25' // Simplified "ListBullet" or "QueueList" like icon
    },
  ];

  $: currentMainNavItems = (() => {
    let items = [...baseNavItems];
    if (loggedIn && role === 'admin') {
      items.push({
        id: 'admin',
        path: '/admin',
        label: 'Global Admin',
        icon: 'M9 12.75L11.25 15 15 9.75m-3-7.036A11.959 11.959 0 013.598 6 11.959 11.959 0 011.044 10.044m19.912 0A11.959 11.959 0 0117.402 6 11.959 11.959 0 0119.956 10.044M4.75 17.13A11.94 11.94 0 0012 21.75a11.94 11.94 0 007.25-4.62M12 10.5h.008v.008H12v-.008z' // ShieldCheckIcon
      });
    }
    if (loggedIn && role === 'org_admin') {
      items.push({
        id: 'org_users',
        path: '/organization/users',
        label: 'Manage Users', // Label for Org Admins to manage their users
        icon: 'M18 18.72a9.094 9.094 0 003.741-.479 3 3 0 00-3.741-5.588M14.25 10.5a3 3 0 11-6 0 3 3 0 016 0zm-3 6a3 3 0 100-6 3 3 0 000 6zM21 12c0 3.182-3.035 5.752-7.035 5.752S6.93 15.182 6.93 12c0-3.182 3.035-5.752 7.035-5.752S21 8.818 21 12zM3.803 14.94A9.048 9.048 0 012.25 12c0-2.02.745-3.886 2-5.282m15.994 10.564A9.047 9.047 0 0121.75 12c0-2.02-.745-3.886-2-5.282' // UsersIcon (simplified representation)
      });
    }
    // Settings and Pipeline Editor are global toggles, not main navigation views here.
    return items;
  })();

</script>

<GlobalLoadingIndicator loading={globalLoading} />

<main class="min-h-screen flex bg-base text-gray-900 dark:bg-neutral-900 dark:text-gray-100">
  {#if loggedIn && org}
    <Sidebar navItems={currentMainNavItems} currentPath={$page.url.pathname} />
  {/if}

  <div class="flex-1 flex flex-col p-0 overflow-y-auto custom-scrollbar-thin"> {/* Removed main padding, pages handle their own */}
    <div class="flex-grow p-4 sm:p-6 md:p-8"> {/* Inner div for page padding */}
      <slot></slot> <!-- SvelteKit page content will go here -->
    </div>
  </div>

  <!-- Global Overlays -->
  {#if loggedIn && org}
    <SlideOver
      isOpen={showSettingsPanel} <!-- Removed && !!org as parent #if already checks org -->
      title="Organization Settings"
      position="right"
      maxWidth="max-w-lg"
      on:close={() => showSettingsPanel = false}
    >
      <div slot="content">
        {#if showSettingsPanel} <!-- org is already confirmed by parent #if -->
          <SettingsForm {orgId} on:saved={settingsSaved} />
        {/if}
      </div>
    </SlideOver>

    <SlideOver
      isOpen={showPipelineEditorPanel && !!org}
      title={pipelineToEdit ? "Edit Pipeline" : "Create New Pipeline"}
      position="right"
      maxWidth="max-w-xl"
      on:close={() => {
        closePipelineEditor();
        // pipelineToEdit = null; // Also clear here if not relying on setTimeout in closePipelineEditor
      }}
    >
      <div slot="content">
        {#if showPipelineEditorPanel && org}
          <PipelineEditor
            orgId={org}
            initialPipeline={pipelineToEdit}
            on:saved={() => {
              closePipelineEditor();
              // TODO: Consider dispatching global 'pipelinesUpdated' event for other pages to react
              // For example, using a custom event on `document.body` or a Svelte store.
              // This would allow a pipeline list page to refresh automatically.
              // For now, manual refresh or re-navigation will show changes.
            }}
            on:cancel={closePipelineEditor}
          />
        {/if}
      </div>
    </SlideOver>
  {/if}

  {#if currentViewedJobId}
    <AnalysisJobDetail jobId={currentViewedJobId} on:close={closeJobDetails} />
  {/if}

  <!-- Temporary Dev Quick Toggles - can be moved to a dev utility component or removed -->
  <div class="fixed bottom-4 right-4 z-[100] p-2 bg-neutral-700/60 dark:bg-neutral-800/80 backdrop-blur-md rounded-lg shadow-xl space-x-2">
      {#if loggedIn && org}
          <Button variant="secondary" on:click={toggleSettingsPanel} customClass="text-xs !py-1 !px-2">{showSettingsPanel ? 'Close Settings' : 'Settings'}</Button>
          <Button variant="secondary" on:click={devTogglePipelineEditor} customClass="text-xs !py-1 !px-2">{showPipelineEditorPanel ? 'Close Editor' : 'Pipeline Editor'}</Button>
      {:else if !loggedIn}
          <span class="text-xs text-white/70">Login to access dev toggles.</span>
      {/if}
  </div>
</main>
