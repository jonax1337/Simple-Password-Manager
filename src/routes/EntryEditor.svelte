<script lang="ts">
  import { Button, Input, Textarea, Label, Checkbox, Select, Tabs, TabContent, toast } from "$lib/ui";
  import IconPicker from "$lib/components/IconPicker.svelte";
  import PasswordStrengthMeter from "$lib/components/PasswordStrengthMeter.svelte";
  import TotpSection from "$lib/components/TotpSection.svelte";
  import { Eye, EyeOff, Copy, Wand2, Check, Clock, Save, ArrowLeft, Shield, Plus, Trash2 } from "@lucide/svelte";
  import { writeText } from "@tauri-apps/plugin-clipboard-manager";
  import { onMount } from "svelte";
  import { pop } from "svelte-spa-router";
  import { getEntry, updateEntry, generatePassword, type EntryData, type CustomField, type HistoryEntry } from "$lib/tauri";
  import { validateUrl, getDefaultExpiryDate, getExpiryDate, formatTimestamp } from "$lib/entry-utils";

  type Props = { params?: { uuid?: string } };
  let { params }: Props = $props();

  const uuid = $derived(params?.uuid ?? "");

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
  let formData = $state<EntryData>({ ...empty });
  let original = $state<EntryData>({ ...empty });
  let showPassword = $state(false);
  let repeatPassword = $state("");
  let hasChanges = $state(false);
  let urlError = $state<string | null>(null);
  let copiedUsername = $state(false);
  let copiedPassword = $state(false);
  let tab = $state("general");

  let editingFieldIndex = $state<number | null>(null);
  let editingField = $state<CustomField | null>(null);

  onMount(() => {
    void load();
  });

  async function load() {
    try {
      const e = await getEntry(uuid);
      formData = e;
      original = { ...e };
      repeatPassword = e.password;
      hasChanges = false;
      loaded = true;
    } catch (err) {
      toast.error("Failed to load entry", String(err));
      pop();
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

  async function copy(text: string, label: string, field: "username" | "password") {
    if (!text) return;
    try {
      await writeText(text);
      if (field === "username") {
        copiedUsername = true;
        setTimeout(() => (copiedUsername = false), 2000);
      } else {
        copiedPassword = true;
        setTimeout(() => (copiedPassword = false), 2000);
      }
      toast.success("Copied", `${label} copied to clipboard`);
      setTimeout(() => void writeText(""), 30000);
    } catch {
      toast.error("Failed to copy to clipboard");
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
    try {
      await updateEntry(formData);
      toast.success("Saved");
      hasChanges = false;
      original = { ...formData };
    } catch (e) {
      toast.error("Error", String(e));
    }
  }

  function handleCancel() {
    pop();
  }

  // custom fields helpers
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

  // restore from history
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
</script>

<div class="flex h-full flex-col">
  {#if loaded}
    <div class="shrink-0 flex items-center gap-3 border-b px-4 py-3 bg-card/30">
      <Button variant="ghost" size="icon" onclick={handleCancel} title="Back">
        <ArrowLeft class="h-4 w-4" />
      </Button>
      <IconPicker
        value={formData.icon_id ?? 0}
        onChange={(id) => patch({ icon_id: id })}
      />
      <div class="min-w-0 flex-1">
        <h1 class="text-lg font-semibold truncate">{formData.title || "Edit Entry"}</h1>
        <p class="text-xs text-muted-foreground">
          {hasChanges ? "Unsaved changes" : "No changes"}
        </p>
      </div>
      <Button variant="outline" onclick={handleCancel}>Cancel</Button>
      <Button onclick={handleSave} disabled={!hasChanges || !!urlError || !passwordsMatch}>
        <Save class="mr-2 h-4 w-4" />
        Save
      </Button>
    </div>

    <Tabs bind:value={tab} tabs={[
      { value: "general", label: "General" },
      { value: "advanced", label: "Advanced" },
      { value: "history", label: "History" },
    ]} class="flex-1 min-h-0 px-4 pt-3">
      {#snippet children(_value: string)}
        <TabContent value="general" class="overflow-y-auto pt-2 pb-4">
          <div class="space-y-4 max-w-2xl">
            <div class="grid grid-cols-[100px_1fr] items-center gap-2">
              <Label for="title">Title:</Label>
              <Input id="title" value={formData.title} oninput={(e) => patch({ title: e.currentTarget.value })} placeholder="Entry title" />
            </div>

            <div class="grid grid-cols-[100px_1fr_auto] items-center gap-2">
              <Label for="username">User name:</Label>
              <Input id="username" value={formData.username} oninput={(e) => patch({ username: e.currentTarget.value })} />
              <Button
                variant="outline"
                size="icon"
                onclick={() => copy(formData.username, "Username", "username")}
                disabled={!formData.username}
                title="Copy username"
              >
                {#if copiedUsername}<Check class="h-4 w-4 text-success" />{:else}<Copy class="h-4 w-4" />{/if}
              </Button>
            </div>

            <div class="grid grid-cols-[100px_1fr_auto] items-center gap-2">
              <Label for="password">Password:</Label>
              <div class="relative">
                <Input
                  id="password"
                  type={showPassword ? "text" : "password"}
                  value={formData.password}
                  oninput={(e) => patch({ password: e.currentTarget.value })}
                  class="pr-10"
                />
                <button
                  type="button"
                  class="absolute right-0 top-0 h-full px-2 text-muted-foreground hover:text-foreground"
                  onclick={() => (showPassword = !showPassword)}
                  aria-label={showPassword ? "Hide password" : "Show password"}
                >
                  {#if showPassword}<EyeOff class="h-4 w-4" />{:else}<Eye class="h-4 w-4" />{/if}
                </button>
              </div>
              <Button
                variant="outline"
                size="icon"
                onclick={() => copy(formData.password, "Password", "password")}
                disabled={!formData.password}
                title="Copy password"
              >
                {#if copiedPassword}<Check class="h-4 w-4 text-success" />{:else}<Copy class="h-4 w-4" />{/if}
              </Button>
            </div>

            <div class="grid grid-cols-[100px_1fr_auto] items-center gap-2">
              <Label for="repeat">Repeat:</Label>
              <Input
                id="repeat"
                type="password"
                bind:value={repeatPassword}
                class={!passwordsMatch ? "border-destructive" : ""}
                disabled={showPassword}
              />
              <div class="w-9">
                <Select
                  items={strengthSelectItems}
                  value={undefined}
                  placeholder=""
                  onValueChange={(v) => handleGenerate(v)}
                  class="h-9 w-9 px-0 justify-center"
                />
              </div>
            </div>

            <div class="grid grid-cols-[100px_1fr] items-center gap-2">
              <Label>Quality:</Label>
              <PasswordStrengthMeter password={formData.password} />
            </div>

            <div class="grid grid-cols-[100px_1fr] items-center gap-2">
              <Label for="url">URL:</Label>
              <div class="space-y-1">
                <Input
                  id="url"
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
            </div>

            <TotpSection formData={formData} onUpdate={(next) => { formData = next; hasChanges = true; }} />

            <div class="grid grid-cols-[100px_1fr] items-start gap-2">
              <Label for="notes" class="pt-2">Notes:</Label>
              <Textarea
                id="notes"
                value={formData.notes}
                oninput={(e) => patch({ notes: e.currentTarget.value })}
                placeholder="Additional notes…"
                class="min-h-[100px]"
              />
            </div>

            <div class="grid grid-cols-[100px_1fr] items-center gap-2">
              <label class="flex items-center gap-2">
                <Checkbox checked={formData.expires} onCheckedChange={(c) => handleExpiresToggle(c === true)} />
                <span class="text-sm font-medium">Expires:</span>
              </label>
              <div class="flex gap-2 w-full">
                <Input
                  type="datetime-local"
                  value={formData.expiry_time?.slice(0, 16) || ""}
                  oninput={(e) => patch({ expiry_time: e.currentTarget.value, expires: true })}
                  disabled={!formData.expires}
                  class="flex-1"
                />
                <div class="w-9">
                  <Select
                    items={expiryPresetItems}
                    value={undefined}
                    placeholder=""
                    disabled={!formData.expires}
                    onValueChange={(v) => patch({ expiry_time: getExpiryDate(v), expires: true })}
                    class="h-9 w-9 px-0 justify-center"
                  />
                </div>
              </div>
            </div>
          </div>
        </TabContent>

        <TabContent value="advanced" class="overflow-y-auto pt-2 pb-4">
          <div class="space-y-4 max-w-2xl">
            <div class="border rounded-md">
              <div class="px-3 py-2 bg-muted/50 border-b font-medium text-sm flex items-center justify-between">
                String fields
                <Button variant="outline" size="sm" onclick={addField}>
                  <Plus class="h-3.5 w-3.5" />
                  Add
                </Button>
              </div>
              {#if (formData.custom_fields ?? []).length === 0}
                <div class="text-center text-muted-foreground py-8 text-sm">No custom fields</div>
              {:else}
                <table class="w-full text-sm">
                  <thead class="border-b">
                    <tr>
                      <th class="text-left font-medium px-3 py-2 w-[180px]">Name</th>
                      <th class="text-left font-medium px-3 py-2">Value</th>
                      <th class="w-16"></th>
                    </tr>
                  </thead>
                  <tbody>
                    {#each (formData.custom_fields ?? []) as field, i (i)}
                      <tr
                        class="border-b last:border-b-0 cursor-pointer hover:bg-accent/30 {editingFieldIndex === i ? 'bg-accent' : ''}"
                        onclick={() => {
                          editingFieldIndex = i;
                          editingField = { ...field };
                        }}
                      >
                        <td class="px-3 py-2 font-medium">
                          {#if field.protected}<Shield class="h-3 w-3 inline mr-1" />{/if}
                          {field.name || "(unnamed)"}
                        </td>
                        <td class="px-3 py-2 font-mono">{field.protected ? "••••••••" : field.value || "—"}</td>
                        <td class="px-3 py-2 text-right">
                          <Button variant="ghost" size="icon" class="h-7 w-7" onclick={(e: MouseEvent) => { e.stopPropagation(); deleteField(i); }}>
                            <Trash2 class="h-3.5 w-3.5 text-destructive" />
                          </Button>
                        </td>
                      </tr>
                    {/each}
                  </tbody>
                </table>
              {/if}
            </div>

            {#if editingField !== null && editingFieldIndex !== null}
              <div class="border rounded-md p-4 space-y-3 bg-muted/30">
                <h4 class="font-medium">Edit Field</h4>
                <div class="grid grid-cols-[80px_1fr] items-center gap-2">
                  <Label>Name:</Label>
                  <Input
                    value={editingField.name}
                    oninput={(e) => editingField && (editingField = { ...editingField, name: e.currentTarget.value })}
                    placeholder="Field name"
                  />
                </div>
                <div class="grid grid-cols-[80px_1fr] items-center gap-2">
                  <Label>Value:</Label>
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
                    onCheckedChange={(c) => editingField && (editingField = { ...editingField, protected: c === true })}
                  />
                  <Shield class="h-3 w-3" /> Protected
                </label>
                <div class="flex gap-2 justify-end">
                  <Button variant="outline" size="sm" onclick={() => { editingFieldIndex = null; editingField = null; }}>
                    Cancel
                  </Button>
                  <Button size="sm" onclick={saveField}>OK</Button>
                </div>
              </div>
            {/if}

            <div class="border rounded-md">
              <div class="px-3 py-2 bg-muted/50 border-b font-medium text-sm">Tags</div>
              <div class="p-3">
                <Input
                  value={formData.tags}
                  oninput={(e) => patch({ tags: e.currentTarget.value })}
                  placeholder="Comma-separated tags"
                />
              </div>
            </div>
          </div>
        </TabContent>

        <TabContent value="history" class="overflow-y-auto pt-2 pb-4">
          <div class="space-y-3 max-w-2xl">
            {#if (formData.history ?? []).length === 0}
              <p class="text-sm text-muted-foreground">No history yet.</p>
            {:else}
              <table class="w-full text-sm border rounded-md overflow-hidden">
                <thead class="bg-muted/50 border-b text-xs">
                  <tr>
                    <th class="text-left px-3 py-2">Timestamp</th>
                    <th class="text-left px-3 py-2">Title</th>
                    <th class="text-left px-3 py-2">Username</th>
                    <th class="w-24"></th>
                  </tr>
                </thead>
                <tbody>
                  {#each formData.history as h, i (i)}
                    <tr class="border-b last:border-b-0">
                      <td class="px-3 py-2 text-xs text-muted-foreground whitespace-nowrap">{formatTimestamp(h.timestamp)}</td>
                      <td class="px-3 py-2 truncate">{h.title || "—"}</td>
                      <td class="px-3 py-2 truncate text-muted-foreground">{h.username || "—"}</td>
                      <td class="px-3 py-2 text-right">
                        <Button variant="outline" size="sm" onclick={() => restoreHistory(h)}>Restore</Button>
                      </td>
                    </tr>
                  {/each}
                </tbody>
              </table>
            {/if}
          </div>
        </TabContent>
      {/snippet}
    </Tabs>
  {:else}
    <div class="flex-1 grid place-items-center text-sm text-muted-foreground">Loading…</div>
  {/if}
</div>
