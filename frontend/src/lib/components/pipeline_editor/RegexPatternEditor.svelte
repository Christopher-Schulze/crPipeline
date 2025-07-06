<script lang="ts">
  import Button from '../Button.svelte';
  import type { RegexPatternConfig } from './types';
  export let patterns: RegexPatternConfig[] = [];

  function addPattern() {
    patterns = [
      ...patterns,
      {
        id: Date.now().toString() + Math.random().toString(36).substring(2, 9),
        name: '',
        regex: '',
        captureGroupIndex: 1,
      },
    ];
  }

  function removePattern(id: string) {
    patterns = patterns.filter((p) => p.id !== id);
  }
</script>

<div class="space-y-2 py-2 text-xs">
  <label class="block font-medium text-gray-300 mb-1">Regex Patterns:</label>
  {#each patterns as pattern, k (pattern.id)}
    <div class="p-2 bg-black/20 rounded space-y-1.5 mb-1.5">
      <div class="flex items-center space-x-2">
        <input
          type="text"
          bind:value={pattern.name}
          class="glass-input flex-grow !text-xs !bg-neutral-500/40"
          placeholder="Field Name (e.g., InvoiceID)"
        />
        <Button
          variant="ghost"
          customClass="!px-1.5 !py-0.5 !text-error hover:!text-error-content"
          on:click={() => removePattern(pattern.id)}
          >X</Button
        >
      </div>
      <input
        type="text"
        bind:value={pattern.regex}
        class="glass-input w-full !text-xs !bg-neutral-500/40"
        placeholder="Regex Pattern (e.g., INV-\d+)"
      />
      <div>
        <label
          for={`pattern-group-index-${pattern.id}`}
          class="block text-xs font-medium text-gray-400 mb-0.5 mt-1"
          >Capture Group Index</label
        >
        <input
          type="number"
          id={`pattern-group-index-${pattern.id}`}
          bind:value={pattern.captureGroupIndex}
          min="0"
          step="1"
          class="glass-input w-full !text-xs !bg-neutral-500/40"
          placeholder="e.g., 1 (0 for full match)"
        />
        <p class="text-sm font-light text-gray-500 dark:text-gray-400 mt-0.5">
          Default: 1 (first group). Use 0 for full match.
        </p>
      </div>
    </div>
  {/each}
  <Button variant="secondary" customClass="!text-xs !py-1" on:click={addPattern}
    >Add Regex Pattern</Button
  >
</div>

