<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { apiFetch } from '$lib/utils/apiUtils';

  const dispatch = createEventDispatcher();
  let email = '';
  let password = '';
  let error = '';

  async function submit() {
    error = '';
    const res = await apiFetch('/api/login', {
      method: 'POST',
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
  <input class="input input-bordered w-full" type="email" bind:value={email} placeholder="Email" required />
  <input class="input input-bordered w-full" type="password" bind:value={password} placeholder="Password" required />
  <button class="btn btn-primary w-full" on:click|preventDefault={submit}>Login</button>
  {#if error}
    <p class="text-red-500 text-sm">{error}</p>
  {/if}
</form>
