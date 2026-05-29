<script lang="ts">
  import { Dialog, Button, Input, Label, toast } from "$lib/ui";
  import PasswordStrengthMeter from "./PasswordStrengthMeter.svelte";
  import { FolderOpen } from "@lucide/svelte";
  import { createDatabase, openDatabaseInNewInstance } from "$lib/tauri";
  import { saveLastDatabasePath, addRecentDatabase } from "$lib/storage";
  import { open as openDialog } from "@tauri-apps/plugin-dialog";

  type Props = {
    open?: boolean;
    hasOpenDatabase?: boolean;
    onSuccess: (openedInNewInstance: boolean) => void | Promise<void>;
  };

  let { open = $bindable(false), hasOpenDatabase = false, onSuccess }: Props = $props();

  let folderPath = $state("");
  let fileName = $state("");
  let password = $state("");
  let confirmPassword = $state("");
  let loading = $state(false);

  function reset() {
    folderPath = "";
    fileName = "";
    password = "";
    confirmPassword = "";
  }

  async function handleSelectFolder() {
    try {
      const selected = await openDialog({ directory: true, multiple: false });
      if (selected) folderPath = selected as string;
    } catch (error) {
      toast.error("Error", String(error) || "Failed to select folder");
    }
  }

  async function handleCreate() {
    if (!folderPath || !fileName || !password) {
      toast.error("Missing Information", "Please fill in all fields");
      return;
    }
    if (password !== confirmPassword) {
      toast.error("Password Mismatch", "Passwords do not match");
      return;
    }
    if (password.length < 4) {
      toast.error("Weak Password", "Password must be at least 4 characters");
      return;
    }

    loading = true;
    try {
      const fileNameWithExt = fileName.endsWith(".kdbx") ? fileName : `${fileName}.kdbx`;
      const sep = folderPath.includes("\\") ? "\\" : "/";
      const fullPath = `${folderPath}${sep}${fileNameWithExt}`;

      await createDatabase(fullPath, password);
      addRecentDatabase(fullPath);

      if (hasOpenDatabase) {
        await openDatabaseInNewInstance(fullPath);
        toast.success("Database created", "Opened in new window");
        await onSuccess(true);
      } else {
        saveLastDatabasePath(fullPath);
        toast.success("Database created");
        await onSuccess(false);
      }
      reset();
      open = false;
    } catch (error) {
      toast.error("Failed to Create", String(error) || "Could not create database");
    } finally {
      loading = false;
    }
  }
</script>

<Dialog
  bind:open
  title="Create New Database"
  description="Create a new KeePass database file. Choose a secure master password to protect your data."
  class="sm:max-w-[500px]"
  onOpenChange={(o) => {
    if (!o && !loading) reset();
  }}
>
  <div class="space-y-4">
    <div class="space-y-2">
      <Label for="folder">Save Location</Label>
      <div class="flex gap-2">
        <Input id="folder" bind:value={folderPath} placeholder="Select folder…" readonly class="flex-1" />
        <Button type="button" variant="outline" size="icon" onclick={handleSelectFolder} class="h-9 w-9">
          <FolderOpen class="h-4 w-4" />
        </Button>
      </div>
    </div>
    <div class="space-y-2">
      <Label for="filename">Database Name</Label>
      <Input id="filename" bind:value={fileName} placeholder="MyDatabase" />
      <p class="text-xs text-muted-foreground">.kdbx extension will be added automatically</p>
    </div>
    <div class="space-y-2">
      <Label for="new-password">Master Password</Label>
      <Input id="new-password" type="password" bind:value={password} placeholder="Choose a strong password" />
      <PasswordStrengthMeter {password} />
    </div>
    <div class="space-y-2">
      <Label for="confirm-new-password">Confirm Password</Label>
      <Input
        id="confirm-new-password"
        type="password"
        bind:value={confirmPassword}
        placeholder="Confirm your password"
        onkeydown={(e) => e.key === "Enter" && handleCreate()}
      />
    </div>
  </div>

  {#snippet footer()}
    <Button
      variant="outline"
      onclick={() => {
        if (!loading) {
          reset();
          open = false;
        }
      }}
      disabled={loading}
    >
      Cancel
    </Button>
    <Button
      onclick={handleCreate}
      disabled={loading || !folderPath || !fileName || !password || !confirmPassword}
    >
      {loading ? "Creating…" : "Create Database"}
    </Button>
  {/snippet}
</Dialog>
