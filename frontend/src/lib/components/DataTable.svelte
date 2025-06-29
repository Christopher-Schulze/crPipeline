<script context="module" lang="ts">
  /** Table header definition exported for external consumption */
  export interface TableHeader {
    key: string;
    label: string;
    sortable?: boolean;
    resizable?: boolean; // allow disabling resize per column
    width?: number;      // initial width in pixels
    minWidth?: number;   // minimum width in pixels
    customClass?: string;
    headerClass?: string;
    cellClass?: string | ((value: any, item: any, rowIndex: number) => string);
  }
</script>

<script lang="ts">
  import { createEventDispatcher, onMount, onDestroy } from 'svelte';
  import { writable } from 'svelte/store';

  /** Slot typings for DataTable */
  export interface $$Slots {
    default: {};
    ['cell-id']: { item: any; value: any; rowIndex: number };
    ['cell-document_name']: { item: any; value: any; rowIndex: number };
    ['cell-pipeline_name']: { item: any; value: any; rowIndex: number };
    ['cell-status']: { item: any; value: any; rowIndex: number };
    ['cell-created_at_formatted']: { item: any; value: any; rowIndex: number };
    ['cell-actions']: { item: any; value: any; rowIndex: number };
    ['cell-display_name']: { item: any; value: any; rowIndex: number };
    ['cell-type']: { item: any; value: any; rowIndex: number };
    ['cell-upload_date_formatted']: { item: any; value: any; rowIndex: number };
    ['cell-name']: { item: any; value: any; rowIndex: number };
    ['cell-stage_count']: { item: any; value: any; rowIndex: number };
    ['cell-created_at']: { item: any; value: any; rowIndex: number };
    ['cell-updated_at']: { item: any; value: any; rowIndex: number };
    paginationControls?: {
      currentPage: number;
      totalPages: number;
      itemsPerPage: number | null;
      totalItems: number | null;
    };
  }

  export let headers: TableHeader[] = [];
  export let items: any[] = [];
  export let tableSortable: boolean = true; // New: Global toggle for table sortability

  // Styling props with defaults for a glassy table appearance
  export let tableContainerClass: string =
    "overflow-auto max-h-[75vh] shadow-md rounded-lg border border-neutral-700/50 bg-neutral-800/40 backdrop-blur-md";
  export let tableClass: string = "min-w-full divide-y divide-neutral-700/30";
  export let thClass: string =
    "px-4 py-2.5 text-left text-xs font-light text-gray-200 dark:text-gray-300 uppercase tracking-wider "+
    "bg-neutral-700/85 dark:bg-neutral-800/90 backdrop-blur-sm shadow-sm";
  export let trClass: string = "hover:bg-neutral-700/40 transition-colors duration-150 group";
  export let tdClass: string = "px-4 py-3 whitespace-nowrap text-sm text-gray-300";
  // export let noDataMessage: string = "No data available."; // Removed old prop
  export let emptyStateMessage: string = "No data available.";
  export let emptyStateIconPath: string | null = null; // SVG path data

  // Key function for #each block on items for Svelte reactivity
  export let keyField: string | null = 'id';

  // Props for pagination (passed through from parent)
  export let currentPage: number = 1;
  export let totalPages: number = 0; // Default to 0 if no items / not paginated
  export let totalItems: number | null = null; // Optional for display
  export let itemsPerPage: number | null = null; // Optional for display

  // New props for server-side sorting state
  export let serverSideSort: boolean = false;
  export let currentSortKey: string | null = null;
  export let currentSortDirection: 'asc' | 'desc' | null = null;

  // State for client-side sorting (used if serverSideSort is false)
  let sortKey: string | null = null;
  let sortDirection: 'asc' | 'desc' = 'asc';

  const dispatch = createEventDispatcher();

  // State for column widths (maps header key to pixel width)
  let columnWidths = writable<Record<string, number>>({});

  // State for active resizing
  let currentlyResizingKey: string | null = null;
  let initialMouseX: number = 0;
  let initialWidth: number = 0;

  // Initialize widths on mount or when headers change
  function initializeWidths() {
      const newWidths: Record<string, number> = {};
      headers.forEach(header => {
          if (header.width) {
              newWidths[header.key] = Math.max(header.width, header.minWidth || 50);
          }
      });
      columnWidths.set(newWidths);
  }

  onMount(() => {
      initializeWidths();
  });

  // Re-initialize if headers prop changes
  // Consider more sophisticated merge if user-resized widths should be preserved across header changes
  $: if (headers) {
      initializeWidths();
  }

  function handleMouseDown(event: MouseEvent, headerKey: string) {
      event.preventDefault();
      event.stopPropagation();

      const headerElement = (event.currentTarget as HTMLElement)?.closest('th');
      if (!headerElement) return;

      currentlyResizingKey = headerKey;
      initialMouseX = event.clientX;
      initialWidth = headerElement.offsetWidth;

      window.addEventListener('mousemove', handleMouseMove);
      window.addEventListener('mouseup', handleMouseUp);
  }

  function handleMouseMove(event: MouseEvent) {
      if (!currentlyResizingKey) return;
      event.preventDefault();

      const currentHeaderConf = headers.find(h => h.key === currentlyResizingKey);
      const minW = currentHeaderConf?.minWidth || 50;

      const deltaX = event.clientX - initialMouseX;
      let newWidth = initialWidth + deltaX;
      newWidth = Math.max(newWidth, minW);

      columnWidths.update(widths => {
          widths[currentlyResizingKey!] = newWidth;
          return widths;
      });
  }

  function handleMouseUp(event: MouseEvent) {
      if (!currentlyResizingKey) return;
      event.preventDefault();
      currentlyResizingKey = null;
      window.removeEventListener('mousemove', handleMouseMove);
      window.removeEventListener('mouseup', handleMouseUp);
  }

  onDestroy(() => {
      window.removeEventListener('mousemove', handleMouseMove);
      window.removeEventListener('mouseup', handleMouseUp);
  });

  // For applying styles
  function getColumnStyle(headerKey: string): string {
      const width = $columnWidths[headerKey];
      // For table-layout: fixed, 'width' is enough. Using min/max for robustness if layout changes.
      return width ? `width: ${width}px; min-width: ${width}px; max-width: ${width}px;` : '';
  }

  // Click handler for sortable headers
  function handleSort(headerKey: string, isColumnSortable?: boolean) {
    if (!tableSortable || isColumnSortable === false) return;

    if (serverSideSort) {
      let newDirection: 'asc' | 'desc' = 'asc';
      if (currentSortKey === headerKey) {
        newDirection = currentSortDirection === 'asc' ? 'desc' : 'asc';
      } else {
        newDirection = 'asc'; // Default for new column
      }
      dispatch('sortChange', { sortKey: headerKey, sortDirection: newDirection });
    } else {
      // Existing client-side sorting logic:
      if (sortKey === headerKey) {
        sortDirection = sortDirection === 'asc' ? 'desc' : 'asc';
      } else {
        sortKey = headerKey;
        sortDirection = 'asc';
      }
    }
  }

  // Computed property for sorted items
  $: sortedItems = (() => {
    if (serverSideSort || !sortKey || !tableSortable) { // If server-side sort, items are already sorted (or no client sort needed)
      return [...items];
    }
    // Client-side sorting logic:
    const itemsCopy = [...items];

    return itemsCopy.sort((a, b) => {
      const valA = a[sortKey!];
      const valB = b[sortKey!];

      let comparison = 0;
      if (valA === null || valA === undefined) comparison = -1;
      else if (valB === null || valB === undefined) comparison = 1;
      else if (typeof valA === 'number' && typeof valB === 'number') {
        comparison = valA - valB;
      } else if (typeof valA === 'string' && typeof valB === 'string') {
        comparison = valA.localeCompare(valB);
      } else {
        comparison = String(valA).localeCompare(String(valB));
      }
      return sortDirection === 'asc' ? comparison : -comparison;
    });
  })();

  function resolveCellClass(header: TableHeader, item: any, value: any, rowIndex: number): string {
    if (typeof header.cellClass === 'function') {
      return header.cellClass(value, item, rowIndex);
    } else if (typeof header.cellClass === 'string') {
      return header.cellClass;
    }
    return '';
  }

