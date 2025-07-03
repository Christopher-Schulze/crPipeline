import { render } from '@testing-library/svelte';
import { expect, test, vi } from 'vitest';
import JobsList from '../JobsList.svelte';

const now = new Date().toISOString();

test('shows provided document and pipeline names', () => {
  vi.stubGlobal('EventSource', class { close() {} } as any);
  const jobs = [
    {
      id: 'j1',
      org_id: 'o1',
      document_id: 'd1',
      pipeline_id: 'p1',
      status: 'pending',
      created_at: now,
      document_name: 'Doc.pdf',
      pipeline_name: 'MyPipe',
    },
  ];
  const { getByText } = render(JobsList, { props: { jobs } });
  expect(getByText('Doc.pdf')).toBeInTheDocument();
  expect(getByText('MyPipe')).toBeInTheDocument();
});
