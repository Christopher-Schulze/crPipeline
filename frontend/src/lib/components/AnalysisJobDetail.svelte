<script lang="ts">
  import { onMount, createEventDispatcher } from 'svelte';
  import GlassCard from './GlassCard.svelte';
  import Button from './Button.svelte';
  import Modal from './Modal.svelte';
  import Spinner from './Spinner.svelte';
  import { createReconnectingEventSource, type ReconnectingEventSource } from '$lib/utils/eventSourceUtils';
  import * as Diff from 'diff'; // Import the diff library
  import { apiFetch } from '$lib/utils/apiUtils';
  import { errorStore } from '$lib/utils/errorStore';

  export let jobId: string;

  // Updated TypeScript Interfaces
  interface StageOutput {
    id: string; // UUID
    job_id: string; // UUID
    stage_name: string;
    output_type: string; // "json", "pdf", "txt", etc.
    s3_bucket: string;
    s3_key: string;
    created_at: string; // ISO date string
  }

  interface JobDetails {
    // From AnalysisJob
    id: string; // UUID
    org_id: string; // UUID
    document_id: string; // UUID
    pipeline_id: string; // UUID
    status: string;
    job_created_at: string; // ISO date string

    // From Document
    document_name: string;

    // From Pipeline
    pipeline_name: string;

    // From JobStageOutput
    stage_outputs: StageOutput[];
  }

  let jobDetails: JobDetails | null = null;
  let isLoading: boolean = true; // Start with loading true
  let error: string | null = null;

  // State variables for viewing output content
  let viewingOutputContent: string | null = null;
  let viewingOutputTitle: string = '';
  let viewingOutputType: 'txt' | 'json' | null = null;
  let isLoadingOutputContent: boolean = false;
  let showOutputViewerModal: boolean = false;

  // New state for dedicated OCR text display
  let ocrTextOutput: string | null = null;
  let isLoadingOcrText: boolean = false;
  let ocrTextError: string | null = null;
  let ocrOutputToDisplay: StageOutput | null = null;

  // State for Parse JSON output
  let parseJsonOutput: string | null = null;
  let isLoadingParseJson: boolean = false;
  let parseJsonError: string | null = null;
  let parseOutputToDisplay: StageOutput | null = null;

  // State for AI JSON output
  let aiJsonOutput: string | null = null;
  let isLoadingAiJson: boolean = false;
  let aiJsonError: string | null = null;
  let aiOutputToDisplay: StageOutput | null = null;

  // New State Variables for AI Diff View
  let aiInputJsonForDiff: string | null = null;
  let aiOutputJsonForDiff: string | null = null;
  let isLoadingAiDiff: boolean = false;
  let aiDiffError: string | null = null;
  let showAiDiffModal: boolean = false;

  let aiInputStageOutputMetadata: StageOutput | null = null;
  let aiOutputStageOutputMetadata: StageOutput | null = null;

  // State variables for OCR Text / Parse JSON Compare Modal
  let showOcrJsonCompareModal: boolean = false;
  let ocrTextForCompareModal: string | null = null;
  let parseJsonForCompareModal: string | null = null;
  let isLoadingOcrParseCompare: boolean = false;
  let ocrParseCompareError: string | null = null;
  let ocrParseDiffViewHtml: string | null = null; // For OCR vs Parse JSON diff

  // For AI Diff
  let aiDiffViewHtml: string | null = null;       // For AI Input vs Output diff

  // SSE and polling
  let stream: ReconnectingEventSource | null = null;
  let pollTimer: ReturnType<typeof setInterval> | null = null;


  const dispatch = createEventDispatcher();

  function handleClose() {
    dispatch('close');
  }

  async function fetchJobDetails(id: string) {
    isLoading = true;
    error = null;
    try {
      const response = await apiFetch(`/api/jobs/${id}/details`);
      const data: JobDetails = await response.json();
      jobDetails = data;

      // Auto-load first OCR text output if available
      const firstOcrTextOutput = jobDetails?.stage_outputs.find(
        (so) => so.stage_name.toLowerCase().includes('ocr') && so.output_type === 'txt'
      );
      if (firstOcrTextOutput) {
        loadOcrTextOutput(firstOcrTextOutput); // Don't await, let it load in background
      } else {
        ocrTextOutput = null;
        ocrOutputToDisplay = null;
        isLoadingOcrText = false;
        ocrTextError = null;
      }

      // Find and load first "parse" JSON output
      const parseOutput = data.stage_outputs.find(
        (so) => so.stage_name.toLowerCase().includes('parse') && so.output_type === 'json'
      );
      if (parseOutput) {
        loadJsonOutput(parseOutput, (c) => parseJsonOutput = c, (l) => isLoadingParseJson = l, (e) => parseJsonError = e, (o) => parseOutputToDisplay = o);
      } else {
        parseJsonOutput = null; parseOutputToDisplay = null; isLoadingParseJson = false; parseJsonError = null;
      }

      // Find and load first "ai" JSON output
      const aiOutput = data.stage_outputs.find(
        (so) => so.stage_name.toLowerCase().includes('ai') && so.output_type === 'json'
      );
      if (aiOutput) {
        loadJsonOutput(aiOutput, (c) => aiJsonOutput = c, (l) => isLoadingAiJson = l, (e) => aiJsonError = e, (o) => aiOutputToDisplay = o);
      } else {
        aiJsonOutput = null; aiOutputToDisplay = null; isLoadingAiJson = false; aiJsonError = null;
      }

      // Identify AI input and output stage metadata after jobDetails are loaded
      aiInputStageOutputMetadata = data.stage_outputs.find(
        (so) => so.stage_name.toLowerCase().endsWith('_input') && so.stage_name.toLowerCase().includes('ai') && so.output_type === 'json'
      ) || null;
      aiOutputStageOutputMetadata = data.stage_outputs.find(
        (so) => so.stage_name.toLowerCase() === 'ai' && so.output_type === 'json' // Assuming primary AI output is named 'ai'
      ) || null;

      // Reset diff view states if job changes
      aiInputJsonForDiff = null;
      aiOutputJsonForDiff = null;
      // showAiDiffModal = false; // Don't close it if it was already open for this job and job details just re-fetched.
      aiDiffError = null;


    } catch (e: any) {
      error = e.message;
      jobDetails = null;
      // Clear all output states if main job fetch fails
      ocrTextOutput = null; ocrOutputToDisplay = null; isLoadingOcrText = false; ocrTextError = null;
      parseJsonOutput = null; parseOutputToDisplay = null; isLoadingParseJson = false; parseJsonError = null;
      aiJsonOutput = null; aiOutputToDisplay = null; isLoadingAiJson = false; aiJsonError = null;

      aiInputStageOutputMetadata = null; aiOutputStageOutputMetadata = null;
      aiInputJsonForDiff = null; aiOutputJsonForDiff = null; aiDiffError = null;

      console.error("Error fetching job details:", e);
    } finally {
      isLoading = false;
    }
  }

  function startPolling() {
    if (pollTimer) return;
    pollTimer = setInterval(() => {
      if (jobId) fetchJobDetails(jobId);
    }, 5000);
  }

  function stopPolling() {
    if (pollTimer) {
      clearInterval(pollTimer);
      pollTimer = null;
    }
  }

  function startStream() {
    if (!jobId || typeof EventSource === 'undefined') {
      startPolling();
      return;
    }
    startPolling();
    stream = createReconnectingEventSource(
      `/api/jobs/${jobId}/events`,
      (e: MessageEvent) => {
        const status = e.data;
        if (status && status !== jobDetails?.status) {
          fetchJobDetails(jobId);
        }
        if (status === 'completed' || status === 'failed') {
          stopPolling();
          stream?.close();
        }
      },
      1000,
      () => stopPolling(),
      () => startPolling()
    );
  }

  onMount(() => {
    if (jobId) {
      fetchJobDetails(jobId);
      startStream();
    } else {
      error = "Job ID is missing.";
      isLoading = false;
    }
  });

  onDestroy(() => {
    stream?.close();
    if (pollTimer) {
      clearInterval(pollTimer);
      pollTimer = null;
    }
  });

  // Reactive fetch if jobId changes (optional, as App.svelte remounts it)
  // $: if (jobId && changed(jobId) && !isLoading) { // 'changed' is not a Svelte built-in
  //   fetchJobDetails(jobId);
  // }
  // Simpler reactivity for jobId changes if the component isn't destroyed/recreated:
  // $: if (jobId !== jobDetails?.id && jobId && !isLoading && jobDetails !== undefined) {
  //    // Logic from onMount could be refactored if reactive updates are common for jobId prop
  // }

  async function openOcrJsonCompareModal() {
    if (!ocrOutputToDisplay || !parseOutputToDisplay) {
        ocrParseCompareError = "Primary OCR Text or Parse JSON output metadata is not available for comparison.";
        ocrTextForCompareModal = null;
        parseJsonForCompareModal = null;
        isLoadingOcrParseCompare = false;
        showOcrJsonCompareModal = true; // Open modal to show this specific error
        return;
    }

    showOcrJsonCompareModal = true;
    isLoadingOcrParseCompare = true;
    ocrParseCompareError = null;
    ocrTextForCompareModal = null; // Clear previous content
    parseJsonForCompareModal = null; // Clear previous

    let tempOcrText: string | null = null;
    let tempParseJson: string | null = null;
    let success = true;

    try {
        // Use already loaded content if available from embedded viewers
        if (ocrTextOutput && ocrOutputToDisplay /* no specific ID check, assume it's the one */) {
            tempOcrText = ocrTextOutput;
            console.info(`Using pre-loaded OCR text for comparison: ${ocrOutputToDisplay.stage_name}`);
        } else if (ocrOutputToDisplay) { // Ensure metadata exists before fetching
            console.info(`Fetching OCR text for comparison: ${ocrOutputToDisplay.stage_name}`);
            const presignedUrlResponse = await apiFetch(`/api/jobs/outputs/${ocrOutputToDisplay.id}/download_url`);
            if (!presignedUrlResponse.ok) { success = false; throw new Error(`OCR Text (Compare): ${ (await presignedUrlResponse.json().catch(() => ({}))).error || presignedUrlResponse.statusText }`); }
            const presignedUrlData = await presignedUrlResponse.json();
            if (!presignedUrlData.url) { success = false; throw new Error("OCR Text URL (Compare) not found."); }
            const contentResponse = await apiFetch(presignedUrlData.url);
            if (!contentResponse.ok) { success = false; throw new Error(`OCR Text Fetch (Compare): ${contentResponse.statusText}`); }
            tempOcrText = await contentResponse.text();
        } else { // Should have been caught by the initial check but safeguard
            success = false; throw new Error("OCR Output metadata missing for fetch.");
        }

        if (parseJsonOutput && parseOutputToDisplay /* no specific ID check, assume it's the one */) {
            tempParseJson = parseJsonOutput; // Already pretty-printed by loadJsonOutput
            console.info(`Using pre-loaded Parse JSON for comparison: ${parseOutputToDisplay.stage_name}`);
        } else if (parseOutputToDisplay) { // Ensure metadata exists before fetching
            console.info(`Fetching Parse JSON for comparison: ${parseOutputToDisplay.stage_name}`);
            const presignedUrlResponse = await apiFetch(`/api/jobs/outputs/${parseOutputToDisplay.id}/download_url`);
            if (!presignedUrlResponse.ok) { success = false; throw new Error(`Parse JSON (Compare): ${ (await presignedUrlResponse.json().catch(() => ({}))).error || presignedUrlResponse.statusText }`); }
            const presignedUrlData = await presignedUrlResponse.json();
            if (!presignedUrlData.url) { success = false; throw new Error("Parse JSON URL (Compare) not found."); }
            const contentResponse = await apiFetch(presignedUrlData.url);
            if (!contentResponse.ok) { success = false; throw new Error(`Parse JSON Fetch (Compare): ${contentResponse.statusText}`); }
            const rawJson = await contentResponse.text();
            try {
                tempParseJson = JSON.stringify(JSON.parse(rawJson), null, 2);
            } catch (e) {
                tempParseJson = `Error parsing Parse JSON (Compare). Raw content:\n\n${rawJson}`;
                console.warn(`Failed to parse Parse JSON for comparison modal, showing raw: ${e}`);
            }
        } else { // Should have been caught by initial check
             success = false; throw new Error("Parse Output metadata missing for fetch.");
        }

        if(success) {
            // Store original texts for potential copying
            ocrTextForCompareModal = tempOcrText;
            parseJsonForCompareModal = tempParseJson;
            if (tempOcrText !== null && tempParseJson !== null) {
                ocrParseDiffViewHtml = generateDiffViewHtml(tempOcrText, tempParseJson);
            } else {
                ocrParseDiffViewHtml = "<span class='text-red-400'>One or both contents could not be loaded for diff.</span>";
                if (!ocrParseCompareError && (!tempOcrText || !tempParseJson)) { // Set a generic error if not already set by fetch
                    ocrParseCompareError = "One or both contents are missing.";
                }
            }
        } else {
             ocrParseDiffViewHtml = `<span class='text-red-400'>Error loading data for diff: ${ocrParseCompareError || "Unknown error"}</span>`;
        }

    } catch (err: any) {
        console.error("Failed to load data for OCR/JSON comparison:", err);
        ocrParseCompareError = err.message; // This will be displayed if ocrParseDiffViewHtml isn't set with specific error
        ocrParseDiffViewHtml = `<span class='text-red-400'>Error loading data for diff: ${err.message}</span>`;
    } finally {
        isLoadingOcrParseCompare = false;
    }
  }

  async function viewAiDiff() {
    if (!aiInputStageOutputMetadata || !aiOutputStageOutputMetadata) {
      aiDiffError = "AI input or output data metadata is not available for diffing.";
      aiInputJsonForDiff = null; // Clear previous
      aiOutputJsonForDiff = null; // Clear previous
      aiDiffViewHtml = `<span class='text-red-400'>${aiDiffError}</span>`;
      showAiDiffModal = true;
      isLoadingAiDiff = false;
      return;
    }

    isLoadingAiDiff = true;
    aiDiffError = null;
    aiInputJsonForDiff = null;
    aiOutputJsonForDiff = null;
    aiDiffViewHtml = null; // Clear previous diff
    showAiDiffModal = true;

    let tempAiInputJson: string | null = null;
    let tempAiOutputJson: string | null = null;

    try {
      // Fetch AI Input
      let presignedUrlResponse = await apiFetch(`/api/jobs/outputs/${aiInputStageOutputMetadata.id}/download_url`);
      if (!presignedUrlResponse.ok) throw new Error(`AI Input: ${ (await presignedUrlResponse.json().catch(() => ({}))).error || presignedUrlResponse.statusText }`);
      let presignedUrlData = await presignedUrlResponse.json();
      if (!presignedUrlData.url) throw new Error("AI Input URL not found.");
      let contentResponse = await apiFetch(presignedUrlData.url);
      if (!contentResponse.ok) throw new Error(`AI Input Fetch: ${contentResponse.statusText}`);
      const rawInput = await contentResponse.text();
      try {
          tempAiInputJson = JSON.stringify(JSON.parse(rawInput), null, 2);
      } catch (e) {
          tempAiInputJson = `Error parsing AI Input JSON. Raw content:\n\n${rawInput}`;
          console.warn("AI Input JSON parsing error for diff:", e);
      }
      aiInputJsonForDiff = tempAiInputJson; // Store for copy button

      // Fetch AI Output
      presignedUrlResponse = await apiFetch(`/api/jobs/outputs/${aiOutputStageOutputMetadata.id}/download_url`);
      if (!presignedUrlResponse.ok) throw new Error(`AI Output: ${ (await presignedUrlResponse.json().catch(() => ({}))).error || presignedUrlResponse.statusText }`);
      presignedUrlData = await presignedUrlResponse.json();
      if (!presignedUrlData.url) throw new Error("AI Output URL not found.");
      contentResponse = await apiFetch(presignedUrlData.url);
      if (!contentResponse.ok) throw new Error(`AI Output Fetch: ${contentResponse.statusText}`);
      const rawOutput = await contentResponse.text();
      try {
          tempAiOutputJson = JSON.stringify(JSON.parse(rawOutput), null, 2);
      } catch (e) {
          tempAiOutputJson = `Error parsing AI Output JSON. Raw content:\n\n${rawOutput}`;
          console.warn("AI Output JSON parsing error for diff:", e);
      }
      aiOutputJsonForDiff = tempAiOutputJson; // Store for copy button

      if (tempAiInputJson !== null && tempAiOutputJson !== null) {
        aiDiffViewHtml = generateDiffViewHtml(tempAiInputJson, tempAiOutputJson);
      } else {
        aiDiffViewHtml = "<span class='text-red-400'>One or both AI JSON contents could not be fully processed for diff.</span>";
        if (!aiDiffError) aiDiffError = "Content loading or parsing issue prevented diff generation.";
      }

    } catch (err: any) {
      console.error("Failed to load AI diff data:", err);
      aiDiffError = err.message;
      aiDiffViewHtml = `<span class='text-red-400'>Error loading data for diff: ${err.message}</span>`;
    } finally {
      isLoadingAiDiff = false;
    }
  }

  // New helper function
  function generateDiffViewHtml(text1: string, text2: string): string {
    const diffResult = Diff.diffLines(text1, text2, { newlineIsToken: true, ignoreWhitespace: false });

    let html = '';
    diffResult.forEach(part => {
      const colorClass = part.added ? 'bg-green-500/20 text-green-100' :
                        part.removed ? 'bg-red-500/20 text-red-100' :
                        'text-gray-300 dark:text-gray-300';

      const escapedValue = part.value.replace(/&/g, '&amp;')
                                   .replace(/</g, '&lt;')
                                   .replace(/>/g, '&gt;');

      html += `<span class="${colorClass}">${escapedValue}</span>`;
    });
    return html;
  }

  async function loadJsonOutput(
    output: StageOutput,
    setContent: (content: string | null) => void,
    setLoading: (loading: boolean) => void,
    setError: (error: string | null) => void,
    setOutputToDisplay: (outputDisplay: StageOutput | null) => void
  ) {
    if (!output || output.output_type !== 'json') {
        setError(`Invalid output type for JSON display (${output?.stage_name || 'N/A'}).`);
        return;
    }

    setLoading(true);
    setContent(null);
    setError(null);
    setOutputToDisplay(output);

    try {
      const presignedUrlResponse = await apiFetch(`/api/jobs/outputs/${output.id}/download_url`);
      if (!presignedUrlResponse.ok) {
        const errorData = await presignedUrlResponse.json().catch(() => ({}));
        throw new Error(errorData.error || `Failed to get JSON URL (${output.stage_name}): ${presignedUrlResponse.statusText}`);
      }
      const presignedUrlData = await presignedUrlResponse.json();
      if (!presignedUrlData.url) {
        throw new Error(`JSON URL not found for ${output.stage_name}.`);
      }

      const contentResponse = await apiFetch(presignedUrlData.url);
      if (!contentResponse.ok) {
        throw new Error(`Failed to fetch JSON content from S3 (${output.stage_name}): ${contentResponse.statusText}`);
      }

      const rawContent = await contentResponse.text();
      try {
        const parsedJson = JSON.parse(rawContent);
        setContent(JSON.stringify(parsedJson, null, 2)); // Pretty print
      } catch (jsonError) {
        console.error(`Failed to parse JSON content for ${output.stage_name}:`, jsonError);
        // Set error or display raw content with a note
        setContent(`Error parsing JSON. Raw content below:\n\n${rawContent}`);
        // setError(`Error parsing JSON for ${output.stage_name}. Raw content displayed above.`);
      }
    } catch (err: any) {
      console.error(`Failed to load JSON output for ${output.stage_name}:`, err);
      setError(err.message);
      setContent(null);
    } finally {
      setLoading(false);
    }
  }

  async function copyToClipboard(text: string | null, type: string) {
    if (!text || typeof navigator.clipboard?.writeText !== 'function') {
      errorStore.show('Clipboard API not available or no text to copy.');
      return;
    }
    try {
      await navigator.clipboard.writeText(text);
      // Optionally show success toast here in the future
    } catch (err) {
      console.error(`Failed to copy ${type} to clipboard:`, err);
      errorStore.show(`Failed to copy ${type}. See console for details.`);
    }
  }

  async function loadOcrTextOutput(output: StageOutput) {
    if (!output || output.output_type !== 'txt') {
      ocrTextError = "Invalid output type for OCR text display.";
      return;
    }

    isLoadingOcrText = true;
    ocrTextOutput = null;
    ocrTextError = null;
    ocrOutputToDisplay = output;

    try {
      const presignedUrlResponse = await apiFetch(`/api/jobs/outputs/${output.id}/download_url`);
      if (!presignedUrlResponse.ok) {
        const errorData = await presignedUrlResponse.json().catch(() => ({}));
        throw new Error(errorData.error || `Failed to get OCR text URL: ${presignedUrlResponse.statusText}`);
      }
      const presignedUrlData = await presignedUrlResponse.json();
      if (!presignedUrlData.url) {
        throw new Error("OCR text URL not found in server response.");
      }

      const contentResponse = await apiFetch(presignedUrlData.url);
      if (!contentResponse.ok) {
        throw new Error(`Failed to fetch OCR text content from S3: ${contentResponse.statusText}`);
      }
      ocrTextOutput = await contentResponse.text();
    } catch (err: any) {
      console.error("Failed to load OCR text output:", err);
      ocrTextError = err.message;
    } finally {
      isLoadingOcrText = false;
    }
  }

  async function viewStageOutput(output: StageOutput) {
    if (output.output_type !== 'txt' && output.output_type !== 'json') {
      errorStore.show("Viewing is currently supported only for .txt and .json files.");
      return;
    }

    isLoadingOutputContent = true;
    viewingOutputContent = null;
    viewingOutputType = null;
    viewingOutputTitle = `Output: ${output.stage_name} (${output.output_type})`;

    try {
      const presignedUrlResponse = await apiFetch(`/api/jobs/outputs/${output.id}/download_url`);
      if (!presignedUrlResponse.ok) {
        const errorData = await presignedUrlResponse.json().catch(() => ({}));
        throw new Error(errorData.error || `Failed to get view URL: ${presignedUrlResponse.statusText}`);
      }
      const presignedUrlData = await presignedUrlResponse.json();
      if (!presignedUrlData.url) {
        throw new Error("View URL not found in server response.");
      }

      const contentResponse = await apiFetch(presignedUrlData.url);
      if (!contentResponse.ok) {
        throw new Error(`Failed to fetch content from S3: ${contentResponse.statusText}`);
      }

      const rawContent = await contentResponse.text();

      if (output.output_type === 'json') {
        try {
          const parsedJson = JSON.parse(rawContent);
          viewingOutputContent = JSON.stringify(parsedJson, null, 2); // Pretty print
          viewingOutputType = 'json';
        } catch (jsonError) {
          console.error("Failed to parse JSON content:", jsonError);
          viewingOutputContent = `Error parsing JSON. Raw content:\n\n${rawContent}`;
          viewingOutputType = 'txt';
        }
      } else { // txt
        viewingOutputContent = rawContent;
        viewingOutputType = 'txt';
      }
      showOutputViewerModal = true; // Open the modal with content

    } catch (err: any) {
      console.error("Failed to view stage output:", err);
      viewingOutputContent = `Error loading content: ${err.message}`;
      viewingOutputTitle = "Error Loading Output"; // Set title for error view
      viewingOutputType = 'txt';
      showOutputViewerModal = true; // Open modal to show error
    } finally {
      isLoadingOutputContent = false;
    }
  }

  function closeOutputViewer() {
    showOutputViewerModal = false;
    // Optionally reset content, or let it persist until next 'View' click
    // viewingOutputContent = null;
    // viewingOutputTitle = '';
    // viewingOutputType = null;
  }

  async function downloadStageOutput(outputId: string, filenameSuggestion?: string) {
    try {
      const response = await apiFetch(`/api/jobs/outputs/${outputId}/download_url`);
      if (!response.ok) {
        const errorData = await response.json().catch(() => ({})); // Try to get error message
        let message = errorData.error || errorData.detail || `Failed to get download URL: ${response.statusText}`;
        if (response.status === 404) message = "Download link not found or file is no longer available.";
        throw new Error(message);
      }
      const data = await response.json();
      if (data.url) {
        const link = document.createElement('a');
        link.href = data.url;
        if (filenameSuggestion) {
          link.download = filenameSuggestion;
        }
        document.body.appendChild(link);
        link.click();
        document.body.removeChild(link);
      } else {
        throw new Error("Download URL not found in the server response.");
      }
  } catch (err: any) {
    console.error("Download failed for output ID", outputId, err);
    errorStore.show(`Could not initiate download: ${err.message}`);
  }
}

  function getStatusColor(status: string): string {
    if (status === 'completed' || status === 'success') return 'text-green-400';
    if (status === 'failed' || status === 'error') return 'text-error';
    if (status === 'in_progress' || status === 'running') return 'text-blue-400';
    return 'text-gray-300'; // Default for unknown or pending statuses
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      handleClose();
    }
  }
