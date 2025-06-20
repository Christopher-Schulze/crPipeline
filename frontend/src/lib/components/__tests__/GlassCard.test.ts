import { render } from '@testing-library/svelte';
import GlassCard from '../GlassCard.svelte';
import { expect, test } from 'vitest';

test('applies custom opacity', () => {
  const { container } = render(GlassCard, { props: { opacity: 0.5 } });
  const div = container.firstElementChild as HTMLElement;
  expect(div.style.getPropertyValue('--glass-opacity')).toBe('0.5');
});
