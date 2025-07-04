import { render, fireEvent } from '@testing-library/svelte';
import { expect, test, vi } from 'vitest';
import Modal from '../Modal.svelte';

test('emits close when backdrop clicked', async () => {
  const { container, component } = render(Modal, { props: { isOpen: true } });
  const handler = vi.fn();
  component.$on('close', handler);
  const backdrop = container.firstElementChild as HTMLElement;
  await fireEvent.click(backdrop);
  expect(handler).toHaveBeenCalled();
});
