<script lang="ts">
  import {
    cloudListMembers,
    cloudShareVault,
    cloudUnshareVault,
    cloudUpdateMemberRole,
    type CloudMemberRow,
  } from "$lib/tauri";
  import { Dialog, Button, Input, Label, Select, toast } from "$lib/ui";
  import { Loader2, UserPlus, Users, Trash2, Shield, KeyRound, Eye } from "@lucide/svelte";

  type Props = {
    open?: boolean;
    /** The vault to manage. Pass `null` when nothing's active — the dialog
     * renders an inert state rather than throwing. */
    vaultId: string | null;
    vaultName: string;
    /** The caller's role in this vault. Drives whether role-change /
     * invite / revoke controls are even rendered. */
    callerRole: "owner" | "editor" | "reader" | null;
    /** The caller's own user_id, so we can hide destructive controls on
     * the caller's own row. */
    callerUserId: string | null;
  };

  let {
    open = $bindable(false),
    vaultId,
    vaultName,
    callerRole,
    callerUserId,
  }: Props = $props();

  let members = $state<CloudMemberRow[]>([]);
  let loading = $state(false);
  let error = $state("");

  // Invite form (owner-only).
  let inviteEmail = $state("");
  let inviteRole = $state<"editor" | "reader">("reader");
  let inviting = $state(false);
  let inviteError = $state("");

  const isOwner = $derived(callerRole === "owner");

  $effect(() => {
    if (open && vaultId) {
      void loadMembers();
    }
    if (!open) {
      // Drop transient form state on close so the next open is clean.
      inviteEmail = "";
      inviteError = "";
    }
  });

  async function loadMembers() {
    if (!vaultId) return;
    loading = true;
    error = "";
    try {
      members = await cloudListMembers(vaultId);
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  async function handleInvite() {
    if (!vaultId || !inviteEmail.trim()) return;
    inviting = true;
    inviteError = "";
    try {
      await cloudShareVault(vaultId, inviteEmail.trim(), inviteRole);
      inviteEmail = "";
      await loadMembers();
      toast.success("Invited", `${inviteEmail.trim()} is now a ${inviteRole}`);
    } catch (e) {
      inviteError = String(e);
    } finally {
      inviting = false;
    }
  }

  async function handleRoleChange(m: CloudMemberRow, next: string) {
    if (!vaultId) return;
    if (next === m.role) return;
    try {
      await cloudUpdateMemberRole(vaultId, m.user_id, next as "owner" | "editor" | "reader");
      await loadMembers();
      toast.success("Role updated", `${m.email} is now a ${next}`);
    } catch (e) {
      toast.error("Role change failed", String(e));
      await loadMembers(); // resync UI back to truth
    }
  }

  async function handleRevoke(m: CloudMemberRow) {
    if (!vaultId) return;
    const ok = window.confirm(
      `Remove ${m.email} from "${vaultName}"? They'll lose access immediately.`,
    );
    if (!ok) return;
    try {
      await cloudUnshareVault(vaultId, m.user_id);
      await loadMembers();
      toast.success("Removed", `${m.email} no longer has access`);
    } catch (e) {
      toast.error("Remove failed", String(e));
    }
  }

  function roleIcon(role: string) {
    if (role === "owner") return Shield;
    if (role === "editor") return KeyRound;
    return Eye;
  }

  function formatDate(seconds: number) {
    return new Date(seconds * 1000).toLocaleDateString();
  }

  const roleItems = [
    { value: "owner", label: "Owner" },
    { value: "editor", label: "Editor" },
    { value: "reader", label: "Reader" },
  ];
  const inviteRoleItems = [
    { value: "editor", label: "Editor" },
    { value: "reader", label: "Reader" },
  ];
</script>

<Dialog
  bind:open
  title={vaultId ? `Members of "${vaultName}"` : "Members"}
  description="Owners can invite, change roles, and remove members. Editors can read and write. Readers can only view."
  size="md"
>
  {#if !vaultId}
    <p class="text-sm text-muted-foreground">No cloud vault is active.</p>
  {:else}
    <!-- Member list -->
    <div class="rounded-md border border-border overflow-hidden">
      {#if loading}
        <div class="p-6 text-sm text-muted-foreground flex items-center gap-2 justify-center">
          <Loader2 class="size-4 animate-spin" />
          Loading members…
        </div>
      {:else if error}
        <div class="p-4 text-xs text-destructive break-all">{error}</div>
      {:else if members.length === 0}
        <div class="p-6 text-sm text-muted-foreground text-center">
          No members.
        </div>
      {:else}
        {#each members as m (m.user_id)}
          {@const Icon = roleIcon(m.role)}
          {@const isSelf = m.user_id === callerUserId}
          <div class="flex items-center gap-3 px-3 py-2.5 border-b border-border-subtle last:border-b-0">
            <div class="grid place-items-center size-8 rounded-md bg-muted text-muted-foreground shrink-0">
              <Icon class="size-4" />
            </div>
            <div class="min-w-0 flex-1">
              <div class="text-sm font-medium truncate flex items-center gap-2">
                {m.email}
                {#if isSelf}
                  <span class="text-2xs text-muted-foreground">(you)</span>
                {/if}
              </div>
              <div class="text-2xs text-muted-foreground">
                Joined {formatDate(m.invited_at)}
              </div>
            </div>

            {#if isOwner && !isSelf}
              <div class="w-28">
                <Select
                  items={roleItems}
                  value={m.role}
                  onValueChange={(v) => handleRoleChange(m, v)}
                />
              </div>
              <Button variant="outline" size="icon" onclick={() => handleRevoke(m)} aria-label="Remove">
                <Trash2 class="size-3.5" />
              </Button>
            {:else}
              <span class="text-2xs uppercase tracking-wider text-muted-foreground px-2 py-0.5 rounded-full border border-border-subtle bg-muted/40">
                {m.role}
              </span>
            {/if}
          </div>
        {/each}
      {/if}
    </div>

    <!-- Invite form (owners only) -->
    {#if isOwner}
      <div class="mt-4 space-y-2">
        <Label class="flex items-center gap-1.5">
          <UserPlus class="size-3.5" />
          Invite a member
        </Label>
        <div class="flex gap-2">
          <Input
            type="email"
            bind:value={inviteEmail}
            placeholder="user@example.com"
            class="flex-1"
            onkeydown={(e) => e.key === "Enter" && void handleInvite()}
          />
          <div class="w-28">
            <Select
              items={inviteRoleItems}
              value={inviteRole}
              onValueChange={(v) => (inviteRole = v as "editor" | "reader")}
            />
          </div>
          <Button onclick={handleInvite} disabled={inviting || !inviteEmail.trim()}>
            {#if inviting}<Loader2 class="size-4 animate-spin" />Inviting…{:else}Invite{/if}
          </Button>
        </div>
        {#if inviteError}
          <p class="text-xs text-destructive break-all">{inviteError}</p>
        {/if}
        <p class="text-2xs text-muted-foreground">
          The recipient must already have a cloud account on this server with sharing enabled.
        </p>
      </div>
    {:else}
      <p class="mt-3 text-xs text-muted-foreground flex items-center gap-1.5">
        <Users class="size-3.5" />
        Only the owner can change roles or invite members.
      </p>
    {/if}
  {/if}

  {#snippet footer()}
    <Button variant="outline" onclick={() => (open = false)}>Close</Button>
  {/snippet}
</Dialog>
