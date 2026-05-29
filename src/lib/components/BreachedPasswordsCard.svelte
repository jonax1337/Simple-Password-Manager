<script lang="ts">
  import { Button, Card, CardHeader, CardTitle, CardDescription, CardContent, toast } from "$lib/ui";
  import { ShieldAlert, EyeOff, RefreshCw, ChevronRight } from "@lucide/svelte";
  import {
    checkBreachedPasswords,
    type BreachedEntry,
  } from "$lib/tauri";
  import { getDismissedBreaches, saveDismissedBreach, clearDismissedBreach } from "$lib/storage";

  type Props = {
    refreshTrigger?: number;
    databasePath?: string;
    isDirty?: boolean;
    onJumpToEntry?: (uuid: string) => void;
  };

  let { refreshTrigger = 0, databasePath, isDirty = false, onJumpToEntry }: Props = $props();

  let loading = $state(false);
  let breaches = $state<BreachedEntry[]>([]);
  let dismissed = $state<Set<string>>(new Set());
  let lastChecked = $state<Date | null>(null);
  let error = $state<string | null>(null);
  let showDismissed = $state(false);

  $effect(() => {
    void refreshTrigger;
    void databasePath;
    if (databasePath) void run();
  });

  async function run() {
    if (!databasePath) return;
    loading = true;
    error = null;
    try {
      const [list, dismissedList] = await Promise.all([
        checkBreachedPasswords(),
        getDismissedBreaches(databasePath),
      ]);
      breaches = list;
      dismissed = new Set(dismissedList);
      lastChecked = new Date();
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  async function dismiss(uuid: string) {
    if (!databasePath) return;
    try {
      await saveDismissedBreach(databasePath, uuid);
      dismissed = new Set([...dismissed, uuid]);
    } catch (e) {
      toast.error("Failed", String(e));
    }
  }

  async function undismiss(uuid: string) {
    if (!databasePath) return;
    try {
      await clearDismissedBreach(databasePath, uuid);
      const next = new Set(dismissed);
      next.delete(uuid);
      dismissed = next;
    } catch (e) {
      toast.error("Failed", String(e));
    }
  }

  const active = $derived(breaches.filter((b) => !dismissed.has(b.uuid)));
  const dismissedItems = $derived(breaches.filter((b) => dismissed.has(b.uuid)));
</script>

<Card>
  <CardHeader>
    <div class="flex items-start justify-between gap-2">
      <div>
        <CardTitle class="flex items-center gap-2">
          <ShieldAlert class="h-4 w-4 {active.length > 0 ? 'text-destructive' : 'text-success'}" />
          Breach detection
        </CardTitle>
        <CardDescription>
          {#if active.length > 0}
            <span class="text-destructive font-medium">{active.length}</span>
            {active.length === 1 ? "password" : "passwords"} found in known breaches
          {:else if breaches.length > 0}
            All known breaches dismissed
          {:else if lastChecked}
            No breached passwords found
          {:else}
            Check your passwords against haveibeenpwned.com
          {/if}
        </CardDescription>
      </div>
      <Button size="sm" variant="outline" onclick={run} disabled={loading}>
        <RefreshCw class="h-3.5 w-3.5 {loading ? 'animate-spin' : ''}" />
        {loading ? "Checking…" : "Re-check"}
      </Button>
    </div>
  </CardHeader>

  <CardContent>
    {#if isDirty}
      <p class="text-xs text-muted-foreground mb-3">
        ⚠ The database has unsaved changes. Save first to ensure breach data reflects current entries.
      </p>
    {/if}

    {#if error}
      <p class="text-xs text-destructive">{error}</p>
    {:else if active.length === 0 && breaches.length === 0 && lastChecked}
      <p class="text-sm text-success">Nothing to worry about.</p>
    {:else if active.length === 0 && breaches.length === 0 && !loading}
      <p class="text-sm text-muted-foreground">Click "Re-check" to scan your database.</p>
    {/if}

    {#if active.length > 0}
      <ul class="space-y-1.5">
        {#each active as b (b.uuid)}
          <li class="flex items-center gap-2 rounded-md border border-destructive/30 bg-destructive/5 px-3 py-2 text-sm">
            <div class="flex-1 min-w-0">
              <div class="font-medium truncate">{b.title}</div>
              <div class="text-xs text-muted-foreground truncate">
                {b.username ? `${b.username} · ` : ""}seen {b.breach_count.toLocaleString()} times
              </div>
            </div>
            {#if onJumpToEntry}
              <Button size="sm" variant="ghost" onclick={() => onJumpToEntry(b.uuid)} title="Open entry">
                <ChevronRight class="h-3.5 w-3.5" />
              </Button>
            {/if}
            <Button size="sm" variant="ghost" onclick={() => dismiss(b.uuid)} title="Dismiss for this database">
              <EyeOff class="h-3.5 w-3.5" />
            </Button>
          </li>
        {/each}
      </ul>
    {/if}

    {#if dismissedItems.length > 0}
      <button
        type="button"
        class="mt-3 text-xs text-muted-foreground hover:text-foreground"
        onclick={() => (showDismissed = !showDismissed)}
      >
        {showDismissed ? "▾" : "▸"}
        {dismissedItems.length} dismissed
      </button>
      {#if showDismissed}
        <ul class="mt-2 space-y-1">
          {#each dismissedItems as b (b.uuid)}
            <li class="flex items-center gap-2 rounded-md border bg-muted/20 px-3 py-1.5 text-xs">
              <div class="flex-1 min-w-0">
                <span class="truncate text-muted-foreground">{b.title}</span>
              </div>
              <button type="button" class="text-muted-foreground hover:text-foreground" onclick={() => undismiss(b.uuid)}>
                Restore
              </button>
            </li>
          {/each}
        </ul>
      {/if}
    {/if}
  </CardContent>
</Card>
