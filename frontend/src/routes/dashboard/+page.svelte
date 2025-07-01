<script lang="ts">
  import Dashboard from '$lib/components/Dashboard.svelte';
  import GlassCard from '$lib/components/GlassCard.svelte'; // Used for consistent page structure
  import { page } from '$app/stores'; // To access layout data

  // The `data` prop would come from a +layout.ts or +page.ts load function
  // which should provide session information, including orgId.
  // export let data: { session?: { org?: string | null; loggedIn?: boolean } }; // More specific type
  // For this step, we assume `data.session.org` is populated by the root +layout.svelte's data prop
  // which itself would get it from a future +layout.ts.

  $: orgId = $page.data.session?.org;
  $: loggedIn = $page.data.session?.loggedIn;

  // Components for main view buttons, shown if not in a specific view via sidebar
  import Button from '$lib/components/Button.svelte';
  import UploadForm from '$lib/components/UploadForm.svelte';
  import DocumentList from '$lib/components/DocumentList.svelte';
  import JobsList from '$lib/components/JobsList.svelte';

  // These states are now local to this page, if needed, or part of other components.
  // The global controls are in +layout.svelte.
  // For example, UploadForm and DocumentList might be part of a /documents page.
  // JobsList might be part of this dashboard.
  // For now, including some elements directly on dashboard page.

  // Data for sub-components - this would typically be loaded here or in +page.ts
  // and not rely on App.svelte's old global fetches.
  // For this example, assume Dashboard, DocumentList, JobsList internally fetch their data
  // or receive it if this +page.svelte had a +page.ts load function.
  // The `Dashboard` component already takes `orgId` and fetches its own data.
  // `DocumentList` and `JobsList` take `docs` and `jobs` props.
  // These would need to be loaded here or in a +page.ts.

  // Placeholder data for now, assuming Dashboard component handles its own data fetching via orgId.
  // DocumentList and JobsList would need their data passed or fetched.
  // For simplicity, we are only focusing on Dashboard component here.
  // Other components like DocumentList, JobsList would be on their own pages.

</script>

<div class="space-y-6">
  {#if loggedIn && orgId}
    <GlassCard
      title="Main Dashboard"
      padding="p-4 sm:p-6"
      customClass="text-left w-full"
      titleClass="text-2xl font-semibold text-gray-100 mb-4"
      bgOpacity="!bg-neutral-800/50"
      borderStyle="!border-neutral-700/60"
    >
      <Dashboard {orgId} />
    </GlassCard>

    <!--
      Other sections like DocumentList, JobsList, UploadForm would typically move to
      their own dedicated pages (e.g., /documents, /jobs) accessible via the Sidebar.
      For this example, we'll keep the Dashboard page focused.
      If you want to include them here, you'd need to handle their data loading.
    -->

  {:else if loggedIn && !orgId}
    <GlassCard title="Organization Context Missing" padding="p-6" customClass="text-center">
      <p class="text-gray-400">
        Your organization information is not available. Please ensure you are part of an organization.
      </p>
      <p class="mt-2 text-xs text-gray-500">
        If this issue persists, contact support.
      </p>
    </GlassCard>
  {:else}
    <!-- This state should ideally be handled by redirects in +layout.ts or +page.ts -->
    <GlassCard title="Access Denied" padding="p-6" customClass="text-center">
      <p class="text-gray-400">You need to be logged in to view the dashboard.</p>
      <a href="/login" class="mt-4 inline-block text-accent hover:underline">Go to Login</a>
    </GlassCard>
  {/if}
</div>
