<!-- frontend/src/lib/components/UserManagementView.svelte -->
<script lang="ts">
  import { onMount } from 'svelte';
  // import { page } from '$app/stores'; // Not used directly here, orgId/Name are props
  import DataTable, { type TableHeader } from '$lib/components/DataTable.svelte';
  import Button from '$lib/components/Button.svelte';
  import InviteUserModal from '$lib/components/InviteUserModal.svelte';
  import { apiFetch } from '$lib/utils/apiUtils';

  // Define Organization type, similar to OrgAdmin or a future shared types file
  // For InviteUserModal, it primarily needs 'id' and 'name'.
  export interface Organization {
    id: string;
    name: string;
    api_key?: string; // Optional as not always needed by InviteUserModal if just for display
  }

  interface OrgUserView {
    id: string;
    email: string;
    role: string; // Should be 'user' for those managed by an OrgAdmin
    confirmed: boolean;
    is_active: boolean;
    created_at: string;
    deactivated_at?: string | null; // Included for completeness, though not displayed in a column
  }

  export let orgId: string;
  export let orgName: string = "Your Organization";

  let usersInOrg: OrgUserView[] = [];
  let isLoadingUsers: boolean = false;
  let errorLoadingUsers: string | null = null;

  let showInviteUserModal = false;

  let currentOrgForInvite: Organization; // For InviteUserModal, needs to match its expected type
  $: currentOrgForInvite = { id: orgId, name: orgName, api_key: '' }; // api_key can be dummy

  async function loadUsersInOrg() {
    if (!orgId) {
      errorLoadingUsers = "Organization ID is not available.";
      return;
    }
    isLoadingUsers = true;
    errorLoadingUsers = null;
    try {
      // This endpoint fetches users for the currently authenticated user's organization
      // Ensure the backend correctly identifies the org admin and their org.
      const response = await apiFetch(`/api/organizations/me/users`);
      if (!response.ok) {
        const errText = await response.text();
        const errData = JSON.parse(errText || "{}"); // Try to parse error
        throw new Error(errData.error || `Failed to fetch users: ${response.statusText} - ${errText}`);
      }
      usersInOrg = await response.json();
    } catch (e: any) {
      errorLoadingUsers = e.message;
      usersInOrg = [];
      console.error("Error loading users in org:", e);
    } finally {
      isLoadingUsers = false;
    }
  }

  onMount(() => {
    loadUsersInOrg();
  });

  const userTableHeaders: TableHeader[] = [
    { key: 'email', label: 'Email', sortable: true, cellClass: 'font-medium !text-gray-100 group-hover:!text-accent-lighter truncate' },
    { key: 'role', label: 'Role', sortable: true, cellClass: '!text-gray-300' },
    { key: 'status', label: 'Account Status', sortable: true },
    { key: 'confirmed', label: 'Email Confirmed', sortable: true },
    { key: 'created_at', label: 'Joined On', sortable: true, cellClass: '!text-gray-300' },
  ];

  function getAccountStatusClass(isActive: boolean): string {
    return isActive ? 'bg-success/20 text-success' : 'bg-amber-500/20 text-amber-100';
  }
  function getConfirmationStatusClass(isConfirmed: boolean): string {
    return isConfirmed ? 'bg-success/20 text-success' : 'bg-error/20 text-error';
  }

</script>

