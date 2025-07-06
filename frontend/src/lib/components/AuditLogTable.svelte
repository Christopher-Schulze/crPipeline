<script lang="ts">
  import { onMount } from 'svelte';
  import DataTable, { type TableHeader } from './DataTable.svelte';
  import PaginationControls from './PaginationControls.svelte';
  import { apiFetch } from '$lib/utils/apiUtils';

  export let orgId: string;

  interface AuditLog {
    id: string;
    org_id: string;
    user_id: string;
    action: string;
    created_at: string;

    created_at_formatted?: string;
    user_id_short?: string;
  }

  let logs: AuditLog[] = [];
  let isLoading = true;
  let error: string | null = null;

  let currentPage = 1;
  let totalPages = 0;
  let perPage = 20;
  let totalItems = 0;

  const headers: TableHeader[] = [
    { key: 'created_at_formatted', label: 'Timestamp', sortable: false },
    { key: 'user_id_short', label: 'User', sortable: false },
    { key: 'action', label: 'Action', sortable: false }
  ];

  async function loadLogs(page = 1) {
    if (!orgId) return;
    isLoading = true;
    currentPage = page;
    try {
      const res = await apiFetch(`/api/audit/${orgId}?page=${page}&limit=${perPage}`);
      const data = await res.json();
      logs = data.items.map((l: AuditLog) => ({
        ...l,
        created_at_formatted: new Date(l.created_at).toLocaleString(),
        user_id_short: l.user_id.substring(0, 8) + '...'
      }));
      totalPages = data.total_pages;
      perPage = data.per_page;
      totalItems = data.total_items;
      error = null;
    } catch (e: any) {
      error = e.message;
      logs = [];
      totalPages = 0;
      totalItems = 0;
    } finally {
      isLoading = false;
    }
  }

  function handlePageChange(e: CustomEvent<{ page: number }>) {
    if (e.detail.page !== currentPage) loadLogs(e.detail.page);
  }

  onMount(() => {
    loadLogs();
  });
</script>

<DataTable
  {headers}
  items={logs}
  keyField="id"
  tableSortable={false}
  currentPage={currentPage}
  totalPages={totalPages}
  totalItems={totalItems}
  itemsPerPage={perPage}
  emptyStateMessage={isLoading ? 'Loading logs...' : (error ? `Error: ${error}` : 'No logs found.')}
  tableContainerClass="bg-neutral-800/40 backdrop-blur-sm shadow-lg rounded-xl border border-neutral-700/50 overflow-hidden"
  tableClass="min-w-full divide-y divide-neutral-700/30"
>
  <span slot="cell-user_id_short" let:item title={item.user_id}>{item.user_id_short}</span>
  <span slot="cell-created_at_formatted" let:item class="text-xs text-gray-400">{item.created_at_formatted}</span>
  <div slot="paginationControls" let:currentPageProps let:totalPagesProps>
    <PaginationControls
      currentPage={currentPageProps}
      totalPages={totalPagesProps}
      on:pageChange={handlePageChange}
    />
  </div>
</DataTable>
