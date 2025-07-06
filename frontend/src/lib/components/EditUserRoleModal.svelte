<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import Modal from './Modal.svelte';
  import Button from './Button.svelte';
  import { apiFetch } from '$lib/utils/apiUtils'; // Import apiFetch

  // Assuming these types are passed or defined/imported from a shared location.
  // For self-containment in this example:
  export interface AdminUserView {
    id: string;
    email: string;
    role: string;
    org_id: string;
    organization_name: string | null;
    confirmed: boolean;
  }
  export interface Organization {
    id: string;
    name: string;
    api_key: string; // Not used in this modal, but part of the common Org type
  }

  export let isOpen: boolean = false;
  export let user: AdminUserView | null = null;
  export let organizations: Organization[] = [];

  let selectedRole: string = '';
  let selectedOrgId: string | undefined = undefined;
  let editableEmail: string = ''; // New state for email editing
  let errorMsg: string | null = null;
  let isLoading: boolean = false;

  const dispatch = createEventDispatcher();

  // Reactive initialization/reset when modal opens or user prop changes
  $: if (isOpen && user) {
    selectedRole = user.role;
    selectedOrgId = user.org_id || undefined;
    editableEmail = user.email; // Initialize editableEmail
    errorMsg = null;
  } else if (!isOpen && !isLoading) { // Avoid reset if closing due to successful submit that's still processing
    // Consider if auto-resetting on close is desired, or if parent should control reset.
    // For now, let errorMsg clear, other fields persist until new user prop or explicit reset.
    // errorMsg = null;
  }

  async function handleSubmit() {
    if (!user) return;
    isLoading = true;
    errorMsg = null;
    let roleUpdated = false;
    let emailUpdated = false;
    let finalMessage = "";

    // --- Role/Org Assignment part ---
    const currentRole = user.role;
    const currentOrgId = user.org_id || undefined; // Ensure currentOrgId is also undefined if null/empty for comparison

    let roleChanged = selectedRole !== currentRole;
    // orgChanged needs to consider that selectedOrgId might be undefined if "Keep Current" is selected for a 'user'
    // For 'user' role, if selectedOrgId is undefined, it means "don't change org".
    // If selectedOrgId is defined, it means "change to this org".
    let orgChanged = selectedOrgId !== currentOrgId && selectedOrgId !== undefined;

    // Determine if an API call for role/org is needed
    let needsRoleOrgApiCall = roleChanged || (selectedRole === 'org_admin' && selectedOrgId !== currentOrgId) || (selectedRole !== 'org_admin' && selectedOrgId !== undefined && selectedOrgId !== currentOrgId);


    if (selectedRole === 'org_admin' && !selectedOrgId) {
        errorMsg = "An organization must be selected for an 'org_admin'.";
        // No need to set isLoading = false yet, will be done in finally block or if other errors occur
    } else if (needsRoleOrgApiCall) {
        const rolePayload: { role: string; org_id?: string } = { role: selectedRole };
        if (selectedOrgId) {
            rolePayload.org_id = selectedOrgId;
        }
        // If selectedOrgId is undefined (e.g. "Keep current" for 'user' role), org_id is not sent.
        // Backend `assign_role` should preserve org_id if not provided for 'user'.

        try {
            const response = await apiFetch(`/api/admin/users/${user.id}/assign_role`, { // Use apiFetch
            method: 'POST',
            // headers: { 'Content-Type': 'application/json' }, // apiFetch handles this
            body: JSON.stringify(rolePayload),
            });
            const data = await response.json().catch(() => ({ error: "Invalid JSON response from server." }));
            if (!response.ok) throw new Error(data.error || 'Failed to update role/organization.');
            finalMessage += (data.message || 'Role/organization updated. ');
            roleUpdated = true;
        } catch (e: any) {
            errorMsg = (errorMsg ? errorMsg + "\n" : "") + `Role/Org Update Error: ${e.message}`;
        }
    }

    // --- Email Update part ---
    const trimmedEditableEmail = editableEmail.trim();
    if (trimmedEditableEmail && trimmedEditableEmail.toLowerCase() !== user.email.toLowerCase()) {
        if (!trimmedEditableEmail.includes('@') || trimmedEditableEmail.length === 0) {
            errorMsg = (errorMsg ? errorMsg + "\n" : "") + "Invalid new email format provided.";
        } else if (!errorMsg) { // Proceed only if no previous errors (e.g. from role/org part)
            try {
                const emailPayload = { email: trimmedEditableEmail };
                const emailResponse = await apiFetch(`/api/admin/users/${user.id}/profile`, { // Use apiFetch
                    method: 'PUT',
                    // headers: { 'Content-Type': 'application/json' }, // apiFetch handles this
                    body: JSON.stringify(emailPayload),
                });
                const emailData = await emailResponse.json().catch(() => ({ error: "Invalid JSON response from server." }));
                if (!emailResponse.ok) throw new Error(emailData.error || 'Failed to update email.');
                finalMessage += (emailData.message || 'Email updated, confirmation sent. ');
                emailUpdated = true;
            } catch (e: any) {
                errorMsg = (errorMsg ? errorMsg + "\n" : "") + `Email Update Error: ${e.message}`;
            }
        }
    }

    isLoading = false;
    if (!errorMsg && (roleUpdated || emailUpdated)) {
        dispatch('role_updated'); // Re-use existing event to signal refresh
        closeModal(); // Close on success
        alert(finalMessage.trim() || "Update successful.");
    } else if (!errorMsg && !roleUpdated && !emailUpdated) {
        alert("No changes were made to the user's profile or role.");
        // closeModal(); // Optionally close even if no changes, or leave open. Current behavior is to leave open.
    }
    // If errorMsg is set, it will be displayed in the modal, and modal remains open.
  }

  function closeModal() {
    dispatch('close');
  }
