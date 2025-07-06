<script lang="ts">
  import { onMount } from 'svelte';
  import DataTable, { type TableHeader } from './DataTable.svelte';
  import PaginationControls from './PaginationControls.svelte';
  import { apiFetch } from '$lib/utils/apiUtils';

  interface AuditLog {
    id: string;
    org_id: string;
    user_id: string;
    action: string;
    created_at: string;
  }

  export let orgId: string;

  let logs: AuditLog[] = [];
  let currentPage = 1;
  let totalPages = 0;
  let totalItems = 0;
  let perPage = 10;
  let isLoading = false;
  let error: string | null = null;

  const headers: TableHeader[] = [
    { key: 'created_at', label: 'Timestamp', sortable: false },
    { key: 'user_id', label: 'User ID', sortable: false },
    { key: 'action', label: 'Action', sortable: false },
  ];

  async function load(pageToLoad = 1) {
    if (!orgId) return;
    isLoading = true;
    error = null;
    try {
      const res = await apiFetch(`/api/audit/${orgId}?page=${pageToLoad}&limit=${perPage}`);
      const data = await res.json();
      logs = data.items;
      currentPage = data.page;
      totalPages = data.total_pages;
      totalItems = data.total_items;
      perPage = data.per_page;
    } catch (e: any) {
      error = e.message;
      logs = [];
      totalPages = 0;
      totalItems = 0;
    } finally {
      isLoading = false;
    }
  }

  onMount(() => load(1));

  function handlePageChange(e: CustomEvent<{ page: number }>) {
    load(e.detail.page);
  }
</script>

<DataTable
  {headers}
  items={logs}
  keyField="id"
  currentPage={currentPage}
  totalPages={totalPages}
  totalItems={totalItems}
  itemsPerPage={perPage}
  emptyStateMessage={isLoading ? 'Loading logs...' : (error ? `Error: ${error}` : 'No logs found.')}
  tableContainerClass="overflow-hidden shadow-lg rounded-xl border border-neutral-700/50 bg-neutral-800/40 backdrop-blur-md"
  tableClass="min-w-full divide-y divide-neutral-700/30"
  thClass="px-4 py-2.5 text-left text-xs font-medium text-gray-400 uppercase tracking-wider bg-neutral-700/20"
  trClass="hover:bg-neutral-700/40 transition-colors duration-150"
  tdClass="px-4 py-3 whitespace-nowrap text-sm text-gray-300"
>
  <div slot="paginationControls" let:currentPage let:totalPages>
    <PaginationControls {currentPage} {totalPages} on:pageChange={handlePageChange}/>
  </div>
</DataTable>