<div class="space-y-6">
  <div class="flex justify-between items-center">
    <h2 class="text-2xl font-semibold text-gray-100">Manage Users in <span class="text-accent">{orgName}</span></h2>
    <Button variant="primary" on:click={() => showInviteUserModal = true}>
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="w-5 h-5 mr-2">
        <path d="M10.75 4.75a.75.75 0 00-1.5 0v4.5h-4.5a.75.75 0 000 1.5h4.5v4.5a.75.75 0 001.5 0v-4.5h4.5a.75.75 0 000-1.5h-4.5v-4.5z" />
      </svg>
      Invite New User
    </Button>
  </div>

  {#if isLoadingUsers}
    <div class="flex justify-center items-center py-10">
      <div class="w-8 h-8 border-4 border-accent border-t-transparent rounded-full animate-spin"></div>
      <p class="ml-3 text-gray-400">Loading users...</p>
    </div>
  {:else if errorLoadingUsers}
    <div class="bg-error/10 border border-error/30 text-error px-4 py-3 rounded-lg" role="alert">
      <strong class="font-bold">Error:</strong>
      <span class="block sm:inline ml-1">{errorLoadingUsers}</span>
    </div>
  {:else}
    <DataTable
      headers={userTableHeaders}
      items={usersInOrg}
      keyField="id"
      tableSortable={true}
      emptyStateMessage="No users found in this organization."
      emptyStateIconPath="M15 19.128a9.38 9.38 0 002.625.372M7.5 0A4.5 4.5 0 003 4.5v.75A.75.75 0 004.5 6h4.5a.75.75 0 00.75-.75v-.75A4.5 4.5 0 007.5 0zm0 9a4.5 4.5 0 00-4.5 4.5v.75a.75.75 0 00.75.75h7.5a.75.75 0 00.75-.75v-.75A4.5 4.5 0 007.5 9zm-2.625 5.628a9.37 9.37 0 01-2.625.372m16.5 0a9.37 9.37 0 01-2.625-.372M12 21a9.375 9.375 0 01-3-1.372A9.375 9.375 0 013 21m18 0a9.375 9.375 0 01-3 1.372A9.375 9.375 0 0121 21m-9-1.628c.394.06.794.1.9.1.106 0 .506-.04.9-.1M12 12a3 3 0 11-6 0 3 3 0 016 0zm6 0a3 3 0 11-6 0 3 3 0 016 0z"
      tableContainerClass="bg-neutral-800/40 backdrop-blur-sm shadow-lg rounded-xl border border-neutral-700/50 overflow-hidden"
      tableClass="min-w-full divide-y divide-neutral-700/30"
      thClass="px-4 py-2.5 text-left text-xs font-medium text-gray-400 uppercase tracking-wider bg-neutral-700/20 whitespace-nowrap"
      trClass="group transition-colors duration-150 hover:bg-neutral-700/50"
      tdClass="px-4 py-3 whitespace-nowrap text-sm text-gray-300"
    >
      <span slot="cell-status" let:item>
        <span class="px-2.5 py-0.5 inline-flex text-xs leading-5 font-semibold rounded-full {getAccountStatusClass(item.is_active)}">
          {item.is_active ? 'Active' : 'Deactivated'}
          {#if !item.is_active && item.deactivated_at}
             <span class="ml-1 font-light text-xs text-gray-400 dark:text-gray-500 hidden sm:inline">({new Date(item.deactivated_at).toLocaleDateString()})</span>
          {/if}
        </span>
      </span>
      <span slot="cell-confirmed" let:item>
        <span class="px-2.5 py-0.5 inline-flex text-xs leading-5 font-semibold rounded-full {getConfirmationStatusClass(item.confirmed)}">
          {item.confirmed ? 'Confirmed' : 'Pending'}
        </span>
      </span>
      <span slot="cell-created_at" let:item class="text-xs">
         {new Date(item.created_at).toLocaleDateString('en-CA', { year: 'numeric', month: 'short', day: 'numeric' })}
         <span class="block text-[0.65rem] text-gray-400/80">
             {new Date(item.created_at).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}
         </span>
      </span>
      <span slot="cell-email" let:item title={item.email} class="block max-w-xs group-hover:text-accent-lighter transition-colors truncate">{item.email}</span>
    </DataTable>
  {/if}
</div>

{#if showInviteUserModal && orgId}
  <InviteUserModal
    isOpen={showInviteUserModal}
    organizations={[currentOrgForInvite]}
    isOrgAdminInvite={true}
    defaultOrgId={orgId} {# Pass defaultOrgId explicitly #}
    on:close={() => showInviteUserModal = false}
    on:user_invited={() => {
      showInviteUserModal = false;
      loadUsersInOrg();
    }}
  />
{/if}
</html>