</script>

<Modal {isOpen} title="Edit User: {user?.email || ''}" on:close={closeModal} maxWidth="max-w-lg">
  <div slot="content" class="space-y-4 py-2 text-gray-200">
    {#if user}
      <div>
        <label for="edit-email" class="block text-sm font-medium text-gray-300 mb-1">User Email</label>
        <input type="email" id="edit-email" bind:value={editableEmail} class="glass-input w-full !text-sm !bg-neutral-700/60 !border-neutral-600/80 !text-gray-100" placeholder="user@example.com" required />
      </div>

      <div>
        <label for="role-select" class="block text-sm font-medium text-gray-300 mb-1">Role</label>
        <select id="role-select" bind:value={selectedRole} class="glass-input w-full text-sm !bg-neutral-700/60 !border-neutral-600/80 !text-gray-100">
          <option value="user">User</option>
          <option value="org_admin">Organization Admin</option>
          <!-- Global "admin" role cannot be assigned/unassigned via this UI -->
        </select>
      </div>

      <!-- Organization dropdown - always visible -->
      <div class="mt-3">
        <label for="org-select" class="block text-sm font-medium text-gray-300 mb-1">
            Assign to Organization {selectedRole === 'org_admin' ? '(Required for Org Admin)' : '(Optional for User)'}
        </label>
        {#if organizations.length > 0}
          <select id="org-select" bind:value={selectedOrgId} class="glass-input w-full text-sm !bg-neutral-700/60 !border-neutral-600/80 !text-gray-100">
              <option value={undefined}>
                {selectedRole === 'org_admin' ? 'Select Organization...' : "Keep Current / Don't Assign New"}
              </option>
              {#each organizations as org (org.id)}
                  <option value={org.id}>{org.name} (ID: ...{org.id.substring(org.id.length - 6)})</option>
              {/each}
          </select>
        {:else}
          <p class="text-xs text-gray-400 bg-neutral-700/30 p-2 rounded-md">
              No organizations available. Create one first to assign users.
          </p>
        {/if}
      </div>

      {#if errorMsg}
        <p class="text-sm text-error bg-error/30 p-2 rounded-md">{errorMsg}</p>
      {/if}
    {:else}
      <p class="text-gray-400">No user selected or user data is unavailable.</p>
    {/if}
  </div>
  <div slot="footer" class="flex justify-end space-x-2">
    <Button variant="secondary" on:click={closeModal} disabled={isLoading}>Cancel</Button>
    <Button
      variant="primary"
      on:click={handleSubmit}
      disabled={isLoading || !user || (selectedRole === 'org_admin' && !selectedOrgId) || (editableEmail.trim() !== user?.email.trim() && (!editableEmail.trim() || !editableEmail.includes('@'))) }
    >
      {#if isLoading}Saving...{:else}Save Changes{/if}
    </Button>
  </div>
</Modal>
