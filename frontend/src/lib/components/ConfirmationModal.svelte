<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import Modal from './Modal.svelte'; // The generic modal component
  import Button from './Button.svelte';

  export let isOpen: boolean = false;
  export let title: string = 'Confirm Action';
  export let message: string = 'Are you sure you want to proceed?';
  export let confirmText: string = 'Confirm';
  export let cancelText: string = 'Cancel';
  export let confirmButtonVariant: 'primary' | 'secondary' | 'ghost' | 'danger' = 'primary';
  export let cancelButtonVariant: 'primary' | 'secondary' | 'ghost' = 'secondary';

  const dispatch = createEventDispatcher();

  function handleConfirm() {
    dispatch('confirm');
  }

  function handleCancel() {
    // This component dispatches 'cancel'. The parent <Modal> listens for 'close'.
    // So, when this modal's own "Cancel" button or the generic Modal's X/Esc/backdrop is used,
    // we want to ensure the intention of cancelling this specific confirmation is clear.
    // The generic Modal will dispatch 'close', which should be handled by the parent component
    // (SettingsForm in this case) to set this ConfirmationModal's isOpen to false.
    dispatch('cancel');
  }

  // This function is for when the underlying generic Modal dispatches 'close'
  // (e.g. user clicks backdrop or presses Esc). We treat it as a cancel.
  function handleGenericModalClose() {
    dispatch('cancel');
  }

</script>

<Modal isOpen={isOpen} title={title} on:close={handleGenericModalClose} maxWidth="max-w-md">
  <div slot="content">
    <p class="text-sm text-gray-300 whitespace-pre-line">{message}</p>
  </div>
  <div slot="footer" class="flex justify-end space-x-3">
    <Button variant={cancelButtonVariant} on:click={handleCancel} customClass="!py-1.5 !px-3 text-sm">
      {cancelText}
    </Button>
    <Button
      variant={confirmButtonVariant === 'danger' ? 'primary' : confirmButtonVariant}
      on:click={handleConfirm}
      customClass="{confirmButtonVariant === 'danger' ? '!bg-red-600 hover:!bg-red-700 !border-red-600 hover:!border-red-700 text-white' : ''} !py-1.5 !px-3 text-sm"
    >
      {confirmText}
    </Button>
  </div>
</Modal>
