<script lang="ts">
import Button from './Button.svelte';
import DataTable from './DataTable.svelte';
import type { TableHeader } from './DataTable.svelte';
import PaginationControls from './PaginationControls.svelte';
import { onMount } from 'svelte';
import { apiFetch } from '$lib/utils/apiUtils';
import type { Document as APIDocument } from '$lib/types/api';

// Base Document interface matching backend model
type Document = APIDocument;

// AppDocument for internal use within this component after transformation
interface AppDocument {
  id: string;
  s3_key_name: string; // Stores the backend's 'filename' (S3 key)
  display_name: string; // The user-friendly name for display
  is_target: boolean;
  type: string; // 'Source' or 'Target'
  upload_date: string; // Original ISO string, useful for direct date operations if needed
  upload_date_formatted: string; // YYYY-MM-DD for display
  pages?: number;
  expires_at?: string | null;
}

export let orgId: string; // Consuming component must pass this

let internalDocs: AppDocument[] = [];
let isLoadingDocs = true;
let docsError: string | null = null;

let currentPage = 1;
let totalDocs = 0;
let docsPerPage = 10; // Default, can be updated by API response
let totalDocPages = 0;

let sortBy: string | null = 'upload_date';
let sortOrder: 'asc' | 'desc' = 'desc';

let filterType: 'all' | 'source' | 'target' = 'all';
let displayNameFilter: string = '';
let nameFilterDebounceTimer: number;

// Define table headers
const docTableHeaders: TableHeader[] = [
  { key: 'display_name', label: 'Name', headerClass: 'w-2/5', sortable: true, cellClass: 'group-hover:!text-accent-lighter transition-colors' },
  { key: 'type', label: 'Type', headerClass: 'w-1/5', sortable: false },
  { key: 'upload_date_formatted', label: 'Uploaded', headerClass: 'w-1/5', sortable: true },
  { key: 'actions', label: 'Actions', customClass: 'text-right', headerClass: 'w-1/5 text-right', sortable: false }
];

async function loadDocuments(pageToLoad = 1, newSortBy?: string | null, newSortOrder?: 'asc' | 'desc' | null) {
  if (!orgId) {
    isLoadingDocs = false;
    docsError = "Organization ID is not set. Cannot load documents.";
    internalDocs = [];
    totalDocs = 0;
    totalDocPages = 0;
    currentPage = 1;
    return;
  }
  isLoadingDocs = true;
  docsError = null;
  currentPage = pageToLoad; // Optimistically set current page

  // Use new sort parameters if provided for this call, otherwise use current state
  const currentSortBy = newSortBy !== undefined ? newSortBy : sortBy;
  const currentSortOrder = newSortOrder !== undefined ? newSortOrder : sortOrder;

  try {
    let apiUrl = `/api/documents/${orgId}?page=${currentPage}&limit=${docsPerPage}`;
    if (currentSortBy) {
      apiUrl += `&sort_by=${encodeURIComponent(currentSortBy)}`;
      if (currentSortOrder) {
        apiUrl += `&sort_order=${encodeURIComponent(currentSortOrder)}`;
      }
    }
    if (displayNameFilter.trim() !== '') {
      apiUrl += `&display_name_ilike=${encodeURIComponent(displayNameFilter.trim())}`;
    }
    if (filterType === 'target') {
      apiUrl += `&is_target=true`;
    } else if (filterType === 'source') {
      apiUrl += `&is_target=false`;
    }
    // If filterType is 'all', is_target param is omitted.

    const response = await fetch(apiUrl);
    if (!response.ok) {
      const errorText = await response.text(); // Or response.json().error if backend sends structured error
      throw new Error(`Failed to load documents: ${response.status} ${errorText}`);
    }
    const data = await response.json();

    internalDocs = data.items.map((doc: Document): AppDocument => ({
      id: doc.id,
      s3_key_name: doc.filename, // Backend's 'filename' is the S3 key
      display_name: doc.display_name, // New display_name from backend
      is_target: doc.is_target,
      type: doc.is_target ? 'Target' : 'Source',
      upload_date: doc.upload_date, // Keep original for potential non-display uses
      upload_date_formatted: new Date(doc.upload_date).toISOString().substring(0,10),
      pages: doc.pages,
      expires_at: doc.expires_at,
    }));

    currentPage = data.page;
    totalDocs = data.total_items;
    docsPerPage = data.per_page;
    totalDocPages = data.total_pages;
    // Update sortBy and sortOrder from backend response to stay in sync
    sortBy = data.sort_by || null; // Ensure null if backend sends empty or omits
    sortOrder = data.sort_order || 'asc'; // Default to 'asc' if backend omits (though it shouldn't)

  } catch (e: any) {
    docsError = e.message;
    internalDocs = [];
    totalDocs = 0;
    totalDocPages = 0;
    // currentPage = 1; // Keep current page or reset? Resetting might be confusing on error.
    // Ensure currentPage is not out of bounds after filtering/loading
    if (totalDocPages > 0 && currentPage > totalDocPages) {
        currentPage = totalDocPages; // Go to last valid page
        // Potentially re-load if page changed, but loadDocuments already did for pageToLoad
        // This logic might be better handled by ensuring pageToLoad itself is clamped before call
    }

  } finally {
    isLoadingDocs = false;
  }
}