</script>

<div class="{tableContainerClass}">
  <table class="{tableClass} w-full" style="table-layout: fixed;">
    <thead class="sticky top-0 z-10 bg-neutral-700 dark:bg-neutral-800">
      <tr>
        {#each headers as header (header.key)}
          <th
            scope="col"
            class="{thClass} {header.headerClass || ''} {header.customClass || ''} relative {(tableSortable && header.sortable !== false) ? 'cursor-pointer hover:bg-neutral-600/30' : ''}"
            style={getColumnStyle(header.key)}
            title={(tableSortable && header.sortable !== false) ? `Sort by ${header.label}` : header.label}
          >
            <div class="flex items-center justify-between">
                <div
                    class="flex items-center space-x-1 flex-grow overflow-hidden pr-2 {(tableSortable && header.sortable !== false) ? 'cursor-pointer hover:opacity-80' : ''}"
                    on:click={() => handleSort(header.key, header.sortable)}
                >
                    <span class="truncate" title={header.label}>{header.label}</span>
                    {#if tableSortable && header.sortable !== false && (serverSideSort ? currentSortKey : sortKey) === header.key}
                        <span class="opacity-70">
                        {#if (serverSideSort ? currentSortDirection : sortDirection) === 'asc'}
                            <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="w-3.5 h-3.5">
                            <path fill-rule="evenodd" d="M10 3a.75.75 0 01.75.75v10.5a.75.75 0 01-1.5 0V3.75A.75.75 0 0110 3z" clip-rule="evenodd" />
                            <path fill-rule="evenodd" d="M5.22 6.22a.75.75 0 011.06 0L10 9.94l3.72-3.72a.75.75 0 111.06 1.06l-4.25 4.25a.75.75 0 01-1.06 0L5.22 7.28a.75.75 0 010-1.06z" clip-rule="evenodd" />
                            </svg>
                        {:else} <!-- implies 'desc' -->
                            <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="w-3.5 h-3.5">
                            <path fill-rule="evenodd" d="M10 17a.75.75 0 01-.75-.75V5.75a.75.75 0 011.5 0v10.5A.75.75 0 0110 17z" clip-rule="evenodd" />
                            <path fill-rule="evenodd" d="M14.78 13.78a.75.75 0 01-1.06 0L10 10.06l-3.72 3.72a.75.75 0 11-1.06-1.06l4.25-4.25a.75.75 0 011.06 0l4.25 4.25a.75.75 0 010 1.06z" clip-rule="evenodd" />
                            </svg>
                        {/if}
                        </span>
                    {/if}
                </div>
                {#if header.resizable !== false}
                <div
                    class="absolute right-0 top-0 bottom-0 w-1.5 cursor-col-resize hover:bg-accent/30 active:bg-accent/50 select-none z-10"
                    on:mousedown|stopPropagation={(e) => handleMouseDown(e, header.key)}
                    role="separator"
                    aria-label={`Resize column ${header.label}`}
                ></div>
                {/if}
            </div>
          </th>
        {/each}
      </tr>
    </thead>
    <tbody class="divide-y divide-neutral-700/30 bg-neutral-800/20">
      {#if sortedItems.length === 0}
        <tr>
          <td colspan={headers.length} class="px-6 py-12 text-center">
            {#if $$slots.emptyState}
              <slot name="emptyState"></slot>
            {:else}
              <div class="flex flex-col items-center justify-center space-y-3 text-gray-400">
                {#if emptyStateIconPath}
                  <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-12 h-12 opacity-40">
                    <path stroke-linecap="round" stroke-linejoin="round" d={emptyStateIconPath} />
                  </svg>
                {/if}
                <span class="font-light">{emptyStateMessage}</span>
              </div>
            {/if}
          </td>
        </tr>
      {:else}
        {#each sortedItems as item, rowIndex (keyField && item[keyField] !== undefined ? item[keyField] : rowIndex)}
          <tr class="{trClass}">
            {#each headers as header (header.key)}
              <td class="{tdClass} {resolveCellClass(header, item, item[header.key], rowIndex)}" style={getColumnStyle(header.key)}>
                {#if header.key === 'id' && $$slots['cell-id']}
                  <slot name="cell-id" {item} value={item[header.key]} {rowIndex}></slot>
                {:else if header.key === 'document_name' && $$slots['cell-document_name']}
                  <slot name="cell-document_name" {item} value={item[header.key]} {rowIndex}></slot>
                {:else if header.key === 'pipeline_name' && $$slots['cell-pipeline_name']}
                  <slot name="cell-pipeline_name" {item} value={item[header.key]} {rowIndex}></slot>
                {:else if header.key === 'status' && $$slots['cell-status']}
                  <slot name="cell-status" {item} value={item[header.key]} {rowIndex}></slot>
                {:else if header.key === 'created_at_formatted' && $$slots['cell-created_at_formatted']}
                  <slot name="cell-created_at_formatted" {item} value={item[header.key]} {rowIndex}></slot>
                {:else if header.key === 'actions' && $$slots['cell-actions']}
                  <slot name="cell-actions" {item} value={item[header.key]} {rowIndex}></slot>
                {:else if header.key === 'display_name' && $$slots['cell-display_name']}
                  <slot name="cell-display_name" {item} value={item[header.key]} {rowIndex}></slot>
                {:else if header.key === 'type' && $$slots['cell-type']}
                  <slot name="cell-type" {item} value={item[header.key]} {rowIndex}></slot>
                {:else if header.key === 'upload_date_formatted' && $$slots['cell-upload_date_formatted']}
                  <slot name="cell-upload_date_formatted" {item} value={item[header.key]} {rowIndex}></slot>
                {:else if header.key === 'name' && $$slots['cell-name']}
                  <slot name="cell-name" {item} value={item[header.key]} {rowIndex}></slot>
                {:else if header.key === 'stage_count' && $$slots['cell-stage_count']}
                  <slot name="cell-stage_count" {item} value={item[header.key]} {rowIndex}></slot>
                {:else if header.key === 'created_at' && $$slots['cell-created_at']}
                  <slot name="cell-created_at" {item} value={item[header.key]} {rowIndex}></slot>
                {:else if header.key === 'updated_at' && $$slots['cell-updated_at']}
                  <slot name="cell-updated_at" {item} value={item[header.key]} {rowIndex}></slot>
                {:else}
                  {item[header.key] === null || item[header.key] === undefined ? '' : item[header.key]}
                {/if}
              </td>
            {/each}
          </tr>
        {/each}
      {/if}
    </tbody>
  </table>
  <!-- Slot for pagination controls -->
  {#if totalPages > 0 && $$slots.paginationControls}
    <div class="mt-0 py-2 border-t border-neutral-700/50"> <!-- This border might need adjustment based on container bg -->
      <slot name="paginationControls" {currentPage} {totalPages} {itemsPerPage} {totalItems}></slot>
    </div>
  {/if}
</div>

<style lang="postcss">
  /* Add any specific global styles for table if needed, but prefer Tailwind utilities */
  /* Ensure the backdrop-blur works well; sometimes specific z-indexing might be needed in complex layouts */
  /* The overflow-x-auto on tableContainerClass is important for responsive tables */
</style>
