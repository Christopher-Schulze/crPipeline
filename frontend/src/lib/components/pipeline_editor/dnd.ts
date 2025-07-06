export interface DragState {
  draggedItemId: string | null;
  draggedOverIndex: number | null;
  draggingVisualIndex: number | null;
}

export function createDragHandlers<T>(getItems: () => T[], setItems: (items: T[]) => void) {
  const state: DragState = {
    draggedItemId: null,
    draggedOverIndex: null,
    draggingVisualIndex: null,
  };

  function handleDragStart(event: DragEvent, id: string, index: number) {
    state.draggedItemId = id;
    state.draggingVisualIndex = index;
    event.dataTransfer?.setData('text/plain', id);
    if (event.dataTransfer) {
      event.dataTransfer.effectAllowed = 'move';
    }
  }

  function handleDragOver(event: DragEvent, targetIndex: number) {
    event.preventDefault();
    if (state.draggedItemId === null) return;
    const targetItemId = (getItems()[targetIndex] as any)?.id;
    state.draggedOverIndex = state.draggedItemId !== targetItemId ? targetIndex : null;
    if (event.dataTransfer) {
      event.dataTransfer.dropEffect = 'move';
    }
  }

  function handleDragEnter(event: DragEvent, targetIndex: number) {
    event.preventDefault();
    if (state.draggedItemId === null) return;
    const targetItemId = (getItems()[targetIndex] as any)?.id;
    if (state.draggedItemId !== targetItemId) {
      state.draggedOverIndex = targetIndex;
    }
  }

  function handleDragLeave(event: DragEvent) {
    const currentTarget = event.currentTarget as HTMLElement;
    const relatedTarget = event.relatedTarget as HTMLElement | null;
    if (!relatedTarget || !currentTarget.contains(relatedTarget)) {
      /* nothing */
    }
  }

  function handleDrop(event: DragEvent, targetIndex: number) {
    event.preventDefault();
    if (!state.draggedItemId) return;
    const items = [...getItems()];
    const draggedItemOriginalIndex = items.findIndex((s: any) => s.id === state.draggedItemId);
    if (draggedItemOriginalIndex === -1) return;
    const [draggedItem] = items.splice(draggedItemOriginalIndex, 1);
    items.splice(targetIndex, 0, draggedItem);
    setItems(items);
    state.draggedItemId = null;
    state.draggingVisualIndex = null;
    state.draggedOverIndex = null;
  }

  function handleDragEnd() {
    state.draggedItemId = null;
    state.draggingVisualIndex = null;
    state.draggedOverIndex = null;
  }

  return {
    state,
    handleDragStart,
    handleDragOver,
    handleDragEnter,
    handleDragLeave,
    handleDrop,
    handleDragEnd,
  };
}
