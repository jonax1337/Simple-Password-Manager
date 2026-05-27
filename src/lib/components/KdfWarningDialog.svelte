<script lang="ts">
  import { Dialog, Button, Checkbox } from "$lib/ui";

  type Props = {
    open?: boolean;
    onSkip: () => void;
    onUpgrade: () => void | Promise<void>;
    kdfType: string;
    databasePath: string;
  };

  let { open = $bindable(false), onSkip, onUpgrade, kdfType, databasePath }: Props = $props();
  let dontShowAgain = $state(false);

  function dismissForDatabase() {
    if (!dontShowAgain) return;
    const dismissed = JSON.parse(localStorage.getItem("kdf_warning_dismissed_dbs") || "[]");
    if (!dismissed.includes(databasePath)) {
      dismissed.push(databasePath);
      localStorage.setItem("kdf_warning_dismissed_dbs", JSON.stringify(dismissed));
    }
  }

  async function handleUpgrade() {
    dismissForDatabase();
    await onUpgrade();
  }

  function handleSkip() {
    dismissForDatabase();
    onSkip();
  }
</script>

<Dialog
  bind:open
  title="Weak Key Transformation Settings"
  description="The key transformation settings of the database are weak."
  onOpenChange={(o) => !o && handleSkip()}
>
  <div class="text-xs bg-muted p-3 rounded">
    <div><strong>Current:</strong> {kdfType}</div>
    <div><strong>Recommended:</strong> Argon2id (2 iterations, 64 MB, 2 threads)</div>
  </div>
  <p class="text-sm text-muted-foreground">
    Do you want to set them to the current default values (recommended)?
  </p>
  <label class="flex items-center gap-2 text-sm text-muted-foreground cursor-pointer select-none">
    <Checkbox bind:checked={dontShowAgain} />
    Do not show this dialog again for this database; always 'No'.
  </label>

  {#snippet footer()}
    <Button variant="outline" onclick={handleSkip}>No</Button>
    <Button onclick={handleUpgrade}>Yes</Button>
  {/snippet}
</Dialog>
