<script lang="ts">
  import { goto } from '$app/navigation';
  import GlassCard from '$lib/components/GlassCard.svelte';
  import Button from '$lib/components/Button.svelte';
  import { apiFetch } from '$lib/utils/apiUtils';

  let orgId = '';
  let email = '';
  let password = '';
  let error: string | null = null;
  let success: string | null = null;

  async function submit() {
    error = null;
    success = null;
    try {
      const res = await apiFetch('/api/register', {
        method: 'POST',
        body: JSON.stringify({ org_id: orgId, email, password })
      });
      if (res.ok) {
        success = 'Registration successful. Please check your email to confirm.';
      } else {
        const data = await res.json().catch(() => ({ error: 'Registration failed' }));
        error = data.error || 'Registration failed';
      }
    } catch (e: any) {
      error = e.message || 'Registration failed';
    }
  }
</script>

<div class="min-h-screen flex items-center justify-center p-4">
  <GlassCard padding="p-6" customClass="w-full max-w-md text-center space-y-4">
    <h1 class="text-2xl font-semibold mb-2">Register</h1>
    <form class="space-y-4" on:submit|preventDefault={submit}>
      <input class="glass-input w-full" type="text" bind:value={orgId} placeholder="Organization ID" required />
      <input class="glass-input w-full" type="email" bind:value={email} placeholder="Email" required />
      <input class="glass-input w-full" type="password" bind:value={password} placeholder="Password" required />
      {#if error}
        <p class="text-red-500 text-sm">{error}</p>
      {/if}
      {#if success}
        <p class="text-green-500 text-sm">{success}</p>
      {/if}
      <Button type="submit" variant="primary" customClass="w-full">Register</Button>
    </form>
    <p class="text-sm">Already have an account? <a href="/login" class="text-accent hover:underline">Login</a></p>
  </GlassCard>
</div>
