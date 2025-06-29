<script lang="ts">
  import UploadForm from './lib/components/UploadForm.svelte';
  import DocumentList from './lib/components/DocumentList.svelte';
  import JobsList from './lib/components/JobsList.svelte';
  import LoginForm from './lib/components/LoginForm.svelte';
  import PipelineEditor from './lib/components/PipelineEditor.svelte';
  import SettingsForm from './lib/components/SettingsForm.svelte';
  import Dashboard from './lib/components/Dashboard.svelte';
  import Button from './lib/components/Button.svelte';
  import OrgAdmin from './lib/components/OrgAdmin.svelte';
  import { onMount } from 'svelte';

  let loggedIn = false;
  let org: string | null = null;
  let userId: string | null = null;
  let role: string | null = null;
  let docs: { id: string; filename: string }[] = [];
  let jobs: { id: string; status: string }[] = [];
  // let showPipeline = false; // Removed this line
  let showSettingsPanel = false; // Renamed from showSettings
  let showAdmin = false; // Assuming OrgAdmin is still toggled this way or will be moved to a view

  // Import GlassCard
  import GlassCard from './lib/components/GlassCard.svelte';
  // Import Sidebar
  import Sidebar from './lib/components/Sidebar.svelte';
  // Import AnalysisJobDetail and context API
  import AnalysisJobDetail from './lib/components/AnalysisJobDetail.svelte';
  import { setContext } from 'svelte';
  // Import SlideOver
  import SlideOver from './lib/components/SlideOver.svelte';
  // PipelineEditor is already imported higher up, ensure it's there.

  // Define NavItem interface (could be imported if Sidebar exports it)
  interface NavItem {
    id: string;
    path: string;
    label: string;
    icon?: string;
  }

  let currentPath: string = '/dashboard'; // Default view path
  let currentView: string = 'dashboard';  // To control component display

  const mainNavItems: NavItem[] = [
    { id: 'dashboard', path: '/dashboard', label: 'Dashboard', icon: 'M3.75 13.5l10.5-11.25L12 10.5h8.25L9.75 21.75 12 13.5H3.75z' },
    { id: 'documents', path: '/documents', label: 'Documents', icon: 'M19.5 14.25v-2.625a3.375 3.375 0 00-3.375-3.375h-1.5A1.125 1.125 0 0113.5 7.125v-1.5a3.375 3.375 0 00-3.375-3.375H8.25m0 12.75h7.5m-7.5 3H12M10.5 2.25H5.625c-.621 0-1.125.504-1.125 1.125v17.25c0 .621.504 1.125 1.125 1.125h12.75c.621 0 1.125-.504 1.125-1.125V11.25a9 9 0 00-9-9z' },
    // { id: 'pipelines', path: '/pipelines', label: 'Pipelines', icon: 'M10.5 6h9.75M10.5 6a1.5 1.5 0 11-3 0m3 0a1.5 1.5 0 10-3 0M3.75 6H7.5m3 12h9.75m-9.75 0a1.5 1.5 0 01-3 0m3 0a1.5 1.5 0 00-3 0m-3.75 0H7.5m9-6h3.75m-3.75 0a1.5 1.5 0 01-3 0m3 0a1.5 1.5 0 00-3 0m-3.75 0H16.5m-6 6h3.75m-3.75 0a1.5 1.5 0 01-3 0m3 0a1.5 1.5 0 00-3 0m-3.75 0H10.5' },
    // Pipeline editor is handled by a button for now, not a main view.
  ];
  // Admin/Settings nav items could be added conditionally to mainNavItems based on role.

  let currentViewedJobId: string | null = null;
  let showPipelineEditorPanel = false;

  function handleSidebarNavigate(event: CustomEvent<{ path: string }>) {
    currentPath = event.detail.path;
    const newView = currentPath.substring(1); // Remove leading '/'
    if (newView === 'dashboard' || newView === 'documents') {
        currentView = newView;
    } else {
        currentView = 'dashboard'; // Default to dashboard if path is unknown
    }
    // Close other panels when navigating
    showSettingsPanel = false; // Updated variable name
    showAdmin = false;
    showPipelineEditorPanel = false;
    currentViewedJobId = null; // Ensure job detail modal also closes
    console.log("Navigating to view:", currentView);
  }

  function toggleSettingsPanel() {
    if (showSettingsPanel) {
      showSettingsPanel = false;
    } else {
      currentView = 'settings_view_active'; // Or some other indicator that a panel is the main focus
      showPipelineEditorPanel = false;
      showAdmin = false;
      currentViewedJobId = null;
      showSettingsPanel = true;
    }
  }

  function viewJobDetails(jobId: string) {
    currentViewedJobId = jobId;
  }

  function closeJobDetails() {
    currentViewedJobId = null;
  }

  function togglePipelineEditorPanel() {
    // Updated to ensure mutual exclusivity with other panels/views
    if (showPipelineEditorPanel) {
      showPipelineEditorPanel = false;
    } else {
      currentView = 'pipeline_editor_active'; // Or some other indicator
      showSettingsPanel = false;
      showAdmin = false;
      currentViewedJobId = null;
      showPipelineEditorPanel = true;
    }
  }

  setContext('viewJobDetails', viewJobDetails);
  
  async function checkAuth() {
    const res = await fetch('/api/me');
    if (res.ok) {
      const data = await res.json();
      org = data.org_id;
      userId = data.user_id;
      role = data.role;
      loggedIn = true;
    } else {
      loggedIn = false;
    }
  }

  async function loadData() {
    if (!org) return;
    const res = await fetch(`/api/documents/${org}`);
    if (res.ok) {
      docs = await res.json();
    }
    const jobRes = await fetch(`/api/jobs/${org}`);
    if (jobRes.ok) {
      jobs = await jobRes.json();
    }
  }

  async function loadSettings() {
    if (!org) return;
    const res = await fetch(`/api/settings/${org}`);
    if (res.ok) {
      const data = await res.json();
      document.documentElement.style.setProperty('--color-accent', data.accent_color);
    }
  }

  onMount(async () => {
    await checkAuth();
    if (loggedIn) {
      await loadData();
      await loadSettings();
    }
  });

  async function loggedInHandler() {
    await checkAuth();
    if (loggedIn) {
      await loadData();
      await loadSettings();
    }
  }

  async function uploadedHandler() {
    await loadData();
  }

  function settingsSaved(e: CustomEvent<{ accentColor: string }>) {
    document.documentElement.style.setProperty('--color-accent', e.detail.accentColor);
    showSettingsPanel = false; // Use new variable name
  }
