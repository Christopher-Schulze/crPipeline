import { render, fireEvent } from '@testing-library/svelte';
import { expect, test, vi } from 'vitest';
import ConfirmationModal from '../ConfirmationModal.svelte';

test('emits confirm and cancel events', async () => {
  const { getByText, component } = render(ConfirmationModal, { props: { isOpen: true } });
  const confirmHandler = vi.fn();
  const cancelHandler = vi.fn();
  component.$on('confirm', confirmHandler);
  component.$on('cancel', cancelHandler);
  await fireEvent.click(getByText('Confirm'));
  await fireEvent.click(getByText('Cancel'));
  expect(confirmHandler).toHaveBeenCalled();
  expect(cancelHandler).toHaveBeenCalled();
});
