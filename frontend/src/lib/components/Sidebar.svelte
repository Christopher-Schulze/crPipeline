<script lang="ts">
  // createEventDispatcher and navigate function are no longer needed for basic SvelteKit routing via href
  // import { createEventDispatcher } from 'svelte';

  export interface NavItem {
    id: string; // Unique key for #each
    path: string;
    label: string;
    icon?: string; // SVG path data for a 24x24 viewBox
  }

  export let navItems: NavItem[] = [];
  export let currentPath: string = '/'; // Default current path

  // const dispatch = createEventDispatcher(); // No longer needed
  // function navigate(path: string) { // No longer needed
  //   dispatch('navigate', { path });
  // }

  // Use native navigation instead of SvelteKit's goto
  const goto = (path: string) => { window.location.href = path; };
  import { apiFetch } from '$lib/utils/apiUtils';
  import { sessionStore } from '$lib/utils/sessionStore';

  async function logout() {
    try {
      await apiFetch('/api/logout', { method: 'POST' });
    } catch (e) {
      // ignore network errors
    }
    sessionStore.clear();
    goto('/login');
  }
</script>

<aside class="w-60 h-screen bg-neutral-800/70 backdrop-blur-xl border-r border-neutral-700/50 p-3 space-y-1 flex flex-col shadow-2xl dark:bg-neutral-800/80 dark:border-neutral-700"> <!-- Added dark mode consistency -->
  <div class="text-center py-3 mb-3 border-b border-neutral-700/50">
    <h2 class="text-2xl font-bold text-gray-100 tracking-tight">
      cr<span class="text-accent">Pipeline</span>
    </h2>
  </div>
  <nav class="flex-grow space-y-1">
    <ul>
      {#each navItems as item (item.id)}
    <li>
      <a
        href={item.path}
        class="relative flex items-center space-x-3 px-4 py-2.5 rounded-lg transition-all duration-150 ease-in-out group
               {currentPath === item.path
                 ? 'bg-accent/15 text-accent shadow-inner ring-1 ring-accent/20 scale-[1.01]' // Active state base styling
                 : 'text-gray-400 hover:bg-neutral-700/60 hover:text-gray-100 active:bg-neutral-600/50'}"
        aria-current={currentPath === item.path ? 'page' : undefined}
      >
        {#if currentPath === item.path}
          <!-- Active indicator bar: shorter, vertically centered -->
          <div class="absolute left-0 top-1/4 bottom-1/4 w-[3px] bg-accent rounded-r-sm"></div>
        {/if}

        {#if item.icon}
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5"
               class="w-[18px] h-[18px] flex-shrink-0 transition-colors duration-150 ease-in-out
                      {currentPath === item.path
                        ? 'stroke-accent' // Active icon stroke color
                        : 'stroke-gray-500 group-hover:stroke-gray-300'}">
            <path stroke-linecap="round" stroke-linejoin="round" d={item.icon} />
          </svg>
        {:else}
          <span class="w-[18px] h-[18px] flex-shrink-0"></span> <!-- Placeholder for alignment -->
        {/if}
        <span
          class="transition-colors duration-150 ease-in-out text-sm truncate
                 {currentPath === item.path
                   ? 'font-medium text-accent' // Active text style
                   : 'font-normal text-gray-300 group-hover:text-gray-100'}"
          title={item.label}
        >
          {item.label}
        </span>
      </a>
    </li>
      {/each}
    </ul>
  </nav>
  <div class="mt-auto pt-3 border-t border-neutral-700/50">
    <!-- Placeholder for user profile / logout -->
    <div class="p-2 text-center space-y-2">
      <span class="block text-xs font-light text-gray-500">© crPipeline</span>
      {#if $sessionStore.loggedIn}
        <button
          type="button"
          class="text-xs text-accent hover:underline"
          on:click={logout}
        >
          Logout
        </button>
      {/if}
    </div>
  </div>
</aside>

<style lang="postcss">
  /* Minor adjustments if needed, Tailwind should cover most */
  aside {
    scrollbar-width: thin; /* For Firefox */
    scrollbar-color: theme('colors.neutral.600') theme('colors.neutral.800'); /* For Firefox */
  }
  aside::-webkit-scrollbar {
    width: 6px;
  }
  aside::-webkit-scrollbar-track {
    background: theme('colors.neutral.800');
  }
  aside::-webkit-scrollbar-thumb {
    background-color: theme('colors.neutral.600');
    border-radius: 3px;
  }
  aside::-webkit-scrollbar-thumb:hover {
    background-color: theme('colors.neutral.500');
  }
</style>