</script>

<!-- Adjust main layout to include Sidebar -->
<main class="min-h-screen flex bg-gray-100 text-gray-900 dark:bg-neutral-900 dark:text-gray-200">
  {#if loggedIn && org} <!-- Show sidebar only when logged in and org context is available -->
    <Sidebar navItems={mainNavItems} {currentPath} on:navigate={handleSidebarNavigate} />
  {/if}

  <div class="flex-1 p-4 sm:p-6 md:p-8 overflow-auto">
    {#if loggedIn && org }
      <!-- Main Content Area based on currentView -->
      {#if currentView === 'dashboard'}
        <GlassCard title="Dashboard" padding="p-4 md:p-6" customClass="text-left space-y-4">
          <Dashboard orgId={org} />
        </GlassCard>
      {:else if currentView === 'documents'}
        <GlassCard title="Documents" padding="p-4 md:p-6" customClass="text-left space-y-4">
          <!-- Buttons for main actions related to this view -->
          <div class="mb-4 flex space-x-2">
             <UploadForm orgId={org} userId={userId ?? ''} on:uploaded={uploadedHandler} />
          </div>
          <DocumentList {docs} />
        </GlassCard>
      {:else}
        <GlassCard title="Content Area" padding="p-6">
            <p>Selected Path: {currentPath}</p>
            <p class="mt-4">Welcome, {userId} from org {org}. Role: {role}</p>
            <p>Content for '{currentView}' view to be implemented.</p>
             <div class="mt-6 space-y-4">
                <h3 class="text-lg font-semibold">Quick Toggles (Dev):</h3>
                <div class="space-x-2">
                    <Button variant="secondary" on:click={togglePipelineEditorPanel}>
                        {showPipelineEditorPanel ? 'Close Pipeline Editor' : 'Open Pipeline Editor'}
                    </Button>
                    <Button variant="secondary" on:click={toggleSettingsPanel}>
                        {showSettingsPanel ? 'Close Settings' : 'Open Settings'}
                    </Button>
                    {#if role === 'admin'}
                        <Button variant="secondary" on:click={() => {
                           if (showAdmin) {
                               showAdmin = false;
                           } else {
                               currentView = 'admin_view_active'; // Or similar to indicate context switch
                               showSettingsPanel = false;
                               showPipelineEditorPanel = false;
                               currentViewedJobId = null;
                               showAdmin = true;
                           }
                        }}>
                        {showAdmin ? 'Close Admin' : 'Open Admin'}
                        </Button>
                    {/if}
                </div>
             </div>
        </GlassCard>
      {/if}

      <!-- Containers for components that are not main views but can be shown/hidden globally -->
      {#if showAdmin && org && role === 'admin'}
         <GlassCard title="Admin Panel" customClass="mt-6 text-left space-y-4" padding="p-4 md:p-6">
            <OrgAdmin />
         </GlassCard>
      {/if}

      <!-- JobsList could be part of dashboard or its own view or a global panel -->
      <GlassCard title="Analysis Jobs" customClass="mt-6 text-left space-y-4" padding="p-4 md:p-6">
        <JobsList {jobs} />
      </GlassCard>

    {:else if !loggedIn}
      <div class="w-full max-w-md mx-auto mt-[10vh]">
        <GlassCard padding="p-8">
          <h1 class="text-3xl font-semibold text-center mb-6">crPipeline</h1>
          <LoginForm on:loggedin={loggedInHandler} />
        </GlassCard>
      </div>
    {:else if !org && loggedIn}
        <GlassCard title="Loading Organization..." padding="p-6">
            <p>Please wait while your organization details are being loaded...</p>
        </GlassCard>
    {/if}
  </div>

  {#if currentViewedJobId}
    <AnalysisJobDetail jobId={currentViewedJobId} on:close={closeJobDetails} />
  {/if}

  <SlideOver
    isOpen={showPipelineEditorPanel}
    title="Pipeline Editor"
    position="right"
    maxWidth="max-w-xl"
    on:close={() => showPipelineEditorPanel = false}
  >
    <div slot="content">
      {#if showPipelineEditorPanel && org}
        <PipelineEditor
          orgId={org}
          on:saved={() => {
            console.log('Pipeline saved event received in App.svelte');
            showPipelineEditorPanel = false;
            // Potentially refresh a list of pipelines here
          }}
          on:cancel={() => {
            showPipelineEditorPanel = false;
          }}
        />
      {:else if showPipelineEditorPanel && !org}
         <p class="text-red-500 p-4">Organization ID is not available. Cannot load pipeline editor.</p>
      {/if}
    </div>
  </SlideOver>

  <SlideOver
    isOpen={showSettingsPanel && !!org}
    title="Organization Settings"
    position="right"
    maxWidth="max-w-lg"
    on:close={() => showSettingsPanel = false}
  >
    <div slot="content">
      {#if showSettingsPanel && org}
        <SettingsForm
          orgId={org}
          on:saved={(e) => {
            settingsSaved(e);
          }}
        />
        <!-- No explicit on:cancel needed if SettingsForm doesn't have a dedicated cancel button -->
      {:else if showSettingsPanel && !org}
        <p class="text-red-500 p-4">Organization ID is not available. Cannot load settings.</p>
      {/if}
    </div>
  </SlideOver>
</main>
