import { render, fireEvent } from '@testing-library/svelte';
import { expect, test, vi } from 'vitest';
import PaginationControls from '../PaginationControls.svelte';

test('emits pageChange when next clicked', async () => {
  const { getByText, component } = render(PaginationControls, { props: { currentPage: 1, totalPages: 3 } });
  const handler = vi.fn();
  component.$on('pageChange', handler);
  await fireEvent.click(getByText('Next'));
  expect(handler).toHaveBeenCalledWith(expect.objectContaining({ detail: { page: 2 } }));
});
