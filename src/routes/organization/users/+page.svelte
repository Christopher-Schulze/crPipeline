<script lang="ts">
  import UserManagementView from '$lib/components/UserManagementView.svelte';
  import { page } from '$app/stores';
  import GlassCard from '$lib/components/GlassCard.svelte';
  import { fade } from 'svelte/transition';

  // $: session = $page.data.session; // For easier access if used multiple times
  $: orgId = $page.data.session?.org_id; // Corrected from .org to .org_id based on typical session structure
  $: orgName = $page.data.session?.organization_name; // Corrected from .orgName to .organization_name
  $: loggedIn = $page.data.session?.logged_in; // Corrected from .loggedIn to .logged_in
  $: currentRole = $page.data.session?.role;

  // Fallback for orgName if not in session, UserManagementView has its own default too.
  // This provides a slightly more specific fallback if orgId is present.
  let displayOrgName: string;
  $: displayOrgName = orgName || (orgId ? `Organization ID: ${orgId.substring(0,8)}...` : "Your Organization");

</script>

<div class="container mx-auto px-4 py-8" in:fade={{ duration: 200 }}>
  {#if loggedIn && (currentRole === 'admin' || currentRole === 'org_admin') && orgId}
    <UserManagementView bind:orgId={orgId} bind:orgName={displayOrgName} />
  {:else if loggedIn && !(currentRole === 'admin' || currentRole === 'org_admin')}
    <GlassCard title="Access Denied" padding="p-6 md:p-8">
      <div class="flex flex-col items-center text-center">
        <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-16 h-16 text-error mb-4">
          <path stroke-linecap="round" stroke-linejoin="round" d="M18.364 18.364A9 9 0 005.636 5.636m12.728 12.728A9 9 0 015.636 5.636m12.728 12.728L5.636 5.636" />
        </svg>
        <h2 class="text-2xl font-semibold text-gray-100 mb-2">Permission Required</h2>
        <p class="text-gray-400">
          You do not have the necessary permissions to manage users for this organization.
          Please contact your administrator if you believe this is an error.
        </p>
      </div>
    </GlassCard>
  {:else if loggedIn && !orgId && currentRole === 'org_admin'}
    <GlassCard title="Configuration Error" padding="p-6 md:p-8">
       <div class="flex flex-col items-center text-center">
        <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-16 h-16 text-warning mb-4">
          <path stroke-linecap="round" stroke-linejoin="round" d="M12 9v3.75m-9.303 3.376c-.866 1.5.217 3.374 1.948 3.374h14.71c1.73 0 2.813-1.874 1.948-3.374L13.949 3.378c-.866-1.5-3.032-1.5-3.898 0L2.697 16.126zM12 15.75h.007v.008H12v-.008z" />
        </svg
        <h2 class="text-2xl font-semibold text-gray-100 mb-2">Organization Not Identified</h2>
        <p class="text-gray-400">
          Your user account is configured as an Organization Admin, but no organization ID is associated with your session.
          Please log out and log back in. If the issue persists, contact support.
        </p>
      </div>
    </GlassCard>
  {:else}
    <GlassCard title="User Management" padding="p-6 md:p-8">
      <div class="flex flex-col items-center text-center">
        <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-16 h-16 text-primary-400 mb-4">
          <path stroke-linecap="round" stroke-linejoin="round" d="M16.5 10.5V6.75a4.5 4.5 0 10-9 0v3.75m-.75 11.25h10.5a2.25 2.25 0 002.25-2.25v-6.75a2.25 2.25 0 00-2.25-2.25H6.75a2.25 2.25 0 00-2.25 2.25v6.75a2.25 2.25 0 002.25 2.25z" />
        </svg>
        <h2 class="text-2xl font-semibold text-gray-100 mb-2">Authentication Required</h2>
        <p class="text-gray-400">Please log in to manage users for your organization.</p>
        <Button href="/login" variant="primary" customClass="mt-6">Go to Login</Button>
      </div>
    </GlassCard>
  {/if}
</div>
</html>
