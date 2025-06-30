import { render } from '@testing-library/svelte';
import GlassCard from '../GlassCard.svelte';
import { expect, test } from 'vitest';

test('applies custom bg opacity class', () => {
  const { container } = render(GlassCard, { props: { bgOpacity: 'bg-white/80' } });
  const div = container.firstElementChild as HTMLElement;
  expect(div.classList.contains('bg-white/80')).toBe(true);
});
