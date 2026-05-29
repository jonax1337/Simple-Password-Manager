<script lang="ts">
  import { Dialog, Button } from "$lib/ui";

  type Props = {
    open?: boolean;
    onCancel: () => void;
    onDontSave: () => void | Promise<void>;
    onSave: () => void | Promise<void>;
  };

  let { open = $bindable(false), onCancel, onDontSave, onSave }: Props = $props();
</script>

<Dialog
  bind:open
  title="Unsaved Changes"
  description="You have unsaved changes. What would you like to do?"
  onOpenChange={(o) => !o && onCancel()}
>
  {#snippet footer()}
    <Button variant="outline" onclick={onCancel}>Cancel</Button>
    <Button variant="destructive" onclick={() => void onDontSave()}>Don't Save</Button>
    <Button onclick={() => void onSave()}>Save</Button>
  {/snippet}
</Dialog>
