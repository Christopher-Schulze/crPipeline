<script lang="ts">
  import { createEventDispatcher } from 'svelte';

  const dispatch = createEventDispatcher();
  let email = '';
  let password = '';
  let error = '';

  async function submit() {
    error = '';
    const res = await fetch('/api/login', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ email, password })
    });
    if (res.ok) {
      dispatch('loggedin');
    } else {
      error = 'Login failed';
    }
  }
</script>

<form class="space-y-4" on:submit|preventDefault={submit}>
  <input class="glass-input w-full" type="email" bind:value={email} placeholder="Email" required />
  <input class="glass-input w-full" type="password" bind:value={password} placeholder="Password" required />
  <button class="btn-primary w-full" on:click|preventDefault={submit}>Login</button>
  {#if error}
    <p class="text-red-500 text-sm">{error}</p>
  {/if}
</form>
