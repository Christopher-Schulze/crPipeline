<!-- frontend/src/lib/components/InviteUserModal.svelte -->
<script lang="ts">
  import { createEventDispatcher, onMount } from 'svelte';
  import Modal from './Modal.svelte';
  import Button from './Button.svelte';
  import { apiFetch } from '$lib/utils/apiUtils'; // Import apiFetch
  // Assuming Organization type is defined in a shared location or passed if OrgAdmin exports it
  // For this example, let's define it locally or assume it's available globally/imported
  export interface Organization {
    id: string;
    name: string;
    // api_key?: string; // Not needed for this modal's direct logic
  }

  export let isOpen: boolean = false;
  export let organizations: Organization[] = [];
  export let isOrgAdminInvite: boolean = false;
  export let defaultOrgId: string | undefined = undefined;

  let email: string = '';
  let selectedRole: string = 'user';
  let selectedOrgId: string | undefined = undefined;
  let errorMsg: string | null = null;
  let successMsg: string | null = null;
  let isLoading: boolean = false;

  const dispatch = createEventDispatcher();

  function resetForm() {
    email = '';
    // Respect isOrgAdminInvite on reset
    if (isOrgAdminInvite && defaultOrgId) {
      selectedRole = 'user';
      selectedOrgId = defaultOrgId;
    } else {
      selectedRole = 'user'; // Default for global admin
      selectedOrgId = organizations.length === 1 && !isOrgAdminInvite ? organizations[0].id : undefined; // Pre-select if only one org for global admin
    }
    errorMsg = null;
    successMsg = null;
  }

  // Reset form when modal is closed externally (e.g., by Escape key or backdrop click)
  // This is triggered when isOpen becomes false AFTER it was true.
  let prevIsOpen = isOpen;
  $: if (prevIsOpen && !isOpen) {
      setTimeout(resetForm, 300); // Delay to allow modal close animation
  }
  $: prevIsOpen = isOpen; // Keep track of previous isOpen state

  async function handleSubmit() {
    isLoading = true;
    errorMsg = null;
    successMsg = null;

    if (!email || !email.includes('@')) {
      errorMsg = "Please enter a valid email address.";
      isLoading = false;
      return;
    }
    if (selectedRole === 'org_admin' && !selectedOrgId) {
      errorMsg = "An organization must be selected for an 'org_admin'.";
      isLoading = false;
      return;
    }
    // Backend now requires org_id for 'user' role too on invite
    if (selectedRole === 'user' && !selectedOrgId) {
        errorMsg = "An organization must be selected for a 'user'.";
        isLoading = false;
        return;
    }

    let apiPath: string;
    let apiPayload: any;

    if (isOrgAdminInvite) {
      apiPath = `/api/organizations/me/invite`;
      apiPayload = { email }; // Org admin invites implicitly set role='user' and use their own org_id
    } else {
      apiPath = `/api/admin/invite`;
      apiPayload = {
        email,
        role: selectedRole,
        org_id: selectedOrgId,
      };
    }

    try {
      const response = await apiFetch(apiPath, {
        method: 'POST',
        body: JSON.stringify(apiPayload),
      });
      const data = await response.json();
      if (!response.ok) {
        throw new Error(data.error || `Failed to invite user: ${response.statusText}`);
      }
      successMsg = data.message || 'User invitation sent successfully!';
      dispatch('user_invited');
      // Don't resetForm() immediately here if we want success message to show
      // Parent will close modal, which will then trigger resetForm via the reactive $: block
      // Or if modal stays open, user can see success and then close.
      // Let's clear form for next potential invite *if* modal stays open by choice.
      // For now, assume parent closes it or user closes it.
    } catch (e: any) {
      errorMsg = e.message;
      console.error("Failed to invite user:", e);
    } finally {
      isLoading = false;
    }
  }

  function closeModalAndReset() { // Renamed to be more descriptive
    dispatch('close');
    // setTimeout for reset is now handled by the reactive $: block watching isOpen
  }

  // Reset form if modal is opened AND it was previously closed (to ensure fresh state)
  // This handles reopening the modal after it was closed.
  // The initial `prevIsOpen` is `false` (if `isOpen` starts as `false`).
  // When `isOpen` becomes `true` for the first time or after being `false`, `prevIsOpen` is still `false`.
  $: if (isOpen && !prevIsOpen) { // Condition to run only when opening/reopening
    resetForm(); // Apply initial state or reset based on props
    if (isOrgAdminInvite) {
      selectedRole = 'user';
      if (defaultOrgId) {
        selectedOrgId = defaultOrgId;
      } else if (organizations.length === 1) {
        // Fallback if defaultOrgId somehow not provided but organizations list has the one
        selectedOrgId = organizations[0].id;
      }
    } else {
      // For global admin, if there's only one org, pre-select it.
      if (organizations.length === 1) {
        selectedOrgId = organizations[0].id;
      }
    }
  }

  // Ensure selectedOrgId is valid if organizations list changes or defaultOrgId changes (for org admin)
  $: if (isOrgAdminInvite && defaultOrgId && organizations.length > 0 && organizations[0].id === defaultOrgId) {
    selectedOrgId = defaultOrgId;
    selectedRole = 'user';
  } else if (isOrgAdminInvite && organizations.length > 0 && !defaultOrgId) {
    // If defaultOrgId is not set but it's an org admin invite and there's one org, use it.
    selectedOrgId = organizations[0].id;
    selectedRole = 'user';
  }

