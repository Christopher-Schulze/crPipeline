import { writable } from 'svelte/store';

export interface SessionData {
  loggedIn: boolean;
  userId: string | null;
  org: string | null;
  role: string | null;
  csrfToken: string | null;
}

const initial: SessionData = {
  loggedIn: false,
  userId: null,
  org: null,
  role: null,
  csrfToken: null
};

function createSessionStore() {
  const { subscribe, set } = writable<SessionData>(initial);
  return {
    subscribe,
    setSession: (data: SessionData) => set(data),
    clear: () => set(initial)
  };
}

export const sessionStore = createSessionStore();
