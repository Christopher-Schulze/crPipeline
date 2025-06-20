<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import GlassCard from './GlassCard.svelte';
  import Chart from 'chart.js/auto';
  export let orgId: string;
  let uploadRemaining = 0;
  let analysisRemaining = 0;
  let usageChart: Chart | null = null;
  let canvasEl: HTMLCanvasElement;
  onMount(async () => {
    const res = await fetch(`/api/dashboard/${orgId}`);
    if (res.ok) {
      const data = await res.json();
      uploadRemaining = data.upload_remaining;
      analysisRemaining = data.analysis_remaining;
    }

    const usageRes = await fetch(`/api/dashboard/${orgId}/usage`);
    if (usageRes.ok) {
      const usage = await usageRes.json();
      const labels = usage.map((u: any) => u.month);
      const uploads = usage.map((u: any) => u.uploads);
      const analyses = usage.map((u: any) => u.analyses);
      const accent = getComputedStyle(document.documentElement)
        .getPropertyValue('--color-accent') || '#30D5C8';
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
    }
  });

  onDestroy(() => {
    usageChart?.destroy();
  });
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
<GlassCard class="mt-8 p-4">
  <canvas bind:this={canvasEl}></canvas>
</GlassCard>
