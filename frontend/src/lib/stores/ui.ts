import { writable } from 'svelte/store';

export interface UIState {
  currentPath: string;
  currentView: string;
  showSettingsPanel: boolean;
  showAdmin: boolean;
  showPipelineEditorPanel: boolean;
  currentViewedJobId: string | null;
}

const initial: UIState = {
  currentPath: '/dashboard',
  currentView: 'dashboard',
  showSettingsPanel: false,
  showAdmin: false,
  showPipelineEditorPanel: false,
  currentViewedJobId: null
};

function createUIStore() {
  const { subscribe, update, set } = writable<UIState>(initial);

  return {
    subscribe,
    setCurrentPath: (path: string) =>
      update(s => ({
        ...s,
        currentPath: path,
        currentView: path.substring(1) || 'dashboard'
      })),
    toggleSettings: () =>
      update(s => ({
        ...s,
        showSettingsPanel: !s.showSettingsPanel,
        showPipelineEditorPanel: false,
        showAdmin: false,
        currentViewedJobId: null
      })),
    toggleAdmin: () =>
      update(s => ({
        ...s,
        showAdmin: !s.showAdmin,
        showSettingsPanel: false,
        showPipelineEditorPanel: false,
        currentViewedJobId: null
      })),
    togglePipelineEditor: () =>
      update(s => ({
        ...s,
        showPipelineEditorPanel: !s.showPipelineEditorPanel,
        showSettingsPanel: false,
        showAdmin: false,
        currentViewedJobId: null
      })),
    viewJob: (id: string) => update(s => ({ ...s, currentViewedJobId: id })),
    closeJob: () => update(s => ({ ...s, currentViewedJobId: null })),
    reset: () => set(initial)
  };
}

export const uiStore = createUIStore();
