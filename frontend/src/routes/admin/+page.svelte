<script lang="ts">
  import { onMount } from 'svelte';
  import { sessionStore } from '$lib/stores/session';
  import GlassCard from '$lib/components/GlassCard.svelte';
  import Button from '$lib/components/Button.svelte';
  import DataTable, { type TableHeader } from '$lib/components/DataTable.svelte';
  import PaginationControls from '$lib/components/PaginationControls.svelte';
  import EditUserRoleModal from '$lib/components/EditUserRoleModal.svelte';
  import ConfirmationModal from '$lib/components/ConfirmationModal.svelte';
  import InviteUserModal from '$lib/components/InviteUserModal.svelte';
  import Modal from '$lib/components/Modal.svelte';
  import EditOrganizationModal from '$lib/components/EditOrganizationModal.svelte';
  import { apiFetch } from '$lib/utils/apiUtils';

  // Tab handling
  type AdminTab = 'users' | 'roles' | 'invites' | 'orgs';
  let currentTab: AdminTab = 'users';

  // Organization data
  interface Org { id: string; name: string; api_key: string; }
  let orgs: Org[] = [];
  let newOrgName = '';
  let showCreateOrgModal = false;
  let newlyCreatedOrgWithKey: Org | null = null;
  let showEditOrgModal = false;
  let editingOrg: Org | null = null;

  function openCreateOrgModal() {
    newOrgName = '';
    showCreateOrgModal = true;
  }
  function closeCreateOrgModal() {
    showCreateOrgModal = false;
  }
  function openEditOrgModal(org: Org) {
    editingOrg = org;
    showEditOrgModal = true;
  }
  function closeEditOrgModal() {
    showEditOrgModal = false;
  }
  async function loadOrgs() {
    try {
      const res = await apiFetch('/api/orgs');
      if (res.ok) {
        orgs = await res.json();
      } else {
        console.error('Failed to load organizations:', await res.text());
      }
    } catch (e) {
      console.error('Error loading organizations:', e);
    }
  }
  async function createOrgInModal() {
    if (!newOrgName.trim()) return;
    try {
      const res = await apiFetch('/api/orgs', { method: 'POST', body: JSON.stringify({ name: newOrgName }) });
      if (res.ok) {
        const created: Org = await res.json();
        newlyCreatedOrgWithKey = created;
        closeCreateOrgModal();
        await loadOrgs();
        if (newlyCreatedOrgWithKey) {
          alert(`Organization "${newlyCreatedOrgWithKey.name}" created!\nAPI Key: ${newlyCreatedOrgWithKey.api_key}`);
          newlyCreatedOrgWithKey = null;
        }
      } else {
        console.error('Failed to create organization:', await res.text());
      }
    } catch (e) {
      console.error('Error creating organization:', e);
    }
  }

  function maskApiKey(apiKey: string) {
    return apiKey ? apiKey.substring(0,8) + '...' : '';
  }

  // User data
  interface AdminUserView {
    id: string;
    email: string;
    role: string;
    org_id: string;
    organization_name: string | null;
    confirmed: boolean;
    is_active: boolean;
    deactivated_at?: string | null;
    created_at: string;
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

  let showConfirmationModal = false;
  let confirmationTitle = '';
  let confirmationMessage = '';
  let confirmAction: (() => Promise<void>) | null = null;
  let confirmButtonText = 'Confirm';
  type ButtonVariant = 'primary' | 'secondary' | 'danger' | 'ghost' | 'link';
  let confirmVariant: ButtonVariant = 'primary';

  let showInviteUserModal = false;
  function openInviteUserModal() { showInviteUserModal = true; }

  async function requestDeactivateUser(user: AdminUserView) {
    confirmationTitle = 'Deactivate User';
    confirmationMessage = `Deactivate ${user.email}?`;
    confirmButtonText = 'Deactivate';
    confirmVariant = 'danger';
    confirmAction = async () => {
      try {
        const response = await apiFetch(`/api/admin/users/${user.id}/deactivate`, { method: 'POST' });
        const data = await response.json();
        if (!response.ok) throw new Error(data.error || 'Failed');
        await loadAllUsers(currentUserPage);
      } catch (e) { console.error(e); }
    };
    showConfirmationModal = true;
  }
  async function requestReactivateUser(user: AdminUserView) {
    confirmationTitle = 'Reactivate User';
    confirmationMessage = `Reactivate ${user.email}?`;
    confirmButtonText = 'Reactivate';
    confirmVariant = 'primary';
    confirmAction = async () => {
      try {
        const response = await apiFetch(`/api/admin/users/${user.id}/reactivate`, { method: 'POST' });
        const data = await response.json();
        if (!response.ok) throw new Error(data.error || 'Failed');
        await loadAllUsers(currentUserPage);
      } catch (e) { console.error(e); }
    };
    showConfirmationModal = true;
  }
  async function handleResendConfirmation(userId: string) {
    try {
      const response = await apiFetch(`/api/admin/users/${userId}/resend_confirmation`, { method: 'POST' });
      await response.json();
    } catch (e) { console.error(e); }
  }
  function openEditRoleModal(user: AdminUserView) {
    if (user.role === 'admin') return;
    editingUser = user;
    showEditRoleModal = true;
  }
  async function loadAllUsers(pageToLoad = 1) {
    isLoadingUsers = true;
    usersError = null;
    currentUserPage = pageToLoad;
    try {
      const res = await apiFetch(`/api/admin/users?page=${pageToLoad}&limit=${usersPerPage}`);
      if (!res.ok) {
        const err = await res.json().catch(() => ({}));
        throw new Error(err.error || 'Failed to fetch users');
      }
      const data = await res.json();
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
    } finally {
      isLoadingUsers = false;
    }
  }
  function handleUserPageChange(e: CustomEvent<{ page: number }>) {
    if (e.detail.page !== currentUserPage) loadAllUsers(e.detail.page);
  }

  const userTableHeaders: TableHeader[] = [
    { key: 'email', label: 'Email', sortable: true, cellClass: 'font-medium !text-gray-100 group-hover:!text-accent-lighter truncate' },
    { key: 'role', label: 'Role', sortable: true, cellClass: '!text-gray-300' },
    { key: 'organization_name', label: 'Organization', sortable: true, cellClass: '!text-gray-300 group-hover:!text-accent-lighter truncate' },
    { key: 'status', label: 'Status', sortable: true },
    { key: 'confirmed', label: 'Confirmed', sortable: true },
    { key: 'created_at', label: 'Registered', sortable: true },
  ];
  const roleTableHeaders: TableHeader[] = [...userTableHeaders, { key: 'actions', label: 'Actions', headerClass: 'text-right', cellClass: 'text-right !whitespace-normal', sortable: false }];

  const orgTableHeaders: TableHeader[] = [
    { key: 'name', label: 'Name', cellClass: '!text-gray-200 group-hover:!text-accent-lighter', sortable: true },
    { key: 'id', label: 'ID', cellClass: 'font-mono !text-xs !text-gray-400', sortable: false },
    { key: 'api_key', label: 'API Key', sortable: false },
    { key: 'actions', label: 'Actions', headerClass: 'text-right', cellClass: 'text-right', sortable: false }
  ];

  onMount(() => {
    loadOrgs();
    loadAllUsers();
  });
</script>

<div class="space-y-6">
  <div class="border-b border-neutral-700/50 flex space-x-4">
    <button on:click={() => currentTab = 'users'} class="px-4 py-2 -mb-px border-b-2 font-medium text-sm {currentTab === 'users' ? 'border-accent text-accent' : 'border-transparent text-gray-400 hover:text-gray-200 hover:border-gray-500/50'}">Benutzerliste</button>
    <button on:click={() => currentTab = 'roles'} class="px-4 py-2 -mb-px border-b-2 font-medium text-sm {currentTab === 'roles' ? 'border-accent text-accent' : 'border-transparent text-gray-400 hover:text-gray-200 hover:border-gray-500/50'}">Rollenänderungen</button>
    <button on:click={() => currentTab = 'invites'} class="px-4 py-2 -mb-px border-b-2 font-medium text-sm {currentTab === 'invites' ? 'border-accent text-accent' : 'border-transparent text-gray-400 hover:text-gray-200 hover:border-gray-500/50'}">Einladungen</button>
    <button on:click={() => currentTab = 'orgs'} class="px-4 py-2 -mb-px border-b-2 font-medium text-sm {currentTab === 'orgs' ? 'border-accent text-accent' : 'border-transparent text-gray-400 hover:text-gray-200 hover:border-gray-500/50'}">Organisationsübersicht</button>
  </div>

  {#if currentTab === 'users'}
    <GlassCard padding="p-4 sm:p-6" customClass="overflow-hidden">
      <h3 class="text-lg font-semibold text-gray-200 mb-4">Alle Benutzer</h3>
      {#if isLoadingUsers}
        <p class="text-gray-400 text-center py-5">Loading users...</p>
      {:else if usersError}
        <p class="text-red-400 bg-red-500/10 p-3 rounded-md text-center">Error: {usersError}</p>
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
        >
          <span slot="cell-confirmed" let:item>
            {#if item.confirmed}
              <span class="px-2.5 py-0.5 inline-flex text-xs leading-5 font-semibold rounded-full bg-success/20 text-success">Confirmed</span>
            {:else}
              <span class="px-2.5 py-0.5 inline-flex text-xs leading-5 font-semibold rounded-full bg-error/20 text-error">Pending</span>
            {/if}
          </span>
          <span slot="cell-status" let:item>
            {#if item.is_active}
              <span class="px-2.5 py-0.5 inline-flex text-xs leading-5 font-semibold rounded-full bg-success/20 text-success">Active</span>
            {:else}
              <span class="px-2.5 py-0.5 inline-flex text-xs leading-5 font-semibold rounded-full bg-amber-500/20 text-amber-100">Deactivated</span>
            {/if}
          </span>
          <span slot="cell-created_at" let:item class="text-xs">
            {new Date(item.created_at).toLocaleDateString('en-CA')}
          </span>
          <div slot="paginationControls" let:currentPageProps let:totalPagesProps>
            <PaginationControls currentPage={currentPageProps} totalPages={totalPagesProps} on:pageChange={handleUserPageChange} />
          </div>
        </DataTable>
      {/if}
    </GlassCard>
  {:else if currentTab === 'roles'}
    <GlassCard padding="p-4 sm:p-6" customClass="overflow-hidden">
      <div class="flex justify-between items-center mb-4">
        <h3 class="text-lg font-semibold text-gray-200">Benutzer verwalten</h3>
        <Button variant="primary" on:click={() => openInviteUserModal()}>Benutzer einladen</Button>
      </div>
      {#if isLoadingUsers}
        <p class="text-gray-400 text-center py-5">Loading users...</p>
      {:else if usersError}
        <p class="text-red-400 bg-red-500/10 p-3 rounded-md text-center">Error: {usersError}</p>
      {:else}
        <DataTable
          headers={roleTableHeaders}
          items={allUsers}
          keyField="id"
          currentPage={currentUserPage}
          totalPages={totalUserPages}
          totalItems={totalUsers}
          itemsPerPage={usersPerPage}
          tableContainerClass="overflow-hidden shadow-md rounded-lg border border-neutral-700/50 bg-neutral-800/30 backdrop-blur-sm"
          tableClass="min-w-full divide-y divide-neutral-700/30"
        >
          <span slot="cell-confirmed" let:item>
            {#if item.confirmed}
              <span class="px-2.5 py-0.5 inline-flex text-xs leading-5 font-semibold rounded-full bg-success/20 text-success">Confirmed</span>
            {:else}
              <span class="px-2.5 py-0.5 inline-flex text-xs leading-5 font-semibold rounded-full bg-error/20 text-error">Pending</span>
            {/if}
          </span>
          <span slot="cell-status" let:item>
            {#if item.is_active}
              <span class="px-2.5 py-0.5 inline-flex text-xs leading-5 font-semibold rounded-full bg-success/20 text-success">Active</span>
            {:else}
              <span class="px-2.5 py-0.5 inline-flex text-xs leading-5 font-semibold rounded-full bg-amber-500/20 text-amber-100">Deactivated</span>
            {/if}
          </span>
          <span slot="cell-created_at" let:item class="text-xs">
            {new Date(item.created_at).toLocaleDateString('en-CA')}
          </span>
          <div slot="cell-actions" let:item class="flex justify-end items-center space-x-1">
            {#if item.id === $sessionStore.userId}
              <span class="text-xs text-gray-500 italic px-2 py-1">(Your Account)</span>
            {:else if item.role === 'admin'}
              <span class="text-xs text-gray-500 italic px-2 py-1">(Admin)</span>
            {:else}
              <Button variant="ghost" customClass="!px-2 !py-1 text-xs" on:click={() => openEditRoleModal(item)}>Edit Role</Button>
              {#if item.is_active}
                <Button variant="ghost" customClass="!px-2 !py-1 text-xs !text-orange-400" on:click={() => requestDeactivateUser(item)}>Deactivate</Button>
              {:else}
                <Button variant="ghost" customClass="!px-2 !py-1 text-xs !text-green-400" on:click={() => requestReactivateUser(item)}>Reactivate</Button>
              {/if}
            {/if}
            {#if !item.confirmed && item.is_active && item.id !== $sessionStore.userId && item.role !== 'admin'}
              <Button variant="ghost" customClass="!px-2 !py-1 text-xs !text-sky-400" on:click={() => handleResendConfirmation(item.id)}>Resend Email</Button>
            {/if}
          </div>
          <div slot="paginationControls" let:currentPageProps let:totalPagesProps>
            <PaginationControls currentPage={currentPageProps} totalPages={totalPagesProps} on:pageChange={handleUserPageChange} />
          </div>
        </DataTable>
      {/if}
    </GlassCard>
  {:else if currentTab === 'invites'}
    <GlassCard padding="p-4 sm:p-6">
      <h3 class="text-lg font-semibold text-gray-200 mb-4">Benutzer einladen</h3>
      <Button variant="primary" on:click={() => openInviteUserModal()}>Neue Einladung</Button>
    </GlassCard>
  {:else if currentTab === 'orgs'}
    <GlassCard padding="p-4 sm:p-6" customClass="overflow-hidden">
      <div class="flex justify-between items-center mb-4">
        <h3 class="text-lg font-semibold text-gray-200">Organisationen</h3>
        <Button variant="primary" on:click={openCreateOrgModal}>Neue Organisation</Button>
      </div>
      <DataTable
        headers={orgTableHeaders}
        items={orgs.map(o => ({ ...o, api_key: maskApiKey(o.api_key) }))}
        keyField="id"
        tableContainerClass="overflow-hidden shadow-md rounded-lg border border-neutral-700/50 bg-neutral-800/30 backdrop-blur-sm"
        tableClass="min-w-full divide-y divide-neutral-700/30"
      >
        <span slot="cell-id" let:value title={value}>{value.substring(0,8)}...</span>
        <div slot="cell-actions" let:item class="space-x-2">
          <Button variant="ghost" customClass="!px-2 !py-1 text-xs" on:click={() => openEditOrgModal(item)}>Edit</Button>
        </div>
      </DataTable>
    </GlassCard>
  {/if}

  <Modal isOpen={showCreateOrgModal} title="Create New Organization" on:close={closeCreateOrgModal}>
    <div slot="content" class="space-y-3 py-2">
      <form on:submit|preventDefault={createOrgInModal} id="createOrgForm">
        <div>
          <label for="newOrgNameModal" class="block text-sm font-medium text-gray-700 mb-1">Organization Name</label>
          <input type="text" id="newOrgNameModal" bind:value={newOrgName} class="glass-input w-full" required />
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
      on:role_updated={() => { showEditRoleModal = false; loadAllUsers(); }}
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
      on:user_invited={() => { showInviteUserModal = false; loadAllUsers(); }}
    />
  {/if}

  {#if showEditOrgModal && editingOrg}
    <EditOrganizationModal
      isOpen={showEditOrgModal}
      org={editingOrg}
      on:close={closeEditOrgModal}
      on:updated={() => { closeEditOrgModal(); loadOrgs(); }}
    />
  {/if}
</div>
