<script lang="ts">
  import { onDestroy, onMount, getContext } from 'svelte';
  // GlassCard is not used directly per row anymore, DataTable handles overall container.
  // import GlassCard from './GlassCard.svelte';
  import Button from './Button.svelte';
  import DataTable, { type TableHeader } from './DataTable.svelte'; // Import DataTable and TableHeader
  import PaginationControls from './PaginationControls.svelte';
  import { apiFetch } from '$lib/utils/apiUtils';
  import { createReconnectingEventSource, type ReconnectingEventSource } from '$lib/utils/eventSourceUtils';

  // Define a more complete Job interface based on AnalysisJob model + potential names
  interface Job {
    id: string; // UUID
    org_id: string; // UUID
    document_id: string; // UUID
    pipeline_id: string; // UUID
    status: string;
    created_at: string; // ISO date string
    document_name: string;
    pipeline_name: string;
    // Any other fields from AnalysisJob that might be useful
  }
  export let orgId: string;
  let jobs: Job[] = [];
  let isLoadingJobs = true;
  let jobsError: string | null = null;
  let currentPage = 1;
  let totalJobs = 0;
  let jobsPerPage = 10;
  let totalJobPages = 0;

  const viewJobDetails = getContext<(jobId: string) => void>('viewJobDetails');

  // Helper for status colors
  function getStatusColorClass(status: string): string {
    if (status === 'completed' || status === 'success') return 'bg-green-500/20 text-green-300'; // Adjusted for darker table context
    if (status === 'failed' || status === 'error') return 'bg-red-500/20 text-red-300';
    if (status === 'in_progress' || status === 'running') return 'bg-blue-500/20 text-blue-300';
    return 'bg-gray-500/20 text-gray-300';
  }

  const jobTableHeaders: TableHeader[] = [
    { key: 'id', label: 'Job ID', cellClass: 'font-mono !text-xs !text-gray-400', sortable: false },
    { key: 'document_name', label: 'Document', cellClass: '!text-gray-300 group-hover:!text-accent-lighter transition-colors', sortable: true },
    { key: 'pipeline_name', label: 'Pipeline', cellClass: '!text-gray-300 group-hover:!text-accent-lighter transition-colors', sortable: true },
    { key: 'status', label: 'Status', sortable: true },
    { key: 'created_at_formatted', label: 'Created', sortable: true },
    { key: 'actions', label: 'Actions', headerClass: 'text-right', cellClass: 'text-right', sortable: false }
  ];

  $: tableJobs = jobs.map(job => ({
    ...job,
    // Format for sortability and readability: YYYY-MM-DD HH:MM:SS
    created_at_formatted: new Date(job.created_at).toISOString().replace('T', ' ').substring(0, 19),
    document_name: job.document_name,
    pipeline_name: job.pipeline_name,
  })).sort((a, b) => new Date(b.created_at).getTime() - new Date(a.created_at).getTime()); // Initial sort

  let sources: ReconnectingEventSource[] = [];

  async function loadJobs(pageToLoad = 1) {
    if (!orgId) {
      jobs = [];
      totalJobs = 0;
      totalJobPages = 0;
      isLoadingJobs = false;
      return;
    }
    isLoadingJobs = true;
    jobsError = null;
    currentPage = pageToLoad;
    try {
      const res = await apiFetch(`/api/jobs/${orgId}?page=${currentPage}&per_page=${jobsPerPage}`);
      const data = await res.json();
      jobs = data.items;
      currentPage = data.page;
      totalJobs = data.total_items;
      jobsPerPage = data.per_page;
      totalJobPages = data.total_pages;
    } catch (e: any) {
      jobsError = e.message;
      jobs = [];
      totalJobs = 0;
      totalJobPages = 0;
    } finally {
      isLoadingJobs = false;
    }
  }

  function handlePageChange(event: CustomEvent<{ page: number }>) {
    if (event.detail.page !== currentPage) {
      loadJobs(event.detail.page);
    }
  }

  $: if (jobs.length) {
    sources.forEach((s) => s.close());
    sources = jobs.map((job) => {
      let source: ReconnectingEventSource;
      source = createReconnectingEventSource(
        `/api/jobs/${job.id}/events`,
        (e) => {
          job.status = e.data;
          if (e.data === 'completed' || e.data === 'failed') {
            source.close();
          }
        }
      );
      return source;
    });
  }

  onMount(() => {
    if (orgId) {
      loadJobs(1);
    }
  });

  $: if (orgId) {
    loadJobs(1);
  }

  onDestroy(() => {
    sources.forEach((s) => s.close());
  });
</script>

<DataTable headers={jobTableHeaders} items={tableJobs} keyField="id"
    tableContainerClass="overflow-hidden shadow-lg rounded-xl border border-neutral-700/50 bg-neutral-800/40 backdrop-blur-md"
    tableClass="min-w-full divide-y divide-neutral-700/30"
    thClass="px-4 py-2.5 text-left text-xs font-medium text-gray-400 uppercase tracking-wider bg-neutral-700/20"
    emptyStateMessage={isLoadingJobs ? 'Loading jobs...' : (jobsError ? `Error: ${jobsError}` : 'No analysis jobs found for this organization yet.')}
    emptyStateIconPath="M3.75 12h16.5m-16.5 3.75h16.5M3.75 19.5h16.5M5.625 4.5h12.75a1.875 1.875 0 010 3.75H5.625a1.875 1.875 0 010-3.75z"
    trClass="hover:bg-neutral-700/40 transition-colors duration-150 group"
    tdClass="px-4 py-3 whitespace-nowrap text-sm text-gray-300"
    currentPage={currentPage}
    totalPages={totalJobPages}
    totalItems={totalJobs}
    itemsPerPage={jobsPerPage}
  >
    <!-- Custom slot for 'id' to truncate and add title -->
    <span slot="cell-id" let:value title={value}>
      {value.substring(0, 8)}...
    </span>

    <!-- Custom slot for 'document_name' to add title -->
    <span slot="cell-document_name" let:value title={value} class="group-hover:text-accent-lighter transition-colors">
        {value.length > 30 ? value.substring(0,27) + '...' : value}
    </span>

    <!-- Custom slot for 'pipeline_name' to add title -->
    <span slot="cell-pipeline_name" let:value title={value} class="group-hover:text-accent-lighter transition-colors">
        {value.length > 30 ? value.substring(0,27) + '...' : value}
    </span>

    <!-- Custom slot for 'status' column -->
    <div slot="cell-status" let:item class="flex items-center">
      <span
        class="px-2.5 py-0.5 inline-flex items-center text-xs leading-5 font-semibold rounded-full whitespace-nowrap
               {getStatusColorClass(item.status)}"
      >
        {item.status.charAt(0).toUpperCase() + item.status.slice(1).replace('_', ' ')}
      </span>
    </div>

    <!-- Custom slot for 'actions' column -->
    <div slot="cell-actions" let:item class="flex justify-end">
      <Button variant="ghost" customClass="!px-2 !py-1 text-xs !text-accent-lighter hover:!text-accent" on:click={() => viewJobDetails(item.id)}>
        View Details
      </Button>
    </div>
    <div slot="paginationControls" let:currentPageProps let:totalPagesProps>
      <PaginationControls currentPage={currentPageProps} totalPages={totalPagesProps} on:pageChange={handlePageChange} />
    </div>
  </DataTable>