</script>

<Modal isOpen={isOpen} title="Invite New User" on:close={closeModalAndReset} maxWidth="max-w-lg">
  <div slot="content" class="space-y-4">
    {#if successMsg && !errorMsg} <!-- Show success only if no new error occurred -->
      <p class="text-sm text-green-400 bg-green-500/20 p-3 rounded-md">{successMsg}</p>
    {/if}
    {#if errorMsg}
      <p class="text-sm text-error bg-error/20 p-3 rounded-md">{errorMsg}</p>
    {/if}

    <form on:submit|preventDefault={handleSubmit} id="inviteUserForm" class="space-y-4">
      <div>
        <label for="invite-email" class="block text-sm font-medium text-gray-300 mb-1">User Email</label>
        <input type="email" id="invite-email" bind:value={email} class="glass-input w-full !text-sm" placeholder="user@example.com" required />
      </div>

      <div>
        <label for="invite-role-select" class="block text-sm font-medium text-gray-300 mb-1">Assign Role</label>
        <select id="invite-role-select" bind:value={selectedRole} class="glass-input w-full !text-sm" disabled={isOrgAdminInvite}>
          {#if isOrgAdminInvite}
            <option value="user" selected>User (managed by your organization)</option>
          {:else}
            <option value="user">User</option>
            <option value="org_admin">Organization Admin</option>
          {/if}
        </select>
      </div>

      <div>
        <label for="invite-org-select" class="block text-sm font-medium text-gray-300 mb-1">
          Assign to Organization {#if !isOrgAdminInvite && (selectedRole === 'org_admin' || selectedRole === 'user')} (Required){/if}
          {#if isOrgAdminInvite && organizations.length > 0} (Auto-selected: {organizations[0].name}) {/if}
        </label>
        {#if organizations.length > 0}
           <select
             id="invite-org-select"
             bind:value={selectedOrgId}
             class="glass-input w-full !text-sm"
             required={(selectedRole === 'org_admin' || selectedRole === 'user')}
             disabled={isOrgAdminInvite || (!isOrgAdminInvite && organizations.length === 1 && selectedRole !== 'org_admin')}
             aria-readonly={isOrgAdminInvite}
           >
             {#if !isOrgAdminInvite && organizations.length > 1}
               <option value={undefined}>Select Organization...</option>
             {/if}
             {#each organizations as org (org.id)}
               <option value={org.id} selected={selectedOrgId === org.id}>
                 {org.name}
                 {#if !isOrgAdminInvite}(ID: ...{org.id.slice(-8)}){/if}
               </option>
               {/each}
           </select>
        {:else if !isOrgAdminInvite}
           <p class="text-xs text-gray-400 bg-neutral-700/30 p-2 rounded-md">No organizations available. Global admins can create them in the 'Organizations' tab.</p>
        {:else} <!-- isOrgAdminInvite but no organizations passed (should not happen based on UserManagementView logic) -->
           <p class="text-xs text-error bg-error/20 p-2 rounded-md">Error: Organization details are missing for this invitation.</p>
        {/if}
      </div>
    </form>
  </div>
  <div slot="footer" class="flex justify-end space-x-3">
    <Button variant="secondary" on:click={closeModalAndReset} disabled={isLoading}>Cancel</Button>
    <Button
      variant="primary"
      type="submit"
      form="inviteUserForm"
      disabled={isLoading || !email || !selectedOrgId || (isOrgAdminInvite && selectedRole !== 'user') }
    >
      {isLoading ? 'Sending Invite...' : 'Send Invitation'}
    </Button>
  </div>
</Modal>
