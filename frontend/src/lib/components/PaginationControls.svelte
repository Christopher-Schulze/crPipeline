<!-- frontend/src/lib/components/PaginationControls.svelte -->
<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import Button from './Button.svelte';

  export let currentPage: number = 1;
  export let totalPages: number = 1;
  // export let itemsPerPage: number = 10; // Optional, for display
  // export let totalItems: number = 0;    // Optional, for display

  const dispatch = createEventDispatcher();

  function goToPage(page: number) {
    if (page >= 1 && page <= totalPages && page !== currentPage) {
      dispatch('pageChange', { page });
    }
  }

  // Determine page numbers to display (e.g., up to 5 page links)
  let pageNumbers: (number | '...')[] = [];
  $: {
    const PADDING = 2; // Number of pages around current, plus current = 2*PADDING + 1 links
    let start = Math.max(1, currentPage - PADDING);
    let end = Math.min(totalPages, currentPage + PADDING);

    // Adjust start/end if near boundaries to maintain roughly the same number of visible page links
    if (totalPages <= (2 * PADDING + 1)) { // If total pages is less than or equal to max visible links
        start = 1;
        end = totalPages;
    } else {
        if (currentPage - PADDING <= 1) { // If current page is near the beginning
            end = 1 + 2 * PADDING;
            start = 1;
        }
        if (currentPage + PADDING >= totalPages) { // If current page is near the end
            start = totalPages - 2 * PADDING;
            end = totalPages;
        }
    }

    const tempNumbers: (number | '...')[] = [];
    if (start > 1) {
      tempNumbers.push(1);
      if (start > 2) tempNumbers.push('...');
    }
    for (let i = start; i <= end; i++) {
      tempNumbers.push(i);
    }
    if (end < totalPages) {
      if (end < totalPages - 1) tempNumbers.push('...');
      tempNumbers.push(totalPages);
    }
    pageNumbers = tempNumbers;
  }
</script>

{#if totalPages > 1}
  <div class="flex items-center justify-between mt-4 text-sm text-gray-300">
    <Button
      variant="secondary"
      customClass="!px-3 !py-1.5 text-xs"
      disabled={currentPage <= 1}
      on:click={() => goToPage(currentPage - 1)}
    >
      Previous
    </Button>

    <div class="hidden sm:flex items-center space-x-1">
      {#each pageNumbers as pageNum, i (pageNum === '...' ? 'ellipsis-' + i : pageNum)}
        {#if typeof pageNum === 'number'}
          <Button
            variant={pageNum === currentPage ? 'primary' : 'ghost'}
            customClass="!px-3 !py-1.5 text-xs {pageNum === currentPage ? '' : '!text-gray-400 hover:!text-gray-200'}"
            on:click={() => goToPage(pageNum)}
          >
            {pageNum}
          </Button>
        {:else}
          <span class="px-3 py-1.5 text-xs text-gray-500">...</span>
        {/if}
      {/each}
    </div>
    <div class="sm:hidden text-xs"> <!-- Mobile page indicator -->
        Page {currentPage} of {totalPages}
    </div>

    <Button
      variant="secondary"
      customClass="!px-3 !py-1.5 text-xs"
      disabled={currentPage >= totalPages}
      on:click={() => goToPage(currentPage + 1)}
    >
      Next
    </Button>
  </div>
{/if}