onMount(() => {
  if (orgId) { // Ensure orgId is present onMount before initial load
    loadDocuments(1);
  }
});

// React to orgId changes
$: if (orgId && typeof orgId === 'string') {
    // This will also be caught by onMount if orgId is available initially.
    // If orgId can change reactively after mount, this reloads.
    loadDocuments(1);
}

// Filter and Sort Handlers
function onNameFilterInput() {
    clearTimeout(nameFilterDebounceTimer);
    nameFilterDebounceTimer = window.setTimeout(() => {
        loadDocuments(1); // Reset to page 1 and use current displayNameFilter state
    }, 500);
}

function setFilterType(newType: 'all' | 'source' | 'target') {
    if (filterType !== newType) {
        filterType = newType;
        loadDocuments(1); // Reset to page 1
    }
}

function handlePageChange(event: CustomEvent<{ page: number }>) {
  if (event.detail.page !== currentPage) {
    loadDocuments(event.detail.page); // Uses current sort/filter state
  }
}

function handleSortChange(event: CustomEvent<{ sortKey: string | null, sortDirection: 'asc' | 'desc' | null }>) {
    // Update sortBy and sortOrder state based on event from DataTable
    // These will then be used by loadDocuments
    sortBy = event.detail.sortKey;
    sortOrder = event.detail.sortDirection || (sortBy === 'upload_date' ? 'desc' : 'asc');
    loadDocuments(1); // Reset to page 1
}

async function downloadDocument(id: string) {
  try {
    const res = await fetch(`/api/download/${id}`, { credentials: 'include' });
    if (res.ok) {
      const { url } = await res.json();
      window.open(url, '_blank');
    } else {
      alert('Failed to get download link: ' + await res.text());
    }
  } catch (error) {
    console.error("Download error:", error);
    alert('Error getting download link. See console.');
  }
}

async function deleteDocument(id: string) {
  if (!confirm('Delete this document?')) return;
  try {
    const res = await apiFetch(`/api/documents/${id}`, { method: 'DELETE' });
    if (res.ok) {
      await loadDocuments(currentPage);
    } else {
      alert('Failed to delete document: ' + (await res.text()));
    }
  } catch (e: any) {
    alert(`Error deleting document: ${e.message}`);
  }
}
</script>

