import { render, fireEvent } from '@testing-library/svelte';
import { expect, test } from 'vitest';
import RegexPatternEditor from '../pipeline_editor/RegexPatternEditor.svelte';

test('adds pattern via button', async () => {
  const { getByText, getByPlaceholderText } = render(RegexPatternEditor, { props: { patterns: [] } });
  await fireEvent.click(getByText('Add Regex Pattern'));
  expect(getByPlaceholderText('Field Name (e.g., InvoiceID)')).toBeInTheDocument();
});