</script>

<svelte:window on:keydown={handleKeydown} />

<!-- Backdrop -->
<div
  class="fixed inset-0 bg-black/60 backdrop-blur-lg z-30 flex items-start justify-center p-4 sm:p-8 pt-[5vh] sm:pt-[10vh] overflow-y-auto"
  on:click|self={handleClose}
>
  <!-- Modal Panel using GlassCard -->
  <GlassCard
    customClass="w-full max-w-3xl !shadow-2xl !border-white/30" <!-- More prominent shadow & border -->
    bgOpacity="!bg-neutral-800/80" <!-- Darker glass for this modal -->
    padding="p-0" <!-- Control padding internally -->
    on:click|stopPropagation
  >
    <!-- Header -->
    <div class="flex justify-between items-center p-4 border-b border-white/10">
        <h2 id="modal-title-text" class="text-xl font-semibold text-gray-100 truncate pr-2">
            Job: {jobDetails?.id || jobId || 'Details'}
        </h2>
        <button
          on:click={handleClose}
          aria-label="Close details"
          class="p-1 rounded-full hover:bg-white/20 text-gray-300 hover:text-gray-100 transition-colors"
        >
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-6 h-6">
            <path stroke-linecap="round" stroke-linejoin="round" d="M6 18L18 6M6 6l12 12" />
          </svg>
        </button>
    </div>

    <div class="p-5 sm:p-6 space-y-4 max-h-[calc(90vh-140px)] overflow-y-auto"> <!-- Content Padding & Scroll, adjusted max-h -->
        {#if isLoading}
          <Spinner message="Loading job details..." />
        {:else if error}
          <div class="p-4 bg-error/30 rounded-md text-center">
            <p class="text-error">Error: {error}</p>
          </div>
        {:else if jobDetails}
          <div class="space-y-4">
            <div class="grid grid-cols-1 md:grid-cols-2 gap-x-4 gap-y-3"> <!-- Removed base text-sm from grid div -->
              <div><span class="text-sm font-light text-gray-500 dark:text-gray-400 mr-1">Job ID:</span> <span class="text-sm text-gray-100 dark:text-gray-50 font-mono">{jobDetails.id}</span></div>
              <div>
                <span class="text-sm font-light text-gray-500 dark:text-gray-400 mr-1">Status:</span>
                <span class="font-semibold px-2 py-0.5 rounded-full text-xs {getStatusColor(jobDetails.status)}
                  {jobDetails.status === 'completed' ? 'bg-green-800/70' :
                   jobDetails.status === 'failed' ? 'bg-error/70' :
                   jobDetails.status === 'in_progress' ? 'bg-blue-800/70' :
                   'bg-gray-700/70'}">
                  {jobDetails.status}
                </span>
              </div>
              <div><span class="text-sm font-light text-gray-500 dark:text-gray-400 mr-1">Document:</span> <span class="text-sm text-gray-100 dark:text-gray-50 truncate" title={jobDetails.document_name}>{jobDetails.document_name}</span></div>
              <div><span class="text-sm font-light text-gray-500 dark:text-gray-400 mr-1">Pipeline:</span> <span class="text-sm text-gray-100 dark:text-gray-50 truncate" title={jobDetails.pipeline_name}>{jobDetails.pipeline_name}</span></div>
              <div class="md:col-span-2"><span class="text-sm font-light text-gray-500 dark:text-gray-400 mr-1">Created:</span> <span class="text-sm text-gray-100 dark:text-gray-50">{new Date(jobDetails.job_created_at).toLocaleString()}</span></div>
            </div>

            <hr class="border-white/10 my-4"/>

            <section>
              <h3 class="text-lg font-semibold mb-2 text-gray-200">OCR Text Output</h3>
              {#if jobDetails?.stage_outputs.find(so => so.stage_name.toLowerCase().includes('ocr') && so.output_type === 'txt')}
                <GlassCard
                  title={ocrOutputToDisplay ? `Preview: ${ocrOutputToDisplay.stage_name} (${ocrOutputToDisplay.output_type})` : "OCR Text"}
                  padding="p-0"
                  customClass="mt-1 mb-3 !bg-neutral-900/50 !border-neutral-700"
                >
                  <div class="p-3 max-h-[300px] overflow-y-auto custom-scrollbar relative">
                    {#if isLoadingOcrText}
                      <Spinner message="Loading OCR text..." size="sm" />
                    {:else if ocrTextError}
                      <p class="text-error bg-error/30 p-2 rounded-md">Error: {ocrTextError}</p>
                    {:else if ocrTextOutput}
                       <Button variant="ghost" customClass="!absolute top-1 right-1 !px-1.5 !py-0.5 text-xs z-10" on:click={() => copyToClipboard(ocrTextOutput, 'OCR Text')}>Copy</Button>
                      <pre class="whitespace-pre-wrap break-all text-xs text-gray-300 bg-transparent pt-5">
                        {ocrTextOutput}
                      </pre>
                    {:else}
                      <p class="text-gray-400 text-center py-4">No OCR text loaded or available for preview.</p>
                    {/if}
                  </div>
                </GlassCard>
              {:else}
                 <div class="p-4 bg-black/30 rounded-md text-sm text-gray-400 border border-white/10">
                   No primary OCR text output found for direct preview. Other outputs may be available below.
                 </div>
              {/if}
            </section>

            {#if jobDetails?.stage_outputs.find(so => so.stage_name.toLowerCase().includes('parse') && so.output_type === 'json')}
              <GlassCard
                title={parseOutputToDisplay ? `Parse Output: ${parseOutputToDisplay.stage_name} (JSON)` : "Parse Output (JSON)"}
                padding="p-0"
                customClass="mt-4 !bg-neutral-900/50 !border-neutral-700"
              >
                <div class="p-3 max-h-[400px] overflow-y-auto custom-scrollbar relative">
                  {#if isLoadingParseJson}
                    <Spinner message="Loading Parse JSON output..." size="sm" />
                  {:else if parseJsonError}
                    <p class="text-error bg-error/30 p-2 rounded-md">Error: {parseJsonError}</p>
                  {:else if parseJsonOutput}
                    <Button variant="ghost" customClass="!absolute top-1 right-1 !px-1.5 !py-0.5 text-xs z-10" on:click={() => copyToClipboard(parseJsonOutput, 'Parse JSON')}>Copy</Button>
                    <pre class="whitespace-pre-wrap break-all text-xs text-gray-200 bg-transparent pt-5">
                      {parseJsonOutput}
                    </pre>
                  {:else}
                    <p class="text-gray-400 text-center py-4">No Parse JSON output found or selected.</p>
                  {/if}
                </div>
              </GlassCard>
            {/if}

            {#if jobDetails?.stage_outputs.find(so => so.stage_name.toLowerCase().includes('ai') && so.output_type === 'json')}
              <GlassCard
                title={aiOutputToDisplay ? `AI Output: ${aiOutputToDisplay.stage_name} (JSON)` : "AI Output (JSON)"}
                padding="p-0"
                customClass="mt-4 !bg-neutral-900/50 !border-neutral-700"
              >
                <div class="p-3 max-h-[400px] overflow-y-auto custom-scrollbar relative">
                  {#if isLoadingAiJson}
                    <Spinner message="Loading AI JSON output..." size="sm" />
                  {:else if aiJsonError}
                    <p class="text-error bg-error/30 p-2 rounded-md">Error: {aiJsonError}</p>
                  {:else if aiJsonOutput}
                    <Button variant="ghost" customClass="!absolute top-1 right-1 !px-1.5 !py-0.5 text-xs z-10" on:click={() => copyToClipboard(aiJsonOutput, 'AI JSON')}>Copy</Button>
                    <pre class="whitespace-pre-wrap break-all text-xs text-gray-200 bg-transparent pt-5">
                      {aiJsonOutput}
                    </pre>
                  {:else}
                    <p class="text-gray-400 text-center py-4">No AI JSON output found or selected.</p>
                  {/if}
                </div>
              </GlassCard>
            {/if}


            <hr class="border-white/10 my-4"/>

            <div class="flex justify-center my-4">
              <div class="btn-group">
                {#if ocrOutputToDisplay && parseOutputToDisplay}
                  <Button variant="primary" on:click={openOcrJsonCompareModal} customClass="text-sm !py-1.5">Compare OCR & Parse Output</Button>
                {/if}
                {#if aiInputStageOutputMetadata && aiOutputStageOutputMetadata}
                  <Button variant="secondary" on:click={viewAiDiff} customClass="text-sm !py-1.5">AI Input / Output</Button>
                {/if}
              </div>
            </div>

            <section>
              <h3 class="text-lg font-semibold mb-2 text-gray-200">All Stage Outputs</h3>
              {#if jobDetails.stage_outputs && jobDetails.stage_outputs.length > 0}
                <div class="space-y-3">
                  {#each jobDetails.stage_outputs as output (output.id)}
                    <div class="p-3 bg-black/20 rounded-md flex justify-between items-center border border-neutral-700/50 hover:border-neutral-600/80 transition-colors">
                      <div class="truncate pr-2">
                        <span class="font-semibold text-gray-200 truncate block" title={output.stage_name}>{output.stage_name}</span>
                        <span class="ml-1 px-2 py-0.5 inline-flex text-xs leading-4 font-semibold rounded-full bg-neutral-600/60 text-neutral-300">
                          {output.output_type}
                        </span>
                      </div>
                      <div class="flex items-center flex-shrink-0">
                          <div class="btn-group">
                          {#if output.output_type === 'txt' || output.output_type === 'json'}
                              <Button variant="ghost" customClass="text-xs !py-1 !px-2 !text-sky-400 hover:!text-sky-300 hover:!bg-sky-500/10" on:click={() => viewStageOutput(output)}>
                                  View
                              </Button>
                          {/if}
                          <Button
                              variant="secondary"
                              customClass="text-xs !py-1 !px-2"
                              on:click={() => {
                                if (jobDetails) {
                                  let baseName = jobDetails.document_name && jobDetails.document_name.trim() !== '' ? jobDetails.document_name : `job_${jobDetails.id}`;
                                  baseName = baseName.replace(/\.[^/.]+$/, ""); // Remove existing extension

                                  let suggestedFilename = `${baseName}_${output.stage_name}.${output.output_type}`;
                                  suggestedFilename = suggestedFilename.replace(/[\s\\/:*?"<>|]+/g, '_').replace(/__+/g, '_'); // Sanitize and collapse multiple underscores
                                  downloadStageOutput(output.id, suggestedFilename);
                                }
                              }}
                              title={`S3 Path: s3://${output.s3_bucket}/${output.s3_key}`}
                              disabled={!jobDetails}
                          >
                              Download
                          </Button>
                          </div>
                      </div>
                    </div>
                  {/each}
                </div>
              {:else}
                <div class="p-4 bg-black/20 rounded-md text-center border border-white/10">
                    <p class="text-gray-400">No individual stage outputs recorded for this job.</p>
                </div>
              {/if}
            </section>
          </div>
        {:else}
           <div class="flex justify-center items-center min-h-[200px]">
            <p class="text-gray-400">No job details available.</p>
          </div>
        {/if}
    </div>

    <div class="p-4 border-t border-white/10 flex justify-end sticky bottom-0 bg-neutral-800/80 backdrop-blur-sm rounded-b-xl">
         <Button variant="primary" on:click={handleClose}>Close</Button>
    </div>
  </GlassCard>
</div>

<!-- Modal for Viewing Stage Output Content -->
<Modal
  isOpen={showOutputViewerModal}
  title={viewingOutputTitle}
  on:close={closeOutputViewer}
  maxWidth="max-w-3xl" <!-- Adjusted for potentially wide content like JSON -->
>
  <div slot="content">
    {#if isLoadingOutputContent}
      <Spinner message="Loading output content..." />
    {:else if viewingOutputContent}
      <pre
        class="whitespace-pre-wrap break-all p-2 text-xs text-gray-200 bg-neutral-900/60
               max-h-[70vh] overflow-y-auto rounded custom-scrollbar"
      >
        {viewingOutputContent}
      </pre>
    {:else}
      <div class="flex justify-center items-center min-h-[200px]">
        <p class="text-gray-400">No content to display or an error occurred.</p>
      </div>
    {/if}
  </div>
  <div slot="footer" class="flex justify-end">
    <Button variant="secondary" on:click={closeOutputViewer}>Close Viewer</Button>
  </div>
</Modal>

<!-- Modal for AI Input/Output Diff View -->
<Modal
  isOpen={showAiDiffModal}
  title="AI Stage Input & Output Comparison"
  on:close={() => showAiDiffModal = false}
  maxWidth="max-w-6xl"
>
  <div slot="content">
    {#if isLoadingAiDiff}
      <Spinner message="Loading AI input/output data..." />
    {:else if aiDiffError && !aiDiffViewHtml}
      <p class="text-error bg-error/30 p-2 rounded-md text-center">Error: {aiDiffError}</p>
    {:else}
      <div class="relative">
          <div class="absolute top-1 right-1 z-20 flex space-x-1">
            {#if aiInputJsonForDiff}
              <Button variant="ghost" size="xs" customClass="!px-1.5 !py-0.5 !text-gray-500 hover:!text-gray-700 dark:!text-gray-400 dark:hover:!text-gray-200" on:click={() => copyToClipboard(aiInputJsonForDiff, 'AI Input JSON')}>Copy Input</Button>
            {/if}
            {#if aiOutputJsonForDiff}
              <Button variant="ghost" size="xs" customClass="!px-1.5 !py-0.5 !text-gray-500 hover:!text-gray-700 dark:!text-gray-400 dark:hover:!text-gray-200" on:click={() => copyToClipboard(aiOutputJsonForDiff, 'AI Output JSON')}>Copy Output</Button>
            {/if}
          </div>
          <h4 class="text-md font-semibold mb-1 text-gray-800 dark:text-gray-200 sticky top-0 bg-neutral-100/95 dark:bg-neutral-800/95 py-2 z-10 border-b border-neutral-300 dark:border-neutral-700">
            Comparison: AI Input <span class="text-xs font-light text-gray-500 dark:text-gray-400">({aiInputStageOutputMetadata?.stage_name})</span> vs. AI Output <span class="text-xs font-light text-gray-500 dark:text-gray-400">({aiOutputStageOutputMetadata?.stage_name})</span>
          </h4>
      </div>
      <pre class="whitespace-pre-wrap text-xs text-gray-700 dark:text-gray-200 bg-white dark:bg-black/40 p-4 rounded custom-scrollbar min-h-[200px] mt-1 max-h-[calc(70vh-50px)] overflow-y-auto"> <!-- Changed p-3 to p-4 -->
        {@html aiDiffViewHtml || '<span class="text-gray-400">No diff data loaded or error.</span>'}
      </pre>
    {/if}
  </div>
  <div slot="footer" class="flex justify-end">
    <Button variant="secondary" on:click={() => showAiDiffModal = false}>Close</Button>
  </div>
</Modal>

<!-- Modal for OCR Text / Parse JSON Compare View -->
<Modal
  isOpen={showOcrJsonCompareModal}
  title="Compare OCR Text with Parse JSON Output"
  on:close={() => showOcrJsonCompareModal = false}
  maxWidth="max-w-6xl"
>
  <div slot="content">
    {#if isLoadingOcrParseCompare}
      <Spinner message="Loading comparison data..." />
    {:else if ocrParseCompareError}
      <p class="text-error bg-error/30 p-2 rounded-md text-center">Error: {ocrParseCompareError}</p>
    {:else}
       <div class="relative">
          <div class="absolute top-1 right-1 z-20 flex space-x-1">
            {#if ocrTextForCompareModal}
              <Button variant="ghost" size="xs" customClass="!px-1.5 !py-0.5 !text-gray-500 hover:!text-gray-700 dark:!text-gray-400 dark:hover:!text-gray-200" on:click={() => copyToClipboard(ocrTextForCompareModal, 'OCR Text')}>Copy OCR</Button>
            {/if}
            {#if parseJsonForCompareModal}
              <Button variant="ghost" size="xs" customClass="!px-1.5 !py-0.5 !text-gray-500 hover:!text-gray-700 dark:!text-gray-400 dark:hover:!text-gray-200" on:click={() => copyToClipboard(parseJsonForCompareModal, 'Parse JSON')}>Copy Parse</Button>
            {/if}
          </div>
          <h4 class="text-md font-semibold mb-1 text-gray-800 dark:text-gray-200 sticky top-0 bg-neutral-100/95 dark:bg-neutral-800/95 py-2 z-10 border-b border-neutral-300 dark:border-neutral-700">
            Comparison: OCR Text <span class="text-xs font-light text-gray-500 dark:text-gray-400">({ocrOutputToDisplay?.stage_name})</span> vs. Parse JSON <span class="text-xs font-light text-gray-500 dark:text-gray-400">({parseOutputToDisplay?.stage_name})</span>
          </h4>
      </div>
      <pre class="whitespace-pre-wrap text-xs text-gray-700 dark:text-gray-200 bg-white dark:bg-black/40 p-4 rounded custom-scrollbar min-h-[200px] mt-1 max-h-[calc(70vh-50px)] overflow-y-auto"> <!-- Changed p-3 to p-4 -->
        {@html ocrParseDiffViewHtml || '<span class="text-gray-400">No diff data loaded or error.</span>'}
      </pre>
    {/if}
  </div>
  <div slot="footer" class="flex justify-end">
    <Button variant="secondary" on:click={() => showOcrJsonCompareModal = false}>Close</Button>
  </div>
</Modal>

<style>
  /* Allow scrollbar to be styled subtly if needed, though defaults are usually fine */
  .max-h-\[calc\(90vh-140px\)\]::-webkit-scrollbar {
    width: 8px;
  }
  .max-h-\[calc\(90vh-140px\)\]::-webkit-scrollbar-thumb {
    background-color: rgba(255,255,255,0.2);
    border-radius: 4px;
  }
  .max-h-\[calc\(90vh-140px\)\]::-webkit-scrollbar-track {
    background-color: rgba(0,0,0,0.1);
  }
</style>
