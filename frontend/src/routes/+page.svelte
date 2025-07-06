<script lang="ts">
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';
  import { sessionStore } from '$lib/stores/session';

  // This page might eventually show a landing page or redirect based on auth state.
  // Auth state (loggedIn) is available from the root +layout.svelte's data prop,
  // which would be populated by a +layout.ts load function.

  // For this initial setup, if the user is logged in (determined by layout data),
  // we'll redirect to their dashboard. Otherwise, this page might imply a login is needed.
  // A proper /login route will be created later.

  // Accessing layout data (passed down from root +layout.svelte via SvelteKit)
  // $: loggedIn = $sessionStore.loggedIn || false; // Example of accessing session store

  onMount(() => {
    // Client-side redirect based on a simple cookie check (placeholder for proper auth handling via load functions)
    // A more SvelteKit-idiomatic approach would be to handle this in a +layout.ts or +page.ts load function.
    const isLoggedInFromLayout = $sessionStore.loggedIn;

    if (isLoggedInFromLayout) {
      goto('/dashboard', { replaceState: true });
    } else {
      // If not logged in, and a /login page existed, we might redirect there.
      // For now, this page will just show a prompt to login, assuming LoginForm is not here.
      // Or, if a global LoginForm component was part of +layout.svelte for unauthenticated users:
      // The layout would handle showing login form, and this page might not be reached often by unauth users.
      // If there's no global login form in layout, this page becomes the de-facto pre-login landing.
      // Let's assume a /login route will be created. For now, show message.
       if (window.location.pathname !== '/login' && window.location.pathname !== '/') {
         // If trying to access other protected pages when not logged in, and no proper redirect from load functions yet.
         // goto('/login', { replaceState: true }); // Placeholder for future /login route
       }
    }
  });
</script>

<div class="min-h-screen flex flex-col items-center justify-center p-4 sm:p-8 text-center">
  <div class="space-y-6 p-6 sm:p-8 bg-white/10 dark:bg-neutral-800/50 backdrop-blur-xl rounded-2xl shadow-2xl max-w-lg w-full border border-white/20 dark:border-neutral-700/50">
    <h1 class="text-4xl font-bold text-gray-800 dark:text-gray-100">
      Welcome to cr<span class="text-accent">Pipeline</span>
    </h1>

    {#if !$sessionStore.loggedIn}
      <p class="text-gray-600 dark:text-gray-300">
        Advanced document processing and analysis.
      </p>
      <p class="text-md text-gray-700 dark:text-gray-200">
        Please <a href="/login" class="text-accent hover:underline font-medium">login</a> to access your dashboard and features.
      </p>
      <p class="text-xs text-gray-500 dark:text-gray-400 mt-4">
        (This is the root page. If you have an account, logging in will redirect you to your dashboard.
        A dedicated login page/form component will be integrated later.)
      </p>
    {:else}
       <p class="text-md text-gray-700 dark:text-gray-200">
        Loading your experience...
      </p>
      <!-- This content is usually not seen due to client-side redirect in onMount if loggedIn -->
    {/if}
  </div>
</div>

<style lang="postcss">
  /* Page specific styles if any */
</style>
