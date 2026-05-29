<script lang="ts">
  import { Dialog } from "$lib/ui";
  import { AlertCircle, ArrowDownUp, FileDown, X } from "@lucide/svelte";

  type Props = {
    open?: boolean;
    databasePath: string;
    onSynchronize: () => void | Promise<void>;
    onOverwrite: () => void | Promise<void>;
    onCancel: () => void;
  };

  let {
    open = $bindable(false),
    databasePath,
    onSynchronize,
    onOverwrite,
    onCancel,
  }: Props = $props();
</script>

<Dialog bind:open class="max-w-md" onOpenChange={(o) => !o && onCancel()}>
  <div class="flex items-center gap-2 mb-2">
    <AlertCircle class="h-5 w-5 text-blue-500" />
    <h2 class="text-lg font-semibold tracking-tight">Overwrite the existing file?</h2>
  </div>
  <div class="font-mono text-xs bg-muted p-2 rounded break-all">{databasePath}</div>
  <p class="text-sm text-muted-foreground">
    The file on disk/server has changed since it was loaded. Probably someone else has edited and
    saved the database.
  </p>

  <div class="space-y-2">
    <button
      type="button"
      onclick={() => void onSynchronize()}
      class="w-full flex items-start gap-3 p-3 rounded-md border bg-muted/30 hover:bg-muted/50 transition-colors text-left"
    >
      <ArrowDownUp class="h-5 w-5 text-blue-500 shrink-0 mt-0.5" />
      <div class="flex-1 space-y-1">
        <div class="font-medium text-blue-600">Synchronize</div>
        <div class="text-sm text-muted-foreground">
          Load the file on disk/server and merge it with the current database in memory.
        </div>
      </div>
    </button>
    <button
      type="button"
      onclick={() => void onOverwrite()}
      class="w-full flex items-start gap-3 p-3 rounded-md border bg-muted/30 hover:bg-muted/50 transition-colors text-left"
    >
      <FileDown class="h-5 w-5 text-blue-500 shrink-0 mt-0.5" />
      <div class="flex-1 space-y-1">
        <div class="font-medium text-blue-600">Overwrite</div>
        <div class="text-sm text-muted-foreground">
          Save the current database to the file. Changes made by the other user will be lost.
        </div>
      </div>
    </button>
    <button
      type="button"
      onclick={onCancel}
      class="w-full flex items-start gap-3 p-3 rounded-md border bg-muted/30 hover:bg-muted/50 transition-colors text-left"
    >
      <X class="h-5 w-5 text-blue-500 shrink-0 mt-0.5" />
      <div class="flex-1 space-y-1">
        <div class="font-medium text-blue-600">Cancel</div>
        <div class="text-sm text-muted-foreground">Abort the current operation.</div>
      </div>
    </button>
  </div>
</Dialog>
