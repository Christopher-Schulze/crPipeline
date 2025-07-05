import { readable } from 'svelte/store';
export const page = readable({ data: { session: { userId: 'test' } } });
export const navigating = readable(null);
