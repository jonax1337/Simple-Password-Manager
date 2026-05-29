<script lang="ts">
  import { Button, Input, Textarea, Label, Checkbox, Select, toast, DropdownMenu, DropdownItem, DropdownSeparator } from "$lib/ui";
  import IconPicker from "./IconPicker.svelte";
  import DynamicIcon from "./DynamicIcon.svelte";
  import PasswordStrengthMeter from "./PasswordStrengthMeter.svelte";
  import TotpSection from "./TotpSection.svelte";
  import TotpFieldCard from "./TotpFieldCard.svelte";
  import {
    Eye,
    EyeOff,
    Copy,
    Check,
    Save,
    Shield,
    Plus,
    Trash2,
    X,
    Star,
    ExternalLink,
    Pencil,
    MoreHorizontal,
    Tag,
    Clock,
    Calendar,
    History,
  } from "@lucide/svelte";
  import { writeText } from "@tauri-apps/plugin-clipboard-manager";
  import { open as openShell } from "@tauri-apps/plugin-shell";
  import {
    getEntry,
    updateEntry,
    deleteEntry,
    createEntry,
    generatePassword,
    type EntryData,
    type CustomField,
    type HistoryEntry,
  } from "$lib/tauri";
  import { undoStack } from "$lib/undo-stack.svelte";
  import { appState } from "$lib/app-state.svelte";
  import { validateUrl, getDefaultExpiryDate, getExpiryDate, formatTimestamp, getTotpFieldValue } from "$lib/entry-utils";
  import { ask } from "@tauri-apps/plugin-dialog";

  type Props = {
    uuid: string;
    onClose?: () => void;
    onChange?: () => void | Promise<void>;
  };
  let { uuid, onClose, onChange }: Props = $props();

  const empty: EntryData = {
    uuid: "",
    title: "",
    username: "",
    password: "",
    url: "",
    notes: "",
    tags: "",
    group_uuid: "",
    icon_id: 0,
    is_favorite: false,
    expires: false,
    usage_count: 0,
    custom_fields: [],
    history: [],
  };
  let loaded = $state(false);
  let entry = $state<EntryData>({ ...empty });
  let formData = $state<EntryData>({ ...empty });
  let editing = $state(false);
  let showPassword = $state(false);
  let repeatPassword = $state("");
  let hasChanges = $state(false);
  let urlError = $state<string | null>(null);
  let saving = $state(false);
  let editingFieldIndex = $state<number | null>(null);
  let editingField = $state<CustomField | null>(null);

  // Per-row "copied" flashes for the read-mode login card
  let copiedUsername = $state(false);
  let copiedPassword = $state(false);
  let copiedUrl = $state(false);
  let revealReadPassword = $state(false);

  $effect(() => {
    void uuid;
    loaded = false;
    editing = false;
    revealReadPassword = false;
    if (uuid) void load(uuid);
  });

  // Silently re-sync from disk when something external touched the DB
  // (DnD move, remote merge, etc.) — but only while we're idle in read
  // mode. We never blow away an in-progress edit.
  $effect(() => {
    void appState.refreshCounter;
    if (uuid && loaded && !editing && !hasChanges) {
      void silentReload();
    }
  });

  async function silentReload() {
    try {
      const e = await getEntry(uuid);
      entry = e;
      formData = { ...e };
      repeatPassword = e.password;
    } catch {
      // ignore — stay on whatever we had
    }
  }

  async function load(id: string) {
    try {
      const e = await getEntry(id);
      entry = e;
      formData = { ...e };
      repeatPassword = e.password;
      hasChanges = false;
      editingField = null;
      editingFieldIndex = null;
      loaded = true;
    } catch (err) {
      toast.error("Failed to load entry", String(err));
      onClose?.();
    }
  }

  function patch(p: Partial<EntryData>) {
    formData = { ...formData, ...p };
    hasChanges = true;
    if ("url" in p) {
      const v = validateUrl(formData.url);
      urlError = v.error;
    }
  }

  async function handleGenerate(strength: string) {
    try {
      let length = 16;
      let useSymbols = true;
      switch (strength) {
        case "weak": length = 8; useSymbols = false; break;
        case "medium": length = 12; break;
        case "strong": length = 16; break;
        case "very-strong": length = 20; break;
        case "maximum": length = 32; break;
      }
      const pw = await generatePassword(length, true, true, true, useSymbols);
      patch({ password: pw });
      repeatPassword = pw;
      toast.success("Password generated", `${strength} (${length} chars)`);
    } catch {
      toast.error("Failed to generate password");
    }
  }

  async function handleSave() {
    const v = validateUrl(formData.url);
    if (!v.isValid) {
      toast.error("Invalid URL", "Please enter a valid URL or leave it empty");
      return;
    }
    if (formData.password && repeatPassword && formData.password !== repeatPassword) {
      toast.error("Passwords don't match");
      return;
    }
    saving = true;
    const before = { ...entry };
    const after = { ...formData };
    try {
      await updateEntry(after);
      undoStack.add(
        `Edit entry "${after.title}"`,
        async () => { await updateEntry(before); },
        async () => { await updateEntry(after); },
      );
      toast.success("Saved");
      hasChanges = false;
      entry = { ...after };
      editing = false;
      appState.markDirty();
      await onChange?.();
    } catch (e) {
      toast.error("Error", String(e));
    } finally {
      saving = false;
    }
  }

  function handleCancel() {
    formData = { ...entry };
    repeatPassword = entry.password;
    hasChanges = false;
    urlError = null;
    editing = false;
    editingField = null;
    editingFieldIndex = null;
  }

  function startEdit() {
    formData = { ...entry };
    repeatPassword = entry.password;
    hasChanges = false;
    urlError = null;
    editing = true;
  }

  async function toggleFavorite() {
    if (editing) {
      patch({ is_favorite: !formData.is_favorite });
      return;
    }
    const next = !entry.is_favorite;
    try {
      const updated = { ...entry, is_favorite: next };
      await updateEntry(updated);
      undoStack.add(
        `${next ? "Favorite" : "Unfavorite"} "${entry.title}"`,
        async () => { await updateEntry(entry); },
        async () => { await updateEntry(updated); },
      );
      entry = updated;
      appState.markDirty();
      await onChange?.();
    } catch (e) {
      toast.error("Error", String(e));
    }
  }

  async function openCurrentUrl() {
    const url = (editing ? formData.url : entry.url) || "";
    if (!url) return;
    try {
      const full = url.match(/^https?:\/\//) ? url : `https://${url}`;
      await openShell(full);
    } catch {
      toast.error("Failed to open URL");
    }
  }

  async function copyValue(value: string, label: string, kind?: "username" | "password" | "url") {
    if (!value) return;
    try {
      await writeText(value);
      if (kind === "username") {
        copiedUsername = true;
        setTimeout(() => (copiedUsername = false), 1500);
      } else if (kind === "password") {
        copiedPassword = true;
        setTimeout(() => (copiedPassword = false), 1500);
      } else if (kind === "url") {
        copiedUrl = true;
        setTimeout(() => (copiedUrl = false), 1500);
      }
      toast.success("Copied", `${label} copied to clipboard`);
      setTimeout(() => void writeText(""), 30000);
    } catch {
      toast.error("Copy failed");
    }
  }

  async function handleDeleteEntry() {
    const ok = await ask(`Delete "${entry.title}"? You can undo with Ctrl+Z.`, {
      kind: "warning",
      title: "Delete Entry",
    });
    if (!ok) return;
    const snap = { ...entry };
    try {
      await deleteEntry(entry.uuid);
      undoStack.add(
        `Delete entry "${snap.title}"`,
        async () => { await createEntry(snap); },
        async () => { await deleteEntry(snap.uuid); },
      );
      toast.success("Entry deleted");
      await onChange?.();
      onClose?.();
    } catch (e) {
      toast.error("Error", String(e));
    }
  }

  function addField() {
    const newField: CustomField = { name: "", value: "", protected: false };
    patch({ custom_fields: [...(formData.custom_fields ?? []), newField] });
    editingFieldIndex = (formData.custom_fields ?? []).length;
    editingField = newField;
  }

  function deleteField(i: number) {
    const next = (formData.custom_fields ?? []).filter((_, idx) => idx !== i);
    patch({ custom_fields: next });
    if (editingFieldIndex === i) {
      editingFieldIndex = null;
      editingField = null;
    }
  }

  function saveField() {
    if (editingFieldIndex === null || !editingField) return;
    const next = [...(formData.custom_fields ?? [])];
    next[editingFieldIndex] = editingField;
    patch({ custom_fields: next });
    editingFieldIndex = null;
    editingField = null;
  }

  function restoreHistory(h: HistoryEntry) {
    patch({
      title: h.title,
      username: h.username,
      password: h.password,
      url: h.url,
      notes: h.notes,
    });
    repeatPassword = h.password;
  }

  const strengthSelectItems = [
    { value: "weak", label: "Weak (8)" },
    { value: "medium", label: "Medium (12)" },
    { value: "strong", label: "Strong (16)" },
    { value: "very-strong", label: "Very Strong (20)" },
    { value: "maximum", label: "Maximum (32)" },
  ];

  const expiryPresetItems = [
    { value: "1day", label: "1 Day" },
    { value: "1week", label: "1 Week" },
    { value: "2weeks", label: "2 Weeks" },
    { value: "1month", label: "1 Month" },
    { value: "3months", label: "3 Months" },
    { value: "6months", label: "6 Months" },
    { value: "1year", label: "1 Year" },
  ];

  const passwordsMatch = $derived(
    !(formData.password && repeatPassword && formData.password !== repeatPassword),
  );

  function handleExpiresToggle(checked: boolean) {
    if (checked && !formData.expiry_time) {
      patch({ expires: true, expiry_time: getDefaultExpiryDate() });
    } else {
      patch({ expires: checked });
    }
  }

  const readTotp = $derived(getTotpFieldValue(entry));
  const readExtraFields = $derived(
    (entry.custom_fields ?? []).filter((f) => f.name.toLowerCase() !== "otp"),
  );
  const readTags = $derived(
    (entry.tags ?? "")
      .split(",")
      .map((t) => t.trim())
      .filter(Boolean),
  );

  const passwordMask = $derived(entry.password ? "•".repeat(Math.min(entry.password.length, 14)) : "");
</script>

<div class="flex h-full flex-col bg-background">
  {#if loaded}
    {#if !editing}
      <!-- ======== READ MODE ======== -->
      <div class="shrink-0 px-8 pt-8 pb-6">
        <div class="flex items-start gap-5">
          <div class="grid place-items-center size-[72px] rounded-2xl bg-primary/10 text-primary shrink-0 shadow-xs">
            <DynamicIcon iconId={entry.icon_id ?? 0} class="size-9" />
          </div>
          <div class="min-w-0 flex-1 pt-1">
            <h1 class="text-[26px] font-semibold tracking-tight truncate leading-tight">
              {entry.title || "Untitled"}
            </h1>
            <p class="text-[12.5px] text-muted-foreground mt-1 truncate">
              {entry.username || entry.url || "—"}
            </p>
          </div>
          <div class="flex items-center gap-1 shrink-0 -mt-1">
            <button
              type="button"
              onclick={toggleFavorite}
              title={entry.is_favorite ? "Unfavorite" : "Favorite"}
              class="size-9 inline-flex items-center justify-center rounded-md hover:bg-accent/60 text-muted-foreground hover:text-foreground transition-colors"
              aria-label={entry.is_favorite ? "Unfavorite" : "Favorite"}
            >
              {#if entry.is_favorite}
                <Star class="size-4 text-warning fill-warning" />
              {:else}
                <Star class="size-4" />
              {/if}
            </button>
            <button
              type="button"
              onclick={startEdit}
              title="Edit"
              class="h-9 px-3 inline-flex items-center gap-1.5 rounded-md bg-primary/10 hover:bg-primary/15 text-primary text-[12.5px] font-medium transition-colors"
            >
              <Pencil class="size-3.5" />
              <span>Edit</span>
            </button>
            <DropdownMenu align="end">
              {#snippet trigger()}
                <button
                  type="button"
                  class="size-9 inline-flex items-center justify-center rounded-md hover:bg-accent/60 text-muted-foreground hover:text-foreground transition-colors"
                  aria-label="More actions"
                >
                  <MoreHorizontal class="size-4" />
                </button>
              {/snippet}
              {#if entry.url}
                <DropdownItem onSelect={openCurrentUrl}>
                  <ExternalLink class="size-4" />
                  <span>Open URL</span>
                </DropdownItem>
              {/if}
              <DropdownItem onSelect={() => copyValue(entry.username, "Username", "username")} disabled={!entry.username}>
                <Copy class="size-4" />
                <span>Copy username</span>
              </DropdownItem>
              <DropdownItem onSelect={() => copyValue(entry.password, "Password", "password")} disabled={!entry.password}>
                <Copy class="size-4" />
                <span>Copy password</span>
              </DropdownItem>
              <DropdownSeparator />
              <DropdownItem destructive onSelect={handleDeleteEntry}>
                <Trash2 class="size-4" />
                <span>Delete entry</span>
              </DropdownItem>
            </DropdownMenu>
            {#if onClose}
              <button
                type="button"
                onclick={onClose}
                title="Close"
                class="size-9 inline-flex items-center justify-center rounded-md hover:bg-accent/60 text-muted-foreground hover:text-foreground transition-colors"
                aria-label="Close"
              >
                <X class="size-4" />
              </button>
            {/if}
          </div>
        </div>
      </div>

      <div class="flex-1 overflow-y-auto px-8 pb-8">
        <div class="max-w-2xl mx-auto space-y-3">
          <!-- LOGIN DETAILS card with hairline-divided sub-rows (1P style) -->
          <div class="rounded-lg border bg-card shadow-xs overflow-hidden">
            <div class="px-4 py-2 text-[10px] font-semibold uppercase tracking-wider text-muted-foreground border-b border-border/60 bg-muted/30">
              Login details
            </div>
            <div class="divide-y divide-border/60">
              <!-- Username -->
              <div class="group/row relative px-4 py-2.5 hover:bg-accent/30 transition-colors">
                <div class="text-[10.5px] font-medium uppercase tracking-wider text-muted-foreground/80 mb-0.5">Username</div>
                <div class="text-[13.5px] break-all pr-9 min-h-[18px]">
                  {#if entry.username}
                    {entry.username}
                  {:else}
                    <span class="text-muted-foreground/50 italic text-xs">empty</span>
                  {/if}
                </div>
                {#if entry.username}
                  <button
                    type="button"
                    onclick={() => copyValue(entry.username, "Username", "username")}
                    class="absolute right-2 top-1/2 -translate-y-1/2 size-7 inline-flex items-center justify-center rounded-md opacity-0 group-hover/row:opacity-100 transition-opacity hover:bg-foreground/10 text-muted-foreground hover:text-foreground"
                    aria-label="Copy username"
                    title="Copy username"
                  >
                    {#if copiedUsername}<Check class="size-3.5 text-success" />{:else}<Copy class="size-3.5" />{/if}
                  </button>
                {/if}
              </div>

              <!-- Password -->
              <div class="group/row relative px-4 py-2.5 hover:bg-accent/30 transition-colors">
                <div class="text-[10.5px] font-medium uppercase tracking-wider text-muted-foreground/80 mb-0.5">Password</div>
                <div class="text-[13.5px] font-mono break-all pr-16 min-h-[18px]">
                  {#if entry.password}
                    {revealReadPassword ? entry.password : passwordMask}
                  {:else}
                    <span class="text-muted-foreground/50 italic text-xs font-sans">empty</span>
                  {/if}
                </div>
                {#if entry.password}
                  <div class="absolute right-2 top-1/2 -translate-y-1/2 flex items-center gap-0.5">
                    <button
                      type="button"
                      onclick={() => (revealReadPassword = !revealReadPassword)}
                      class="size-7 inline-flex items-center justify-center rounded-md opacity-0 group-hover/row:opacity-100 transition-opacity hover:bg-foreground/10 text-muted-foreground hover:text-foreground"
                      aria-label={revealReadPassword ? "Hide password" : "Reveal password"}
                      title={revealReadPassword ? "Hide" : "Reveal"}
                    >
                      {#if revealReadPassword}<EyeOff class="size-3.5" />{:else}<Eye class="size-3.5" />{/if}
                    </button>
                    <button
                      type="button"
                      onclick={() => copyValue(entry.password, "Password", "password")}
                      class="size-7 inline-flex items-center justify-center rounded-md opacity-0 group-hover/row:opacity-100 transition-opacity hover:bg-foreground/10 text-muted-foreground hover:text-foreground"
                      aria-label="Copy password"
                      title="Copy password"
                    >
                      {#if copiedPassword}<Check class="size-3.5 text-success" />{:else}<Copy class="size-3.5" />{/if}
                    </button>
                  </div>
                {/if}
              </div>

              <!-- Website -->
              <div class="group/row relative px-4 py-2.5 hover:bg-accent/30 transition-colors">
                <div class="text-[10.5px] font-medium uppercase tracking-wider text-muted-foreground/80 mb-0.5">Website</div>
                <div class="text-[13.5px] break-all pr-16 min-h-[18px]">
                  {#if entry.url}
                    <button type="button" onclick={openCurrentUrl} class="text-primary hover:underline text-left">
                      {entry.url}
                    </button>
                  {:else}
                    <span class="text-muted-foreground/50 italic text-xs">empty</span>
                  {/if}
                </div>
                {#if entry.url}
                  <div class="absolute right-2 top-1/2 -translate-y-1/2 flex items-center gap-0.5">
                    <button
                      type="button"
                      onclick={openCurrentUrl}
                      class="size-7 inline-flex items-center justify-center rounded-md opacity-0 group-hover/row:opacity-100 transition-opacity hover:bg-foreground/10 text-muted-foreground hover:text-foreground"
                      aria-label="Open URL"
                      title="Open"
                    >
                      <ExternalLink class="size-3.5" />
                    </button>
                    <button
                      type="button"
                      onclick={() => copyValue(entry.url, "URL", "url")}
                      class="size-7 inline-flex items-center justify-center rounded-md opacity-0 group-hover/row:opacity-100 transition-opacity hover:bg-foreground/10 text-muted-foreground hover:text-foreground"
                      aria-label="Copy URL"
                      title="Copy URL"
                    >
                      {#if copiedUrl}<Check class="size-3.5 text-success" />{:else}<Copy class="size-3.5" />{/if}
                    </button>
                  </div>
                {/if}
              </div>
            </div>
          </div>

          {#if readTotp}
            <TotpFieldCard otpUri={readTotp} />
          {/if}

          {#if entry.notes}
            <div class="rounded-lg border bg-card shadow-xs px-4 py-3">
              <div class="text-[10.5px] font-medium uppercase tracking-wider text-muted-foreground/80 mb-1.5">
                Notes
              </div>
              <p class="text-[13px] whitespace-pre-wrap break-words text-foreground/90">{entry.notes}</p>
            </div>
          {/if}

          {#if readExtraFields.length > 0}
            <div class="rounded-lg border bg-card shadow-xs overflow-hidden">
              <div class="px-4 py-2 text-[10px] font-semibold uppercase tracking-wider text-muted-foreground border-b border-border/60 bg-muted/30">
                Additional fields
              </div>
              <div class="divide-y divide-border/60">
                {#each readExtraFields as f, i (i)}
                  <div class="group/row relative px-4 py-2.5 hover:bg-accent/30 transition-colors">
                    <div class="text-[10.5px] font-medium uppercase tracking-wider text-muted-foreground/80 mb-0.5 flex items-center gap-1.5">
                      {#if f.protected}<Shield class="size-3" />{/if}
                      {f.name || "(unnamed)"}
                    </div>
                    <div class="text-[13px] {f.protected ? 'font-mono' : ''} break-all pr-9 min-h-[18px]">
                      {#if f.value}
                        {f.protected ? "•".repeat(Math.min(f.value.length, 14)) : f.value}
                      {:else}
                        <span class="text-muted-foreground/50 italic text-xs font-sans">empty</span>
                      {/if}
                    </div>
                    {#if f.value}
                      <button
                        type="button"
                        onclick={() => copyValue(f.value, f.name || "Field")}
                        class="absolute right-2 top-1/2 -translate-y-1/2 size-7 inline-flex items-center justify-center rounded-md opacity-0 group-hover/row:opacity-100 transition-opacity hover:bg-foreground/10 text-muted-foreground hover:text-foreground"
                        aria-label="Copy {f.name || 'field'}"
                        title="Copy"
                      >
                        <Copy class="size-3.5" />
                      </button>
                    {/if}
                  </div>
                {/each}
              </div>
            </div>
          {/if}

          {#if entry.expires && entry.expiry_time}
            <div class="rounded-lg border bg-card shadow-xs px-4 py-3 flex items-center gap-3">
              <Calendar class="size-4 text-muted-foreground" />
              <div class="flex-1 min-w-0">
                <div class="text-[10.5px] font-medium uppercase tracking-wider text-muted-foreground/80">
                  Expires
                </div>
                <div class="text-[13px] mt-0.5">{formatTimestamp(entry.expiry_time)}</div>
              </div>
            </div>
          {/if}

          <div class="pt-3 mt-2 border-t border-border/60 flex flex-wrap items-center gap-x-5 gap-y-1.5 text-[11px] text-muted-foreground">
            {#if entry.created}
              <span class="inline-flex items-center gap-1.5">
                <Clock class="size-3" />
                Created {formatTimestamp(entry.created)}
              </span>
            {/if}
            {#if entry.modified}
              <span class="inline-flex items-center gap-1.5">
                <History class="size-3" />
                Modified {formatTimestamp(entry.modified)}
              </span>
            {/if}
            {#if readTags.length > 0}
              <span class="inline-flex items-center gap-1.5 flex-wrap">
                <Tag class="size-3" />
                {#each readTags as t (t)}
                  <span class="inline-flex items-center rounded-full bg-accent/60 px-2 py-0.5 text-[10.5px] text-foreground/80">{t}</span>
                {/each}
              </span>
            {/if}
          </div>
        </div>
      </div>
    {:else}
      <!-- ======== EDIT MODE ======== -->
      <div class="shrink-0 px-6 pt-6 pb-4 border-b">
        <div class="flex items-start gap-4">
          <IconPicker value={formData.icon_id ?? 0} onChange={(id) => patch({ icon_id: id })} />
          <div class="min-w-0 flex-1">
            <Input
              value={formData.title}
              oninput={(e) => patch({ title: e.currentTarget.value })}
              placeholder="Entry title"
              class="text-base font-semibold"
            />
            <p class="text-[11px] text-muted-foreground mt-1.5">
              {#if hasChanges}
                <span class="text-warning">●</span> Unsaved changes
              {:else}
                Last modified {formatTimestamp(entry.modified)}
              {/if}
            </p>
          </div>
          <div class="flex items-center gap-1 shrink-0">
            <button
              type="button"
              onclick={toggleFavorite}
              title={formData.is_favorite ? "Unfavorite" : "Favorite"}
              aria-label={formData.is_favorite ? "Unfavorite" : "Favorite"}
              class="size-9 inline-flex items-center justify-center rounded-md hover:bg-accent/60 text-muted-foreground hover:text-foreground transition-colors"
            >
              {#if formData.is_favorite}
                <Star class="size-4 text-warning fill-warning" />
              {:else}
                <Star class="size-4" />
              {/if}
            </button>
          </div>
        </div>
      </div>

      <div class="flex-1 overflow-y-auto px-6 py-5">
        <div class="max-w-2xl mx-auto space-y-5">
          <section class="space-y-3">
            <h3 class="text-[10px] font-semibold uppercase tracking-wider text-muted-foreground px-1">
              Login details
            </h3>

            <div class="space-y-2">
              <Label for="ed-username">Username</Label>
              <Input id="ed-username" value={formData.username} oninput={(e) => patch({ username: e.currentTarget.value })} placeholder="user@example.com" />
            </div>

            <div class="space-y-2">
              <Label for="ed-password">Password</Label>
              <div class="flex gap-2">
                <div class="relative flex-1">
                  <Input
                    id="ed-password"
                    type={showPassword ? "text" : "password"}
                    value={formData.password}
                    oninput={(e) => patch({ password: e.currentTarget.value })}
                    class="pr-10 font-mono"
                  />
                  <button
                    type="button"
                    class="absolute right-0 top-0 h-full px-2 text-muted-foreground hover:text-foreground"
                    onclick={() => (showPassword = !showPassword)}
                    aria-label={showPassword ? "Hide password" : "Show password"}
                  >
                    {#if showPassword}<EyeOff class="size-4" />{:else}<Eye class="size-4" />{/if}
                  </button>
                </div>
                <Select
                  items={strengthSelectItems}
                  value={undefined}
                  placeholder=""
                  onValueChange={(v) => handleGenerate(v)}
                  class="h-9 w-9 px-0 justify-center"
                />
              </div>
              <PasswordStrengthMeter password={formData.password} />
            </div>

            <div class="space-y-2">
              <Label for="ed-repeat">Repeat password</Label>
              <Input
                id="ed-repeat"
                type="password"
                bind:value={repeatPassword}
                class={!passwordsMatch ? "border-destructive font-mono" : "font-mono"}
                disabled={showPassword}
              />
            </div>

            <div class="space-y-2">
              <Label for="ed-url">Website</Label>
              <Input
                id="ed-url"
                type="url"
                value={formData.url}
                oninput={(e) => patch({ url: e.currentTarget.value })}
                class={urlError ? "border-destructive" : ""}
                placeholder="https://example.com"
              />
              {#if urlError}
                <p class="text-xs text-destructive">{urlError}</p>
              {/if}
            </div>
          </section>

          <section class="space-y-2">
            <h3 class="text-[10px] font-semibold uppercase tracking-wider text-muted-foreground px-1">
              Two-factor authentication
            </h3>
            <TotpSection formData={formData} onUpdate={(next) => { formData = next; hasChanges = true; }} />
          </section>

          <section class="space-y-2">
            <h3 class="text-[10px] font-semibold uppercase tracking-wider text-muted-foreground px-1">
              Notes
            </h3>
            <Textarea
              value={formData.notes}
              oninput={(e) => patch({ notes: e.currentTarget.value })}
              placeholder="Additional notes…"
              class="min-h-[110px]"
            />
          </section>

          <section class="space-y-2">
            <h3 class="text-[10px] font-semibold uppercase tracking-wider text-muted-foreground px-1">
              Tags
            </h3>
            <Input
              value={formData.tags}
              oninput={(e) => patch({ tags: e.currentTarget.value })}
              placeholder="Comma-separated tags"
            />
          </section>

          <section class="space-y-3">
            <div class="flex items-center justify-between px-1">
              <h3 class="text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">
                Additional fields
              </h3>
              <Button variant="outline" size="sm" onclick={addField}>
                <Plus class="size-3.5" />
                Add field
              </Button>
            </div>

            {#if (formData.custom_fields ?? []).filter((f) => f.name.toLowerCase() !== "otp").length === 0}
              <p class="text-[12px] text-muted-foreground italic px-1">No additional fields.</p>
            {:else}
              <div class="rounded-lg border divide-y divide-border/60 overflow-hidden">
                {#each formData.custom_fields ?? [] as field, i (i)}
                  {#if field.name.toLowerCase() !== "otp"}
                    <div class="flex items-center gap-3 px-3 py-2 hover:bg-accent/30 transition-colors">
                      <button
                        type="button"
                        class="flex-1 min-w-0 flex items-center gap-2 text-left"
                        onclick={() => {
                          editingFieldIndex = i;
                          editingField = { ...field };
                        }}
                      >
                        {#if field.protected}<Shield class="size-3 text-muted-foreground shrink-0" />{/if}
                        <span class="text-[12.5px] font-medium truncate">{field.name || "(unnamed)"}</span>
                        <span class="text-[12px] font-mono text-muted-foreground truncate">{field.protected ? "••••••••" : field.value || "—"}</span>
                      </button>
                      <button
                        type="button"
                        onclick={() => deleteField(i)}
                        class="size-7 inline-flex items-center justify-center rounded hover:bg-destructive/10 text-destructive"
                        title="Remove"
                        aria-label="Remove field"
                      >
                        <Trash2 class="size-3.5" />
                      </button>
                    </div>
                  {/if}
                {/each}
              </div>
            {/if}

            {#if editingField !== null && editingFieldIndex !== null}
              <div class="rounded-lg border p-3 space-y-3 bg-muted/30">
                <div class="grid grid-cols-[80px_1fr] items-center gap-2">
                  <Label>Name</Label>
                  <Input
                    value={editingField.name}
                    oninput={(e) => editingField && (editingField = { ...editingField, name: e.currentTarget.value })}
                    placeholder="Field name"
                  />
                </div>
                <div class="grid grid-cols-[80px_1fr] items-center gap-2">
                  <Label>Value</Label>
                  <Input
                    type={editingField.protected ? "password" : "text"}
                    value={editingField.value}
                    oninput={(e) => editingField && (editingField = { ...editingField, value: e.currentTarget.value })}
                    placeholder="Field value"
                  />
                </div>
                <label class="flex items-center gap-2 text-sm">
                  <Checkbox
                    checked={editingField.protected}
                    onCheckedChange={(c) =>
                      editingField && (editingField = { ...editingField, protected: c === true })}
                  />
                  <Shield class="size-3" /> Protected
                </label>
                <div class="flex gap-2 justify-end">
                  <Button
                    variant="outline"
                    size="sm"
                    onclick={() => {
                      editingFieldIndex = null;
                      editingField = null;
                    }}
                  >
                    Cancel
                  </Button>
                  <Button size="sm" onclick={saveField}>OK</Button>
                </div>
              </div>
            {/if}
          </section>

          <section class="space-y-2">
            <h3 class="text-[10px] font-semibold uppercase tracking-wider text-muted-foreground px-1">
              Expiration
            </h3>
            <div class="flex items-center gap-3">
              <label class="flex items-center gap-2 shrink-0">
                <Checkbox checked={formData.expires} onCheckedChange={(c) => handleExpiresToggle(c === true)} />
                <span class="text-[12.5px]">Expires</span>
              </label>
              <Input
                type="datetime-local"
                value={formData.expiry_time?.slice(0, 16) || ""}
                oninput={(e) => patch({ expiry_time: e.currentTarget.value, expires: true })}
                disabled={!formData.expires}
                class="flex-1"
              />
              <Select
                items={expiryPresetItems}
                value={undefined}
                placeholder=""
                disabled={!formData.expires}
                onValueChange={(v) => patch({ expiry_time: getExpiryDate(v), expires: true })}
                class="h-9 w-9 px-0 justify-center"
              />
            </div>
          </section>

          {#if (formData.history ?? []).length > 0}
            <section class="space-y-2">
              <h3 class="text-[10px] font-semibold uppercase tracking-wider text-muted-foreground px-1">
                History
              </h3>
              <div class="rounded-lg border divide-y divide-border/60 overflow-hidden">
                {#each formData.history ?? [] as h, i (i)}
                  <div class="flex items-center gap-3 px-3 py-2 text-[12px]">
                    <span class="text-muted-foreground tabular-nums shrink-0">{formatTimestamp(h.timestamp)}</span>
                    <span class="flex-1 min-w-0 truncate">{h.title || "—"}</span>
                    <Button variant="outline" size="sm" onclick={() => restoreHistory(h)}>Restore</Button>
                  </div>
                {/each}
              </div>
            </section>
          {/if}
        </div>
      </div>

      <div class="shrink-0 flex items-center justify-between gap-2 border-t px-6 py-3 bg-card/40 backdrop-blur">
        <p class="text-[11.5px] text-muted-foreground">
          {hasChanges ? "You have unsaved changes." : "Click Save to commit, or Cancel to discard."}
        </p>
        <div class="flex items-center gap-2">
          <Button variant="outline" onclick={handleCancel} disabled={saving}>Cancel</Button>
          <Button onclick={handleSave} disabled={saving || !!urlError || !passwordsMatch || !hasChanges}>
            <Save class="size-4" />
            {saving ? "Saving…" : "Save"}
          </Button>
        </div>
      </div>
    {/if}
  {:else}
    <div class="flex-1 grid place-items-center text-sm text-muted-foreground">Loading…</div>
  {/if}
</div>
