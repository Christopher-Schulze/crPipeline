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
  let showPipeline = false;
  let showSettings = false;
  let showAdmin = false;
  
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
    showSettings = false;
  }
</script>

<main class="min-h-screen flex items-center justify-center p-8">
  <div class="glass rounded-2xl p-8 text-center space-y-4 depth-2">
    <h1 class="text-3xl font-semibold">crPipeline</h1>
    {#if loggedIn}
      <div class="space-x-2">
        <Button on:click={() => showPipeline = !showPipeline}>
          {showPipeline ? 'Close Editor' : 'New Pipeline'}
        </Button>
        <Button on:click={() => showSettings = !showSettings}>
          {showSettings ? 'Close Settings' : 'Settings'}
        </Button>
        {#if role === 'admin'}
          <Button on:click={() => showAdmin = !showAdmin}>
            {showAdmin ? 'Close Admin' : 'Admin'}
          </Button>
        {/if}
      </div>
      {#if showPipeline}
        <PipelineEditor />
      {/if}
      {#if showSettings}
        <SettingsForm orgId={org} on:saved={settingsSaved} />
      {/if}
      {#if showAdmin}
        <OrgAdmin />
      {/if}
      <Dashboard orgId={org} />
      <UploadForm orgId={org} userId={userId ?? ''} on:uploaded={uploadedHandler} />
      <DocumentList {docs} />
      <JobsList {jobs} />
    {:else}
      <LoginForm on:loggedin={loggedInHandler} />
    {/if}
  </div>
</main>
