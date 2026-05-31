<script lang="ts">
  import {
    Database,
    ChevronsUpDown,
    Check,
    Loader2,
    FileLock2,
    Plus,
  } from "@lucide/svelte";
  import {
    cloudListVaults,
    cloudOpenVault,
    cloudCreateVault,
    cloudPush,
    cloudStatus,
    type CloudVaultEntry,
  } from "$lib/tauri";
  import { Dialog, DropdownMenu, DropdownItem, DropdownSeparator, Button, Input, Label, toast } from "$lib/ui";
  import { appState } from "$lib/app-state.svelte";

  type Props = {
    /** Active vault label rendered inside the header. */
    label: string;
    /** Sync-pill content rendered below the label (delegated by parent). */
    pillSnippet: import("svelte").Snippet;
    /** Called after a successful switch so the parent can reload groups + entries. */
    onSwitched: () => void | Promise<void>;
  };

  let { label, pillSnippet, onSwitched }: Props = $props();

  // ----- vault list -----
  let vaults = $state<CloudVaultEntry[]>([]);
  let listLoading = $state(false);
  let listError = $state("");

  async function loadList() {
    listLoading = true;
    listError = "";
    try {
      const status = await cloudStatus();
      if (!status.linked) {
        vaults = [];
        return;
      }
      vaults = await cloudListVaults();
    } catch (e) {
      listError = String(e);
    } finally {
      listLoading = false;
    }
  }

  // ----- switching flow -----
  let pendingSwitch = $state<{ id: string; name: string } | null>(null);
  let dirtyPromptOpen = $state(false);
  let kdbxDialogOpen = $state(false);
  let kdbxPassword = $state("");
  let switching = $state(false);
  let switchError = $state("");

  function beginSwitch(v: CloudVaultEntry) {
    // Same vault → no-op.
    if (v.id === appState.cloudVaultId) return;
    pendingSwitch = { id: v.id, name: v.name };
    if (appState.isDirty) {
      // Unsaved local edits — let user decide.
      dirtyPromptOpen = true;
    } else {
      askKdbxPassword();
    }
  }

  function askKdbxPassword() {
    kdbxPassword = "";
    switchError = "";
    kdbxDialogOpen = true;
  }

  async function pushThenSwitch() {
    if (!pendingSwitch) return;
    switching = true;
    switchError = "";
    try {
      // Synchronously flush before switching vaults — markClean so the
      // dirty pill doesn't bleed into the new vault's state.
      appState.setSyncStatus("cloud-sync");
      await cloudPush();
      appState.markClean();
      appState.markSynced();
      dirtyPromptOpen = false;
      askKdbxPassword();
    } catch (e) {
      switchError = `Push failed: ${e}`;
    } finally {
      switching = false;
    }
  }

  function discardAndSwitch() {
    appState.markClean(); // discard locally — the cloud copy stays as-is
    dirtyPromptOpen = false;
    askKdbxPassword();
  }

  async function confirmKdbxUnlock() {
    if (!pendingSwitch || !kdbxPassword) return;
    switching = true;
    switchError = "";
    try {
      await cloudOpenVault(pendingSwitch.id, kdbxPassword);
      const name = pendingSwitch.name;
      appState.setCloudVaultName(name);
      appState.setCloudVaultId(pendingSwitch.id);
      appState.setDbPath("");
      appState.markClean();
      kdbxDialogOpen = false;
      pendingSwitch = null;
      kdbxPassword = "";
      toast.success("Switched", `Now viewing "${name}"`);
      await onSwitched();
    } catch (e) {
      switchError = String(e);
    } finally {
      switching = false;
    }
  }

  function cancelSwitch() {
    pendingSwitch = null;
    kdbxPassword = "";
    switchError = "";
    dirtyPromptOpen = false;
    kdbxDialogOpen = false;
  }

  // ----- create flow -----
  let createDialogOpen = $state(false);
  let createName = $state("");
  let createPassword = $state("");
  let createBusy = $state(false);
  let createError = $state("");

  function openCreateDialog() {
    createName = "";
    createPassword = "";
    createError = "";
    createDialogOpen = true;
  }

  async function confirmCreate() {
    if (!createName.trim() || !createPassword) return;
    createBusy = true;
    createError = "";
    try {
      const resp = await cloudCreateVault(createName.trim(), createPassword);
      appState.setCloudVaultName(createName.trim());
      appState.setCloudVaultId(resp.id);
      appState.setDbPath("");
      appState.markClean();
      createDialogOpen = false;
      const created = createName.trim();
      createName = "";
      createPassword = "";
      toast.success("Vault created", `"${created}" is now your active vault`);
      await onSwitched();
    } catch (e) {
      createError = String(e);
    } finally {
      createBusy = false;
    }
  }
</script>

