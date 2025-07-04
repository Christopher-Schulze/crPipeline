import { render, fireEvent } from '@testing-library/svelte';
import { expect, test, vi } from 'vitest';
import SlideOver from '../SlideOver.svelte';

test('emits close when backdrop clicked', async () => {
  const { container, component } = render(SlideOver, { props: { isOpen: true } });
  const handler = vi.fn();
  component.$on('close', handler);
  const overlay = container.firstElementChild as HTMLElement;
  await fireEvent.click(overlay);
  expect(handler).toHaveBeenCalled();
});
