import { render } from '@testing-library/svelte';
import GlobalLoadingIndicator from '../GlobalLoadingIndicator.svelte';
import { expect, test } from 'vitest';

test('renders progress bar when loading', () => {
  const { getByRole } = render(GlobalLoadingIndicator, { props: { loading: true } });
  expect(getByRole('progressbar')).toBeInTheDocument();
});