<DropdownMenu align="start" side="bottom" class="w-[260px]">
  {#snippet trigger()}
    <button
      type="button"
      onclick={() => void loadList()}
      class="group/vault w-full flex items-center gap-2.5 rounded-lg bg-background/70 hover:bg-background border border-border px-2.5 py-2 transition-colors text-left"
      title="Switch vault"
    >
      <div class="grid place-items-center size-8 rounded-md bg-primary/15 text-primary shrink-0">
        <Database class="size-4" />
      </div>
      <div class="min-w-0 flex-1">
        <div class="text-sm font-semibold truncate leading-tight">{label}</div>
        {@render pillSnippet()}
      </div>
      <ChevronsUpDown class="size-3.5 text-muted-foreground shrink-0 opacity-60 group-hover/vault:opacity-100 transition-opacity" />
    </button>
  {/snippet}

  <div class="px-2 py-1.5 text-2xs uppercase tracking-wider text-muted-foreground">
    Cloud vaults
  </div>

  {#if listLoading}
    <div class="px-2 py-2 text-sm text-muted-foreground flex items-center gap-2">
      <Loader2 class="size-3.5 animate-spin" />
      Loading…
    </div>
  {:else if listError}
    <div class="px-2 py-2 text-xs text-destructive">{listError}</div>
  {:else if vaults.length === 0}
    <div class="px-2 py-2 text-sm text-muted-foreground">
      Not linked to a cloud account.
    </div>
  {:else}
    {#each vaults as v (v.id)}
      {@const active = v.id === appState.cloudVaultId}
      <DropdownItem onSelect={() => beginSwitch(v)} disabled={active}>
        {#if active}
          <Check class="text-primary" />
        {:else}
          <Database class="text-muted-foreground" />
        {/if}
        <span class="flex-1 truncate">{v.name}</span>
        <span class="text-2xs uppercase tracking-wider text-muted-foreground">{v.role}</span>
      </DropdownItem>
    {/each}
  {/if}

  <DropdownSeparator />
  <DropdownItem onSelect={openCreateDialog}>
    <Plus />
    Create new vault…
  </DropdownItem>
  <DropdownItem onSelect={() => void loadList()}>
    <Loader2 class={listLoading ? "animate-spin" : ""} />
    Refresh list
  </DropdownItem>
</DropdownMenu>

<!-- Confirm dialog when there are unsaved edits in the current vault. -->
<Dialog
  bind:open={dirtyPromptOpen}
  title={pendingSwitch ? `Switch to "${pendingSwitch.name}"?` : "Switch vault"}
  description="You have unsaved changes in the current vault."
  size="sm"
  onOpenChange={(o) => {
    if (!o && !switching) cancelSwitch();
  }}
>
  <p class="text-sm text-muted-foreground">
    Push your changes to the cloud first, or discard them and switch immediately.
  </p>
  {#if switchError}
    <p class="text-xs text-destructive break-all">{switchError}</p>
  {/if}
  {#snippet footer()}
    <Button variant="outline" onclick={cancelSwitch} disabled={switching}>Cancel</Button>
    <Button variant="outline" onclick={discardAndSwitch} disabled={switching}>Discard &amp; switch</Button>
    <Button onclick={pushThenSwitch} disabled={switching}>
      {#if switching}<Loader2 class="size-4 animate-spin" />Pushing…{:else}Push &amp; switch{/if}
    </Button>
  {/snippet}
</Dialog>

<!-- Create-new-vault dialog. The KDBX is generated server-side via the
     command — no local file is created. -->
<Dialog
  bind:open={createDialogOpen}
  title="Create new vault"
  description="The vault is generated in memory, sealed locally, and pushed to your cloud account. No file lands on disk."
  size="sm"
  onOpenChange={(o) => {
    if (!o && !createBusy) {
      createName = "";
      createPassword = "";
      createError = "";
    }
  }}
>
  <div class="space-y-3">
    <div class="space-y-1.5">
      <Label for="create-vault-name">Vault name</Label>
      <Input id="create-vault-name" bind:value={createName} placeholder="Work" autofocus />
    </div>
    <div class="space-y-1.5">
      <Label for="create-vault-pw">KeePass master password</Label>
      <Input
        id="create-vault-pw"
        type="password"
        bind:value={createPassword}
        placeholder="••••••••"
        onkeydown={(e) => e.key === "Enter" && void confirmCreate()}
      />
      <p class="text-2xs text-muted-foreground">
        Each vault has its own master password. Pick something you'll remember —
        the cloud only sees the encrypted blob.
      </p>
    </div>
    {#if createError}
      <p class="text-xs text-destructive break-all">{createError}</p>
    {/if}
  </div>
  {#snippet footer()}
    <Button
      variant="outline"
      onclick={() => (createDialogOpen = false)}
      disabled={createBusy}
    >
      Cancel
    </Button>
    <Button
      onclick={confirmCreate}
      disabled={createBusy || !createName.trim() || !createPassword}
    >
      {#if createBusy}<Loader2 class="size-4 animate-spin" />Creating…{:else}<Plus class="size-4" />Create vault{/if}
    </Button>
  {/snippet}
</Dialog>

<!-- KDBX-password modal for the target vault. -->
<Dialog
  bind:open={kdbxDialogOpen}
  title={pendingSwitch ? `Unlock "${pendingSwitch.name}"` : "Unlock vault"}
  description="Each vault has its own KeePass master password — separate from the cloud account."
  size="sm"
  onOpenChange={(o) => {
    if (!o && !switching) cancelSwitch();
  }}
>
  <div class="space-y-3">
    <div class="space-y-1.5">
      <Label for="switch-kdbx-pw">KeePass master password</Label>
      <Input
        id="switch-kdbx-pw"
        type="password"
        bind:value={kdbxPassword}
        autofocus
        onkeydown={(e) => e.key === "Enter" && void confirmKdbxUnlock()}
        placeholder="••••••••"
      />
    </div>
    {#if switchError}
      <p class="text-xs text-destructive break-all">{switchError}</p>
    {/if}
  </div>
  {#snippet footer()}
    <Button variant="outline" onclick={cancelSwitch} disabled={switching}>Cancel</Button>
    <Button onclick={confirmKdbxUnlock} disabled={switching || !kdbxPassword}>
      {#if switching}<Loader2 class="size-4 animate-spin" />Switching…{:else}<FileLock2 class="size-4" />Switch{/if}
    </Button>
  {/snippet}
</Dialog>
