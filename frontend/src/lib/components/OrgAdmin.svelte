<script lang="ts">
  import GlassCard from './GlassCard.svelte';
  import Button from './Button.svelte';
  import Modal from './Modal.svelte';
  import DataTable, { type TableHeader } from './DataTable.svelte';
  import PaginationControls from './PaginationControls.svelte';
  import EditUserRoleModal from './EditUserRoleModal.svelte';
  import ConfirmationModal from './ConfirmationModal.svelte';
  import InviteUserModal from './InviteUserModal.svelte';
  import { onMount } from 'svelte';
import { apiFetch } from '$lib/utils/apiUtils'; // Import apiFetch
import { errorStore } from '$lib/utils/errorStore';

  // Props
  export let currentUserId: string | null | undefined = undefined; // Added as per plan

  // --- Organizations State & Logic ---
  interface Org { id: string; name: string; api_key: string; }
  let orgs: Org[] = [];
  let newOrgName = '';
  let showCreateOrgModal = false;
  let newlyCreatedOrgWithKey: Org | null = null; // For displaying the new API key

  function openCreateOrgModal() {
    newOrgName = '';
    showCreateOrgModal = true;
  }
  function closeCreateOrgModal() {
    showCreateOrgModal = false;
  }
  async function loadOrgs() {
    // ... (existing loadOrgs logic - keeping it the same) ...
    try {
      const res = await apiFetch('/api/orgs'); // Use apiFetch
      if (res.ok) {
        orgs = await res.json();
      } else {
        console.error('Failed to load organizations:', await res.text());
        errorStore.show('Error: Could not load organizations.');
      }
    } catch (error) {
      console.error('Error loading organizations:', error);
      errorStore.show('Error: Could not load organizations.');
    }
  }
  async function createOrgInModal() {
    // ... (existing createOrgInModal logic - keeping it the same) ...
    if (!newOrgName.trim()) {
      errorStore.show('Organization name cannot be empty.');
      return;
    }
    try {
      const res = await apiFetch('/api/orgs', { // Use apiFetch
        method: 'POST',
        // headers: { 'Content-Type': 'application/json' }, // apiFetch handles this
        body: JSON.stringify({ name: newOrgName })
      });
      if (res.ok) {
        const createdOrgData: Org = await res.json();
        newlyCreatedOrgWithKey = createdOrgData;
        closeCreateOrgModal();
        await loadOrgs();
        // Display the API key for the newly created org
        if (newlyCreatedOrgWithKey) {
          alert(`Organization "${newlyCreatedOrgWithKey.name}" created successfully!\nAPI Key: ${newlyCreatedOrgWithKey.api_key}\n\nPlease copy and save this API key securely. It will not be shown again.`);
          newlyCreatedOrgWithKey = null; // Clear after displaying
        }
      } else {
        const errorText = await res.text();
        console.error('Failed to create organization:', errorText);
        errorStore.show('Error: Could not create organization. ' + errorText);
      }
    } catch (error) {
      console.error('Error creating organization:', error);
      errorStore.show('Error: Could not create organization. See console for details.');
    }
  }
  function maskApiKey(apiKey: string) {
    if (!apiKey) return '';
    return apiKey.substring(0, 8) + '...';
  }

  // --- User Management State & Logic ---
  interface AdminUserView {
    id: string;
    email: string;
    role: string;
    org_id: string;
    organization_name: string | null;
    confirmed: boolean;
    is_active: boolean;
    deactivated_at?: string | null;
    created_at: string; // New field for registration date
  }
  let allUsers: AdminUserView[] = [];
  let isLoadingUsers = false;
  let usersError: string | null = null;
  let currentUserPage = 1;
  let totalUsers = 0;
  let usersPerPage = 10;
  let totalUserPages = 0;
  let showEditRoleModal = false;
  let editingUser: AdminUserView | null = null;

  // State for ConfirmationModal
  let showConfirmationModal = false;
  let confirmationTitle = '';
  let confirmationMessage = '';
  let confirmAction: (() => Promise<void>) | null = null;
  let confirmButtonText = 'Confirm';
  type ButtonVariant = 'primary' | 'secondary' | 'danger' | 'ghost' | 'link';
  let confirmVariant: ButtonVariant = 'primary';

  // State for InviteUserModal
  let showInviteUserModal = false;

  function openInviteUserModal() {
    showInviteUserModal = true;
  }


  async function requestDeactivateUser(user: AdminUserView) {
    confirmationTitle = 'Deactivate User';
    confirmationMessage = `Are you sure you want to deactivate the user "${user.email}"? They will no longer be able to log in.`;
    confirmButtonText = 'Deactivate User';
    confirmVariant = 'danger';
    confirmAction = async () => {
      try {
        const response = await apiFetch(`/api/admin/users/${user.id}/deactivate`, { method: 'POST' }); // Use apiFetch
        const data = await response.json();
        if (!response.ok) throw new Error(data.error || 'Failed to deactivate user.');
        alert(data.message || 'User deactivated successfully.');
        loadAllUsers(); // Refresh user list
      } catch (e: any) {
        errorStore.show(`Error: ${e.message}`);
      }
    };
    showConfirmationModal = true;
  }

  async function requestReactivateUser(user: AdminUserView) {
    confirmationTitle = 'Reactivate User';
    confirmationMessage = `Are you sure you want to reactivate the user "${user.email}"? They will be able to log in again.`;
    confirmButtonText = 'Reactivate User';
    confirmVariant = 'primary'; // Not a "danger" action
    confirmAction = async () => {
      try {
        const response = await apiFetch(`/api/admin/users/${user.id}/reactivate`, { method: 'POST' }); // Use apiFetch
        const data = await response.json();
        if (!response.ok) throw new Error(data.error || 'Failed to reactivate user.');
        alert(data.message || 'User reactivated successfully.');
        loadAllUsers(); // Refresh user list
      } catch (e: any) {
        errorStore.show(`Error: ${e.message}`);
        console.error("Error reactivating user:", e);
      }
    };
    showConfirmationModal = true;
  }

  async function handleResendConfirmation(userId: string, userEmail: string) {
    // Using built-in confirm for this one as it's less critical than deactivation
    if (!confirm(`Are you sure you want to resend the confirmation email to ${userEmail}?`)) {
        return;
    }
    try {
      const response = await apiFetch(`/api/admin/users/${userId}/resend_confirmation`, { // Use apiFetch
        method: 'POST',
      });
      const data = await response.json();

      if (!response.ok) {
        throw new Error(data.error || `Failed to resend confirmation: ${response.statusText}`);
      }
      alert(data.message || 'Confirmation email has been resent successfully!');
      // No need to call loadAllUsers() here, as this action doesn't change the 'confirmed' status directly.
      // The user list will reflect the current 'confirmed' status from the DB.
    } catch (e: any) {
      console.error("Error resending confirmation email:", e);
      errorStore.show(`Error: ${e.message}`);
    }
  }

  function openEditRoleModal(userToEdit: AdminUserView) {
    // Backend prevents self-update and demoting last admin.
    // Client-side check for UX is good but not strictly a security measure here.
    // For instance, preventing edits on any 'admin' user via this UI:
    if (userToEdit.role === 'admin') {
        errorStore.show("Global admin roles cannot be modified through this interface.");
        return;
    }
    editingUser = userToEdit;
    showEditRoleModal = true;
  }

  async function loadAllUsers(pageToLoad = 1) {
    isLoadingUsers = true;
    usersError = null;
    currentUserPage = pageToLoad;
    try {
      const response = await apiFetch(`/api/admin/users?page=${pageToLoad}&limit=${usersPerPage}`);
      if (!response.ok) {
        const errData = await response.json().catch(() => ({}));
        throw new Error(errData.error || `Failed to fetch users: ${response.statusText}`);
      }
      const data = await response.json();
      allUsers = data.items;
      totalUsers = data.total_items;
      usersPerPage = data.per_page;
      totalUserPages = data.total_pages;
      currentUserPage = data.page;
    } catch (e: any) {
      usersError = e.message;
      allUsers = [];
      totalUsers = 0;
      totalUserPages = 0;
      console.error("Error loading users:", e);
    } finally {
      isLoadingUsers = false;
    }
  }

  function handleUserPageChange(event: CustomEvent<{ page: number }>) {
    if (event.detail.page !== currentUserPage) {
      loadAllUsers(event.detail.page);
    }
  }

  const orgTableHeaders: TableHeader[] = [
    { key: 'name', label: 'Name', cellClass: '!text-gray-200 group-hover:!text-accent-lighter', sortable: true },
    { key: 'id', label: 'ID', cellClass: 'font-mono !text-xs !text-gray-400', sortable: false },
    { key: 'api_key', label: 'API Key (Masked)', sortable: false }
  ];

  const userTableHeaders: TableHeader[] = [
    { key: 'email', label: 'Email', sortable: true, cellClass: 'font-medium !text-gray-100 group-hover:!text-accent-lighter truncate' },
    { key: 'role', label: 'Role', sortable: true, cellClass: '!text-gray-300' },
    { key: 'organization_name', label: 'Organization', sortable: true, cellClass: '!text-gray-300 group-hover:!text-accent-lighter truncate' },
    { key: 'status', label: 'Account Status', sortable: true },
    { key: 'confirmed', label: 'Email Confirmed', sortable: true },
    { key: 'created_at', label: 'Registered On', sortable: true }, // New column
    { key: 'actions', label: 'Actions', headerClass: 'text-right', cellClass: 'text-right !whitespace-normal', sortable: false }
  ];

  // --- Tab Control ---
  let currentAdminView: 'organizations' | 'users' = 'organizations';

  onMount(() => {
    if (currentAdminView === 'organizations') {
      loadOrgs();
    }
    // loadAllUsers will be called reactively when tab changes
  });

  // Load data when tab becomes visible and data hasn't been loaded
  $: if (currentAdminView === 'users' && allUsers.length === 0 && !isLoadingUsers && !usersError) {
      loadAllUsers();
  }
  $: if (currentAdminView === 'organizations' && orgs.length === 0 && !isLoadingUsers) { // Assuming no separate isLoadingOrgs
      loadOrgs();
  }

