import { writable } from 'svelte/store';

export interface UiState {
  currentPath: string;
  currentView: string;
  showSettingsPanel: boolean;
  showPipelineEditorPanel: boolean;
  showAdmin: boolean;
  currentViewedJobId: string | null;
}

const initialState: UiState = {
  currentPath: '/dashboard',
  currentView: 'dashboard',
  showSettingsPanel: false,
  showPipelineEditorPanel: false,
  showAdmin: false,
  currentViewedJobId: null
};

function createUiStateStore() {
  const { subscribe, set, update } = writable<UiState>(initialState);
  return {
    subscribe,
    navigate: (path: string) =>
      update(() => {
        const newView = path.substring(1);
        return {
          ...initialState,
          currentPath: path,
          currentView:
            newView === 'dashboard' || newView === 'documents'
              ? newView
              : 'dashboard'
        };
      }),
    toggleSettings: () =>
      update((state) => ({
        ...state,
        showSettingsPanel: !state.showSettingsPanel,
        showPipelineEditorPanel: false,
        showAdmin: false,
        currentViewedJobId: null
      })),
    toggleAdmin: () =>
      update((state) => ({
        ...state,
        showAdmin: !state.showAdmin,
        showSettingsPanel: false,
        showPipelineEditorPanel: false,
        currentViewedJobId: null
      })),
    togglePipelineEditor: () =>
      update((state) => ({
        ...state,
        showPipelineEditorPanel: !state.showPipelineEditorPanel,
        showSettingsPanel: false,
        showAdmin: false,
        currentViewedJobId: null
      })),
    viewJobDetails: (id: string) =>
      update((state) => ({ ...state, currentViewedJobId: id })),
    closeJobDetails: () =>
      update((state) => ({ ...state, currentViewedJobId: null })),
    reset: () => set(initialState)
  };
}

export const uiStateStore = createUiStateStore();
