<!-- frontend/src/lib/components/UserManagementView.svelte -->
<script lang="ts">
  import { onMount } from 'svelte';
  import DataTable, { type TableHeader } from '$lib/components/DataTable.svelte';
  import Button from '$lib/components/Button.svelte';
  import InviteUserModal from '$lib/components/InviteUserModal.svelte';
  import ConfirmationModal from '$lib/components/ConfirmationModal.svelte'; // Import ConfirmationModal
  import PaginationControls from '$lib/components/PaginationControls.svelte';
  import { apiFetch } from '$lib/utils/apiUtils';
  import { page } from '$app/stores'; // To get current user ID for self-action prevention

  export interface Organization {
    id: string;
    name: string;
    api_key?: string;
  }

  interface OrgUserView {
    id: string;
    email: string;
    role: string;
    confirmed: boolean;
    is_active: boolean;
    created_at: string;
    deactivated_at?: string | null;
  }

  export let orgId: string;
  export let orgName: string = "Your Organization";

  let usersInOrg: OrgUserView[] = [];
  let isLoadingUsers: boolean = false;
  let errorLoadingUsers: string | null = null;
  let generalError: string | null = null; // For general action errors
  let generalSuccess: string | null = null; // For general action success messages

  // Search & Pagination state
  let emailFilter: string = '';
  let currentPage = 1;
  let usersPerPage = 10;
  let totalUsers = 0;
  let totalPages = 0;
  let filterDebounce: number;


  let showInviteUserModal = false;
  let currentOrgForInvite: Organization;
  $: currentOrgForInvite = { id: orgId, name: orgName, api_key: '' };

  // Confirmation Modal State
  let showConfirmationModal = false;
  let confirmationTitle = '';
  let confirmationMessage = '';
  let confirmActionCallback: (() => Promise<void>) | null = null;
  let confirmButtonText = 'Confirm';
  type ButtonVariant = 'primary' | 'secondary' | 'danger' | 'ghost' | 'link';
  let confirmButtonVariant: ButtonVariant = 'primary';
  let selectedUserForAction: OrgUserView | null = null;

  const currentUserId = $page.data.session?.userId;


  async function loadUsersInOrg(pageToLoad = 1) {
    if (!orgId) {
      errorLoadingUsers = "Organization ID is not available.";
      return;
    }
    isLoadingUsers = true;
    errorLoadingUsers = null;
    generalError = null; // Clear general errors on reload
    generalSuccess = null; // Clear success messages
    currentPage = pageToLoad;
    try {
      let url = `/api/organizations/me/users?page=${pageToLoad}&limit=${usersPerPage}`;
      if (emailFilter.trim() !== '') {
        url += `&email_ilike=${encodeURIComponent(emailFilter.trim())}`;
      }
      const response = await apiFetch(url);
      if (!response.ok) {
        const errText = await response.text();
        const errData = JSON.parse(errText || "{}");
        throw new Error(errData.error || `Failed to fetch users: ${response.statusText} - ${errText}`);
      }
      const data = await response.json();
      if (Array.isArray(data)) {
        usersInOrg = data;
        totalUsers = data.length;
        usersPerPage = data.length;
        totalPages = 1;
        currentPage = 1;
      } else {
        usersInOrg = data.items;
        totalUsers = data.total_items;
        usersPerPage = data.per_page;
        totalPages = data.total_pages;
        currentPage = data.page;
      }
    } catch (e: any) {
      errorLoadingUsers = e.message;
      usersInOrg = [];
      totalUsers = 0;
      totalPages = 0;
      console.error("Error loading users in org:", e);
    } finally {
      isLoadingUsers = false;
    }
  }

  onMount(() => {
    loadUsersInOrg(1);
  });

  const userTableHeaders: TableHeader[] = [
    { key: 'email', label: 'Email', sortable: true, cellClass: 'font-medium !text-gray-100 group-hover:!text-accent-lighter truncate' },
    { key: 'role', label: 'Role', sortable: true, cellClass: '!text-gray-300' },
    { key: 'status', label: 'Account Status', sortable: true },
    { key: 'confirmed', label: 'Email Confirmed', sortable: true },
    { key: 'created_at', label: 'Joined On', sortable: true, cellClass: '!text-gray-300' },
    { key: 'actions', label: 'Actions', sortable: false, headerClass: 'text-right', cellClass: 'text-right !whitespace-nowrap' },
  ];

  function getAccountStatusClass(isActive: boolean): string {
    return isActive ? 'bg-success/20 text-success' : 'bg-amber-500/20 text-amber-100';
  }
  function getConfirmationStatusClass(isConfirmed: boolean): string {
    return isConfirmed ? 'bg-success/20 text-success' : 'bg-error/20 text-error';
  }

  async function performUserAction(actionPath: string, successMessage: string, httpMethod: 'POST' | 'DELETE' = 'POST') {
    generalError = null;
    generalSuccess = null;
    if (!selectedUserForAction) return;
    try {
      const response = await apiFetch(`/api/organizations/me/users/${selectedUserForAction.id}/${actionPath}`, { method: httpMethod });
      const data = await response.json();
      if (!response.ok) throw new Error(data.error || `Failed to ${actionPath} user.`);
      generalSuccess = data.message || successMessage;
      await loadUsersInOrg(); // Refresh list
    } catch (e: any) {
      generalError = e.message;
      console.error(`Error during ${actionPath}:`, e);
    } finally {
      showConfirmationModal = false;
      selectedUserForAction = null;
    }
  }

  function requestRemoveUser(user: OrgUserView) {
    selectedUserForAction = user;
    confirmationTitle = 'Remove User from Organization';
    confirmationMessage = `Are you sure you want to remove (deactivate) ${user.email} from ${orgName}? They will no longer be able to access this organization's resources.`;
    confirmButtonText = 'Remove User';
    confirmButtonVariant = 'danger';
    confirmActionCallback = () => performUserAction('remove', 'User removed successfully.');
    showConfirmationModal = true;
  }

  function requestDeactivateUser(user: OrgUserView) {
    selectedUserForAction = user;
    confirmationTitle = 'Deactivate User Account';
    confirmationMessage = `Are you sure you want to deactivate ${user.email}? They will lose access until reactivated.`;
    confirmButtonText = 'Deactivate';
    confirmButtonVariant = 'danger';
    confirmActionCallback = () => performUserAction('deactivate', 'User deactivated successfully.');
    showConfirmationModal = true;
  }

  function requestReactivateUser(user: OrgUserView) {
    selectedUserForAction = user;
    confirmationTitle = 'Reactivate User Account';
    confirmationMessage = `Are you sure you want to reactivate ${user.email}? They will regain access.`;
    confirmButtonText = 'Reactivate';
    confirmButtonVariant = 'primary';
    confirmActionCallback = () => performUserAction('reactivate', 'User reactivated successfully.');
    showConfirmationModal = true;
  }

  async function handleResendConfirmation(user: OrgUserView) {
    generalError = null;
    generalSuccess = null;
    if (user.confirmed) {
      generalError = "User's email is already confirmed.";
      return;
    }
    try {
      const response = await apiFetch(`/api/organizations/me/users/${user.id}/resend_confirmation`, { method: 'POST' });
      const data = await response.json();
      if (!response.ok) throw new Error(data.error || 'Failed to resend confirmation email.');
      generalSuccess = data.message || 'Confirmation email resent successfully.';
      // No need to reload users list as their status doesn't change immediately
    } catch (e: any) {
      generalError = e.message;
      console.error("Error resending confirmation:", e);
    }
  }

  function onEmailFilterInput() {
    clearTimeout(filterDebounce);
    filterDebounce = window.setTimeout(() => {
      loadUsersInOrg(1);
    }, 400);
  }

  function handlePageChange(event: CustomEvent<{ page: number }>) {
    if (event.detail.page !== currentPage) {
      loadUsersInOrg(event.detail.page);
    }
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
  <div class="mt-2">
    <input
      type="text"
      bind:value={emailFilter}
      on:input={onEmailFilterInput}
      placeholder="Search by email..."
      class="glass-input w-full sm:max-w-xs !text-sm"
    />
  </div>

  {#if generalError}
    <div class="alert alert-error shadow-lg my-4" role="alert">
      <strong class="font-bold mr-1">Error:</strong>
      <span>{generalError}</span>
    </div>
  {/if}
  {#if generalSuccess}
    <div class="alert alert-success shadow-lg my-4" role="alert">
      <strong class="font-bold mr-1">Success:</strong>
      <span>{generalSuccess}</span>
    </div>
  {/if}

  {#if isLoadingUsers}
    <div class="flex justify-center items-center py-10">
      <div class="w-8 h-8 border-4 border-accent border-t-transparent rounded-full animate-spin"></div>
      <p class="ml-3 text-gray-400">Loading users...</p>
    </div>
  {:else if errorLoadingUsers}
    <div class="alert alert-error shadow-lg" role="alert">
      <strong class="font-bold mr-1">Error loading users:</strong>
      <span>{errorLoadingUsers}</span>
    </div>
  {:else}
    <DataTable
      headers={userTableHeaders}
      items={usersInOrg}
      keyField="id"
      tableSortable={true}
      currentPage={currentPage}
      totalPages={totalPages}
      totalItems={totalUsers}
      itemsPerPage={usersPerPage}
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
      <div slot="cell-actions" let:item class="flex justify-end items-center space-x-1">
        {#if item.id === currentUserId}
          <span class="text-xs text-gray-500 italic px-2 py-1">(Your Account)</span>
        {:else if item.role === 'admin' || item.role === 'org_admin'}
          <span class="text-xs text-gray-500 italic px-2 py-1">(Admin Role)</span>
        {:else}
          {#if item.is_active}
            <Button variant="ghost" customClass="!px-1.5 !py-0.5 text-xs !text-orange-400 hover:!text-orange-300" on:click={() => requestDeactivateUser(item)} title="Deactivate User">Deactivate</Button>
          {:else}
            <Button variant="ghost" customClass="!px-1.5 !py-0.5 text-xs !text-green-400 hover:!text-green-300" on:click={() => requestReactivateUser(item)} title="Reactivate User">Reactivate</Button>,
          {/if}
          {#if !item.confirmed && item.is_active}
            <Button variant="ghost" customClass="!px-1.5 !py-0.5 text-xs !text-sky-400 hover:!text-sky-300" on:click={() => handleResendConfirmation(item)} title="Resend Confirmation Email">Resend Email</Button>
          {/if}
          <Button variant="ghost" customClass="!px-1.5 !py-0.5 text-xs !text-red-500 hover:!text-red-400" on:click={() => requestRemoveUser(item)} title="Remove User from Organization">Remove</Button>
        {/if}
      </div>
      <div slot="paginationControls" let:currentPageProps let:totalPagesProps>
        <PaginationControls
          currentPage={currentPageProps}
          totalPages={totalPagesProps}
          on:pageChange={handlePageChange}
        />
      </div>
    </DataTable>
  {/if}
</div>

{#if showInviteUserModal && orgId}
  <InviteUserModal
    isOpen={showInviteUserModal}
    organizations={[currentOrgForInvite]}
    isOrgAdminInvite={true}
    defaultOrgId={orgId}
    on:close={() => showInviteUserModal = false}
    on:user_invited={() => {
      showInviteUserModal = false;
      loadUsersInOrg();
      generalSuccess = 'User invited successfully! They need to confirm their email.'; // Show success message
    }}
  />
{/if}

{#if showConfirmationModal && confirmActionCallback}
  <ConfirmationModal
    isOpen={showConfirmationModal}
    title={confirmationTitle}
    message={confirmationMessage}
    confirmText={confirmButtonText}
    confirmButtonVariant={confirmButtonVariant}
    on:confirm={() => { if (confirmActionCallback) confirmActionCallback(); }}
    on:cancel={() => { showConfirmationModal = false; selectedUserForAction = null; }}
    on:close={() => { showConfirmationModal = false; selectedUserForAction = null; }}
  />
{/if}
