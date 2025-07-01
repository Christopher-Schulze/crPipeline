<script lang="ts">
  import { goto } from '$app/navigation';
  import GlassCard from '$lib/components/GlassCard.svelte';
  import Button from '$lib/components/Button.svelte';
  import { apiFetch } from '$lib/utils/apiUtils';

  let email = '';
  let password = '';
  let error: string | null = null;

  async function submit() {
    error = null;
    try {
      const res = await apiFetch('/api/login', {
        method: 'POST',
        body: JSON.stringify({ email, password })
      });
      if (res.ok) {
        goto('/dashboard');
      } else {
        const data = await res.json().catch(() => ({ error: 'Login failed' }));
        error = data.error || 'Login failed';
      }
    } catch (e: any) {
      error = e.message || 'Login failed';
    }
  }
</script>

<div class="min-h-screen flex items-center justify-center p-4">
  <GlassCard padding="p-6" customClass="w-full max-w-md text-center space-y-4">
    <h1 class="text-2xl font-semibold mb-2">Login</h1>
    <form class="space-y-4" on:submit|preventDefault={submit}>
      <input class="glass-input w-full" type="email" bind:value={email} placeholder="Email" required />
      <input class="glass-input w-full" type="password" bind:value={password} placeholder="Password" required />
      {#if error}
        <p class="text-red-500 text-sm">{error}</p>
      {/if}
      <Button type="submit" variant="primary" customClass="w-full">Login</Button>
    </form>
    <p class="text-sm">Don't have an account? <a href="/register" class="text-accent hover:underline">Register</a></p>
  </GlassCard>
</div>
