import { render } from '@testing-library/svelte';
import { expect, test } from 'vitest';
import StageList from '../pipeline_editor/StageList.svelte';

const stages = [{ id: '1', type: 'ocr', ocr_engine: 'external' }];

test('shows tooltips for OCR endpoint and key', () => {
  const { getByTitle } = render(StageList, { props: { stages, availablePromptTemplates: [] } });
  expect(getByTitle('Endpoint for external OCR service')).toBeTruthy();
  expect(getByTitle('API key for the external OCR service')).toBeTruthy();
});
