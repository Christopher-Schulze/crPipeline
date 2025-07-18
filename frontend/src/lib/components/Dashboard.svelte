<script lang="ts">
  import { onMount, onDestroy, getContext } from 'svelte'; // Added getContext
  import GlassCard from './GlassCard.svelte';
  import Chart from 'chart.js/auto';
  import { apiFetch } from '$lib/utils/apiUtils';
  import { errorStore } from '$lib/utils/errorStore';
  export let orgId: string;

  const viewJobDetails = getContext<(jobId: string) => void>('viewJobDetails'); // Get context

  let uploadRemaining = 0;
  let analysisRemaining = 0;
  let usageChart: Chart | null = null;
  let canvasEl: HTMLCanvasElement;

  let quotaError: string | null = null;
  let usageError: string | null = null;
  let recentError: string | null = null;

  // --- Recent Analyses State ---
  interface RecentAnalysisJob {
    job_id: string; // UUID
    document_name: string;
    pipeline_name: string;
    status: string;
    created_at: string; // ISO date string
    document_id: string; // UUID
  }
  let recentAnalyses: RecentAnalysisJob[] = [];

  // --- Data Loading Functions ---
  async function loadQuota() {
    if (!orgId) return;
    try {
      const res = await apiFetch(`/api/dashboard/${orgId}`);
      const data = await res.json();
      uploadRemaining = data.upload_remaining;
      analysisRemaining = data.analysis_remaining;
      quotaError = null;
    } catch (error: any) {
      quotaError = error.message || 'Failed to load quota data';
      errorStore.show(`Failed to load quota: ${quotaError}`);
    }
  }

  async function loadUsage() {
    if (!orgId || !canvasEl) return; // Ensure canvasEl is available
    try {
      const usageRes = await apiFetch(`/api/dashboard/${orgId}/usage`);
      const usage = await usageRes.json();
        const labels = usage.map((u: any) => u.month);
        const uploads = usage.map((u: any) => u.uploads);
        const analyses = usage.map((u: any) => u.analyses);
        const accent = getComputedStyle(document.documentElement)
          .getPropertyValue('--color-accent') || '#30D5C8';

        if (usageChart) usageChart.destroy();
        usageChart = new Chart(canvasEl, {
          type: 'bar',
          data: {
            labels,
            datasets: [
              { label: 'Uploads', data: uploads, backgroundColor: accent + '99' },
              { label: 'Analyses', data: analyses, backgroundColor: 'rgba(52,199,89,0.6)' }
            ]
          },
          options: {
            responsive: true,
            scales: { x: { stacked: false }, y: { beginAtZero: true } }
          }
        });
        usageError = null;
    } catch (error: any) {
      usageError = error.message || 'Failed to load usage data';
      errorStore.show(`Failed to load usage: ${usageError}`);
    }
  }

  async function loadRecentAnalyses() {
    if (!orgId) return;
    try {
      const res = await apiFetch(`/api/dashboard/${orgId}/recent_analyses`);
      recentAnalyses = await res.json();
      recentError = null;
    } catch (error: any) {
      recentError = error.message || 'Failed to load recent analyses';
      errorStore.show(`Failed to load analyses: ${recentError}`);
    }
  }

  onMount(async () => {
    if (orgId) {
      await Promise.all([loadQuota(), loadUsage(), loadRecentAnalyses()]);
    }
  });

  // Reactive data loading if orgId changes (and component is still mounted)
  // This ensures that if the orgId prop changes, data is re-fetched.
  // The `canvasEl` check ensures that `loadUsage` only runs after the canvas is bound.
  $: if (orgId && typeof window !== 'undefined') {
    // `typeof window !== 'undefined'` is a simple way to check if onMount has likely run
    // and we are in a browser context, preventing server-side rendering issues if any.
    Promise.all([loadQuota(), loadRecentAnalyses()]);
    if (canvasEl) { // Usage data depends on canvas, so load it separately if canvas is ready
        loadUsage();
    }
  }

  onDestroy(() => {
    usageChart?.destroy();
  });

  // --- Helper Functions ---
  function getStatusColor(status: string): string {
    // Ensure these text color classes are defined in Tailwind or global styles
    if (status === 'completed' || status === 'success') return 'text-green-600'; // Standard Tailwind green
    if (status === 'failed' || status === 'error') return 'text-error';
    if (status === 'in_progress' || status === 'running') return 'text-blue-500';
    return 'text-gray-500'; // Default for unknown or pending statuses
  }
</script>
<div class="grid gap-4 md:grid-cols-2">
  <GlassCard class="p-6 flex flex-col items-center">
    <h2 class="text-lg mb-2">Uploads Remaining</h2>
    <span class="text-4xl font-semibold">{uploadRemaining}</span>
  </GlassCard>
  <GlassCard class="p-6 flex flex-col items-center">
    <h2 class="text-lg mb-2">Analyses Remaining</h2>
    <span class="text-4xl font-semibold">{analysisRemaining}</span>
  </GlassCard>
</div>
{#if quotaError}
  <p class="text-error text-sm mt-2">{quotaError}</p>
{/if}
<GlassCard class="mt-8" padding="p-6">
  <canvas bind:this={canvasEl}></canvas>
</GlassCard>
{#if usageError}
  <p class="text-error text-sm mt-2">{usageError}</p>
{/if}

<!-- Recent Analyses Section -->
<GlassCard title="Recent Analyses" customClass="mt-6" padding="p-6">
  {#if recentAnalyses.length === 0}
    <p class="text-center text-gray-500 py-4">No recent analyses found.</p>
  {:else}
    <ul class="space-y-3">
      {#each recentAnalyses as job (job.job_id)}
        <li
          class="p-3 bg-white/20 hover:bg-white/30 rounded-lg shadow-sm transition-colors cursor-pointer group"
          on:click={() => viewJobDetails(job.job_id)}
          role="button"
          tabindex="0"
          on:keydown={(e) => { if (e.key === 'Enter' || e.key === ' ') viewJobDetails(job.job_id)}}
        >
          <div class="flex justify-between items-center">
            <span class="font-semibold text-gray-700 truncate group-hover:text-accent transition-colors" title={job.document_name}>{job.document_name}</span>
            <span class="text-xs text-gray-500 flex-shrink-0 ml-2">
              {new Date(job.created_at).toLocaleDateString()}
            </span>
          </div>
          <div class="text-sm text-gray-600 truncate" title={job.pipeline_name}>
            Pipeline: {job.pipeline_name}
          </div>
          <div class="text-sm">
            Status: <span class="font-medium {getStatusColor(job.status)}">{job.status}</span>
          </div>
        </li>
      {/each}
    </ul>
  {/if}
</GlassCard>
{#if recentError}
  <p class="text-error text-sm mt-2">{recentError}</p>
{/if}
