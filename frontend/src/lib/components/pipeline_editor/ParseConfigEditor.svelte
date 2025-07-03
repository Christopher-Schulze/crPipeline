<script lang="ts">
  import Button from '../Button.svelte';
  import RegexPatternEditor from './RegexPatternEditor.svelte';
  import type { Stage } from './types';

  export let stage: Stage;
  export let initializeParseStrategyParameters: (stage: Stage) => void;
</script>

<div class="form-group mt-3 pt-3 border-t border-neutral-700/50 space-y-3">
  <div>
    <label for={`stage-parse-strategy-${stage.id}`} class="block text-xs font-medium text-gray-300 mb-1">
      Parsing Strategy
    </label>
    <select
      id={`stage-parse-strategy-${stage.id}`}
      bind:value={stage.config.strategy}
      on:change={() => initializeParseStrategyParameters(stage)}
      class="glass-input w-full !text-sm !bg-neutral-600/50 !border-neutral-500/70 !text-gray-100"
    >
      <option value="Passthrough">Passthrough (Basic Lines/Text)</option>
      <option value="KeywordExtraction">Keyword Extraction</option>
      <option value="RegexExtraction">Regex Extraction</option>
      <option value="SimpleTableExtraction">Simple Table Extraction</option>
    </select>
  </div>

  {#if stage.config?.strategy === 'KeywordExtraction'}
    <div class="pl-3 border-l-2 border-neutral-700 space-y-2 py-2 text-xs">
      <label class="block font-medium text-gray-300 mb-1">Keywords:</label>
      {#each stage.config.parameters.keywords as keyword, k (k)}
        <div class="flex items-center space-x-2">
          <input
            type="text"
            bind:value={stage.config.parameters.keywords[k]}
            class="glass-input flex-grow !text-xs !bg-neutral-500/40"
            placeholder="Enter keyword"
          />
          <Button
            variant="ghost"
            customClass="!px-1.5 !py-0.5 !text-red-400 hover:!text-red-300"
            on:click={() => (stage.config.parameters.keywords = stage.config.parameters.keywords.filter((_, idx) => idx !== k))}
          >
            X
          </Button>
        </div>
      {/each}
      <Button
        variant="secondary"
        customClass="!text-xs !py-1"
        on:click={() => (stage.config.parameters.keywords = [...stage.config.parameters.keywords, ''])}
      >
        Add Keyword
      </Button>
      <label class="flex items-center space-x-2 mt-2 cursor-pointer">
        <input
          type="checkbox"
          bind:checked={stage.config.parameters.caseSensitive}
          class="form-checkbox h-4 w-4 text-accent rounded !bg-neutral-700 border-neutral-600 focus:ring-accent/50"
        />
        <span class="text-gray-300">Case Sensitive</span>
      </label>
    </div>
  {/if}

  {#if stage.config?.strategy === 'RegexExtraction'}
    <div class="pl-3 border-l-2 border-neutral-700">
      <RegexPatternEditor bind:patterns={stage.config.parameters.patterns} />
    </div>
  {/if}

  {#if stage.config?.strategy === 'SimpleTableExtraction'}
    <div class="pl-3 border-l-2 border-neutral-700 space-y-2 py-2 text-xs">
      <label class="block font-medium text-gray-300 mb-0.5">Header Keywords (comma-separated):</label>
      <input
        type="text"
        bind:value={stage.config.parameters._headerKeywordsString}
        on:input={() =>
          (stage.config.parameters.headerKeywords = (stage.config.parameters._headerKeywordsString || '')
            .split(',')
            .map((s) => s.trim())
            .filter((s) => s))}
        class="glass-input w-full !text-xs !bg-neutral-500/40"
        placeholder="e.g., Item, Qty, Price"
      />
      <label class="block font-medium text-gray-300 mt-1 mb-0.5">Stop Keywords (optional, comma-separated):</label>
      <input
        type="text"
        bind:value={stage.config.parameters._stopKeywordsString}
        on:input={() =>
          (stage.config.parameters.stopKeywords = (stage.config.parameters._stopKeywordsString || '')
            .split(',')
            .map((s) => s.trim())
            .filter((s) => s))}
        class="glass-input w-full !text-xs !bg-neutral-500/40"
        placeholder="e.g., Total, Subtotal"
      />
      <label class="block font-medium text-gray-300 mt-1 mb-0.5">Column Separator Regex:</label>
      <input
        type="text"
        bind:value={stage.config.parameters._delimiterRegex}
        on:input={() => (stage.config.parameters.delimiterRegex = stage.config.parameters._delimiterRegex)}
        class="glass-input w-full !text-xs !bg-neutral-500/40"
        placeholder="optional, defaults to whitespace or '|'"
      />
    </div>
  {/if}
</div>

