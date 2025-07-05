import { render } from '@testing-library/svelte';
import GlobalLoadingIndicator from '../GlobalLoadingIndicator.svelte';
import { loadingStore } from '$lib/utils/loadingStore';
import { tick } from 'svelte';
import { expect, test } from 'vitest';

test('reacts to loadingStore changes', async () => {
  const { queryByRole } = render(GlobalLoadingIndicator);
  expect(queryByRole('progressbar')).not.toBeInTheDocument();

  loadingStore.start();
  await tick();

  expect(queryByRole('progressbar')).toBeInTheDocument();

  loadingStore.end();
  await tick();

  expect(queryByRole('progressbar')).not.toBeInTheDocument();

  loadingStore.reset();
});