</script>

<GlassCard padding="p-0" customClass="overflow-hidden"> <!-- Changed padding to p-0, internal divs will handle it -->
  <div class="p-4 sm:p-6 border-b border-neutral-700/50"> <!-- Header for title and create org button -->
    <div class="flex justify-between items-center">
      <h2 class="text-xl font-semibold text-gray-100">Admin Panel</h2>
      {#if currentAdminView === 'organizations'}
        <Button variant="primary" on:click={openCreateOrgModal}>Create New Organization</Button>
      {:else if currentAdminView === 'users'}
        <Button variant="primary" on:click={openInviteUserModal}>Invite New User</Button>
      {/if}
    </div>
  </div>

  <!-- Tab Navigation -->
  <div class="px-4 sm:px-6 border-b border-neutral-700/50 flex">
    <button
      on:click={() => currentAdminView = 'organizations'}
      class="px-4 py-3 -mb-px border-b-2 font-medium text-sm transition-colors duration-150
             {currentAdminView === 'organizations' ? 'border-accent text-accent' : 'border-transparent text-gray-400 hover:text-gray-200 hover:border-gray-500/50'}">
      Organizations
    </button>
    <button
      on:click={() => currentAdminView = 'users'}
      class="px-4 py-3 -mb-px border-b-2 font-medium text-sm transition-colors duration-150
             {currentAdminView === 'users' ? 'border-accent text-accent' : 'border-transparent text-gray-400 hover:text-gray-200 hover:border-gray-500/50'}">
      User Management
    </button>
  </div>

  <div class="p-4 sm:p-6 space-y-6"> <!-- Main content area for tabs, increased space-y-4 to space-y-6 -->
    {#if currentAdminView === 'organizations'}
      <h3 class="text-lg font-semibold text-gray-200">Manage Organizations</h3> <!-- Removed mb-3, parent space-y-6 will handle -->
      <!-- Orgs Table -->
      <DataTable
        headers={orgTableHeaders}
        items={orgs.map(org => ({...org, api_key: maskApiKey(org.api_key)}))}
        keyField="id"
        emptyStateMessage="No organizations found. Create one to get started."
        emptyStateIconPath="M3.75 21h16.5M4.5 3h15M5.25 3v18m13.5-18v18M9 6.75h6M9 11.25h6M9 15.75h6"
        tableContainerClass="overflow-hidden shadow-md rounded-lg border border-neutral-700/50 bg-neutral-800/30 backdrop-blur-sm"
        tableClass="min-w-full divide-y divide-neutral-700/30"
        thClass="px-4 py-2.5 text-left text-xs font-medium text-gray-400 uppercase tracking-wider bg-neutral-700/20"
        trClass="hover:bg-neutral-700/40 transition-colors duration-150 group"
        tdClass="px-4 py-3 whitespace-nowrap text-sm text-gray-300"
      >
        <span slot="cell-id" let:value title={value}>{value.substring(0,8)}...</span>
        <span slot="cell-api_key" let:value title="Full key is masked for security">{value}</span>
      </DataTable>

    {:else if currentAdminView === 'users'}
      <h3 class="text-lg font-semibold text-gray-200">Manage System Users</h3> <!-- Removed mb-3, parent space-y-6 will handle -->
      {#if isLoadingUsers}
        <p class="text-gray-400 text-center py-5">Loading users...</p>
      {:else if usersError}
        <p class="text-error bg-error/10 p-3 rounded-md text-center">Error loading users: {usersError}</p>
      <!-- DataTable will now show its own empty state if allUsers is empty and not loading/error -->
      {:else}
        <DataTable
            headers={userTableHeaders}
            items={allUsers}
            keyField="id"
            currentPage={currentUserPage}
            totalPages={totalUserPages}
            totalItems={totalUsers}
            itemsPerPage={usersPerPage}
            tableContainerClass="overflow-hidden shadow-md rounded-lg border border-neutral-700/50 bg-neutral-800/30 backdrop-blur-sm"
            tableClass="min-w-full divide-y divide-neutral-700/30"
            emptyStateMessage="No users found in the system."
            emptyStateIconPath="M15 19.128a9.38 9.38 0 002.625.372M7.5 0A4.5 4.5 0 003 4.5v.75A.75.75 0 004.5 6h4.5a.75.75 0 00.75-.75v-.75A4.5 4.5 0 007.5 0zm0 9a4.5 4.5 0 00-4.5 4.5v.75a.75.75 0 00.75.75h7.5a.75.75 0 00.75-.75v-.75A4.5 4.5 0 007.5 9zm-2.625 5.628a9.37 9.37 0 01-2.625.372m16.5 0a9.37 9.37 0 01-2.625-.372M12 21a9.375 9.375 0 01-3-1.372A9.375 9.375 0 013 21m18 0a9.375 9.375 0 01-3 1.372A9.375 9.375 0 0121 21m-9-1.628c.394.06.794.1.9.1.106 0 .506-.04.9-.1M12 12a3 3 0 11-6 0 3 3 0 016 0zm6 0a3 3 0 11-6 0 3 3 0 016 0z"
            thClass="px-4 py-2.5 text-left text-xs font-medium text-gray-400 uppercase tracking-wider bg-neutral-700/20"
            trClass="hover:bg-neutral-700/40 transition-colors duration-150 group"
            tdClass="px-4 py-3 whitespace-nowrap text-sm text-gray-300"
        >
          <span slot="cell-confirmed" let:item>
            {#if item.confirmed}
              <span class="px-2.5 py-0.5 inline-flex text-xs leading-5 font-semibold rounded-full bg-success/20 text-success">
                Confirmed
              </span>
            {:else}
              <span class="px-2.5 py-0.5 inline-flex text-xs leading-5 font-semibold rounded-full bg-error/20 text-error">
                Pending
              </span>
            {/if}
          </span>
          <span slot="cell-email" let:value title={value} class="block max-w-xs group-hover:text-accent-lighter transition-colors truncate">{value}</span>
          <span slot="cell-organization_name" let:value title={value} class="block max-w-xs group-hover:text-accent-lighter transition-colors truncate">{value || 'N/A'}</span>

          <span slot="cell-status" let:item>
            {#if item.is_active}
              <span class="px-2.5 py-0.5 inline-flex text-xs leading-5 font-semibold rounded-full bg-success/20 text-success">
                Active
              </span>
            {:else}
              <span class="px-2.5 py-0.5 inline-flex text-xs leading-5 font-semibold rounded-full bg-amber-500/20 text-amber-100">
                Deactivated
                {#if item.deactivated_at}
                  <span class="ml-1 font-light text-xs text-gray-400 dark:text-gray-500 hidden sm:inline">({new Date(item.deactivated_at).toLocaleDateString()})</span>
                {/if}
              </span>
            {/if}
          </span>

          <span slot="cell-created_at" let:item class="text-xs text-gray-300 dark:text-gray-400">
            {new Date(item.created_at).toLocaleDateString('en-CA', { year: 'numeric', month: 'short', day: 'numeric' })}
            <span class="block text-[0.65rem] text-gray-400 dark:text-gray-500">
                {new Date(item.created_at).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}
            </span>
          </span>

          <div slot="cell-actions" let:item class="flex justify-end items-center space-x-1">
            {#if item.id === currentUserId}
                <span class="text-xs text-gray-500 italic px-2 py-1">(Your Account)</span>
            {:else if item.role === 'admin'}
                {#if allUsers.filter(u => u.role === 'admin' && u.is_active).length <= 1 && item.is_active}
                    <span class="text-xs text-orange-400 italic px-2 py-1">(Last Active Admin)</span>
                {:else}
                    <span class="text-xs text-gray-500 italic px-2 py-1">(Admin Role)</span>
                {/if}
            {:else} <!-- Not current user, not an admin -->
                <Button variant="ghost" customClass="!px-2 !py-1 text-xs" on:click={() => openEditRoleModal(item)} title="Edit Role">
                    Edit Role
                </Button>
                {#if item.is_active}
                    <Button variant="ghost" customClass="!px-2 !py-1 text-xs !text-orange-400 hover:!text-orange-300 hover:!bg-orange-500/10"
                            on:click={() => requestDeactivateUser(item)} title="Deactivate User Account">
                        Deactivate
                    </Button>
                {:else}
                    <Button variant="ghost" customClass="!px-2 !py-1 text-xs !text-green-400 hover:!text-green-300 hover:!bg-green-500/10"
                            on:click={() => requestReactivateUser(item)} title="Reactivate User Account">
                        Reactivate
                    </Button>
                {/if}
            {/if}

            {#if !item.confirmed && item.is_active && item.id !== currentUserId && item.role !== 'admin'}
                 <Button variant="ghost" customClass="!px-2 !py-1 text-xs !text-sky-400 hover:!text-sky-300 hover:!bg-sky-500/10"
                on:click={() => handleResendConfirmation(item.id, item.email)}
                title="Resend Confirmation Email"
              >
                Resend Email
              </Button>
            {/if}
          </div>
          <div slot="paginationControls" let:currentPageProps let:totalPagesProps>
            <PaginationControls
              currentPage={currentPageProps}
              totalPages={totalPagesProps}
              on:pageChange={handleUserPageChange}
            />
          </div>
        </DataTable>
      {/if}
    {/if}
  </div>

  <!-- Modal for Creating Organization (remains the same) -->
  <Modal isOpen={showCreateOrgModal} title="Create New Organization" on:close={closeCreateOrgModal}>
    <div slot="content" class="space-y-3 py-2">
      <form on:submit|preventDefault={createOrgInModal} id="createOrgForm">
        <div>
          <label for="newOrgNameModal" class="block text-sm font-medium text-gray-700 mb-1">Organization Name</label>
          <input
            type="text"
            id="newOrgNameModal"
            bind:value={newOrgName}
            class="glass-input w-full"
            placeholder="Enter organization name"
            required
          />
        </div>
      </form>
    </div>
    <div slot="footer" class="flex justify-end space-x-2">
      <Button variant="secondary" type="button" on:click={closeCreateOrgModal}>Cancel</Button>
      <Button variant="primary" type="submit" form="createOrgForm">Create</Button>
    </div>
  </Modal>

  {#if showEditRoleModal && editingUser}
    <EditUserRoleModal
      isOpen={showEditRoleModal}
      user={editingUser}
      organizations={orgs}
      on:close={() => showEditRoleModal = false}
      on:role_updated={() => {
        showEditRoleModal = false;
        loadAllUsers(); // Refresh user list
      }}
    />
  {/if}

  {#if showConfirmationModal && confirmAction}
    <ConfirmationModal
      isOpen={showConfirmationModal}
      title={confirmationTitle}
      message={confirmationMessage}
      confirmText={confirmButtonText}
      confirmVariant={confirmVariant}
      on:confirm={() => { if (confirmAction) confirmAction(); showConfirmationModal = false; }}
      on:close={() => showConfirmationModal = false}
    />
  {/if}

  {#if showInviteUserModal}
    <InviteUserModal
      isOpen={showInviteUserModal}
      organizations={orgs}
      on:close={() => showInviteUserModal = false}
      on:user_invited={() => {
        showInviteUserModal = false;
        loadAllUsers(); // Refresh user list
      }}
    />
  {/if}
</GlassCard>