<div class="space-y-4">
  <div class="flex flex-col sm:flex-row items-center space-y-2 sm:space-y-0 sm:space-x-2 mb-4">
      <div class="flex-grow w-full sm:w-auto">
          <input
              type="text"
              bind:value={displayNameFilter}
              on:input={onNameFilterInput}
              placeholder="Search by document name..."
              class="glass-input w-full !text-sm"
              disabled={isLoadingDocs}
          />
      </div>
      <div class="flex space-x-1">
          <Button variant={filterType === 'all' ? 'primary' : 'secondary'} customClass="!px-3 !py-1.5 text-sm" on:click={() => setFilterType('all')} disabled={isLoadingDocs}>All</Button>
          <Button variant={filterType === 'source' ? 'primary' : 'secondary'} customClass="!px-3 !py-1.5 text-sm" on:click={() => setFilterType('source')} disabled={isLoadingDocs}>Source</Button>
          <Button variant={filterType === 'target' ? 'primary' : 'secondary'} customClass="!px-3 !py-1.5 text-sm" on:click={() => setFilterType('target')} disabled={isLoadingDocs}>Target</Button>
      </div>
  </div>

  <!-- DataTable is always rendered, its internal empty state will handle isLoadingDocs/docsError -->
  <DataTable
    headers={docTableHeaders}
    items={internalDocs}  <!-- Changed from filteredDocs -->
    keyField="id"
    tableSortable={true}
    serverSideSort={true}
    currentSortKey={sortBy}
    currentSortDirection={sortOrder}
    on:sortChange={handleSortChange}
    currentPage={currentPage}
    totalPages={totalDocPages}
    totalItems={totalDocs}
    itemsPerPage={docsPerPage}
    emptyStateMessage={isLoadingDocs ? "Loading documents..." : (docsError ? `Error: ${docsError}` : "No documents found. Try adjusting filters or upload new documents.")}
    emptyStateIconPath={isLoadingDocs || docsError ? null : "M19.5 14.25v-2.625a3.375 3.375 0 00-3.375-3.375h-1.5A1.125 1.125 0 0113.5 7.125v-1.5a3.375 3.375 0 00-3.375-3.375H8.25m0 12.75h7.5m-7.5 3H12M10.5 2.25H5.625c-.621 0-1.125.504-1.125 1.125v17.25c0 .621.504 1.125 1.125 1.125h12.75c.621 0 1.125-.504 1.125-1.125V11.25a9 9 0 00-9-9z"}
    tableContainerClass="overflow-hidden shadow-lg rounded-xl border border-neutral-700/50 bg-neutral-800/40 backdrop-blur-md"
    tableClass="min-w-full divide-y divide-neutral-700/30"
    thClass="px-4 py-2.5 text-left text-xs font-medium text-gray-400 uppercase tracking-wider bg-neutral-700/20"
    trClass="hover:bg-neutral-700/40 transition-colors duration-150 group"
    tdClass="px-4 py-3 whitespace-nowrap text-sm text-gray-300"
  >
    <!-- Custom slot for 'display_name' (formerly 'filename') to apply consistent styling and title -->
    <span slot="cell-display_name" let:item title={item.display_name} class="font-medium !text-gray-100 group-hover:!text-accent-lighter transition-colors truncate block">
      {item.display_name}
    </span>

    <!-- Custom slot for 'type' column to display badge -->
    <div slot="cell-type" let:item class="flex items-center">
      <span
        class="px-2 py-0.5 inline-flex text-xs leading-5 font-semibold rounded-full whitespace-nowrap
               {item.is_target ? 'bg-blue-100 text-blue-700' : 'bg-green-100 text-green-700'}"
      >
        {item.type}
      </span>
    </div>

    <!-- Custom slot for 'actions' column -->
    <div slot="cell-actions" let:item class="flex justify-end items-center space-x-2">
      {#if item.is_target && item.expires_at}
        <span class="text-xs text-gray-500 self-center">
          Expires: {new Date(item.expires_at).toLocaleDateString()}
        </span>
      {/if}
      <Button variant="ghost" customClass="!px-2 !py-1 text-xs" on:click={() => downloadDocument(item.id)}>
        Download
      </Button>
      <Button variant="ghost" customClass="!px-2 !py-1 text-xs text-red-400 hover:text-red-300" on:click={() => deleteDocument(item.id)}>
        Delete
      </Button>
    </div>
    <div slot="paginationControls" let:currentPageProps let:totalPagesProps> <!-- Renamed slot props to avoid conflict -->
      <PaginationControls
        currentPage={currentPageProps}
        totalPages={totalPagesProps}
        on:pageChange={handlePageChange}
      />
    </div>
  </DataTable>
</div>
