import { render } from '@testing-library/svelte';
import { tick } from 'svelte';
import { expect, test } from 'vitest';
import ErrorToast from '../ErrorToast.svelte';
import { errorStore } from '$lib/utils/errorStore';

test('displays messages from errorStore', async () => {
  const { getByText } = render(ErrorToast);
  errorStore.show('Test error', 0);
  await tick();
  expect(getByText('Test error')).toBeInTheDocument();
});
