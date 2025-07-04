import { render } from '@testing-library/svelte';
import { expect, test } from 'vitest';
import DataTable from '../DataTable.svelte';

const headers = [{ key: 'name', label: 'Name', sortable: true }];
const items = [{ id: '1', name: 'Item 1' }];

test('renders table headers', () => {
  const { getByText } = render(DataTable, { props: { headers, items, currentPage:1, totalPages:1 } });
  expect(getByText('Name')).toBeInTheDocument();
});
