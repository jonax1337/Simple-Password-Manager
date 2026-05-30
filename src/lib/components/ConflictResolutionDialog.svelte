<script lang="ts">
  import { Dialog, Button, Eyebrow } from "$lib/ui";
  import type { ConflictChoice, EntryConflict } from "$lib/tauri";
  import { AlertCircle, ChevronLeft, ChevronRight, ArrowDownUp } from "@lucide/svelte";

  type Props = {
    open?: boolean;
    conflicts: EntryConflict[];
    onResolve: (decisions: Record<string, ConflictChoice>) => void | Promise<void>;
    onCancel: () => void;
  };

  let {
    open = $bindable(false),
    conflicts,
    onResolve,
    onCancel,
  }: Props = $props();

  let decisions = $state<Record<string, ConflictChoice>>({});
  let index = $state(0);

  // Reset state every time a fresh conflict batch arrives so a previous run
  // doesn't bleed into this one. Comparing by length + first UUID is cheap
  // and matches the "new batch" semantics callers expect.
  let lastBatchKey = $state<string>("");
  $effect(() => {
    const key = `${conflicts.length}:${conflicts[0]?.uuid ?? ""}`;
    if (key !== lastBatchKey) {
      lastBatchKey = key;
      decisions = {};
      index = 0;
    }
  });

  const current = $derived(conflicts[index] ?? null);
  const decidedCount = $derived(Object.keys(decisions).length);
  const allDecided = $derived(decidedCount === conflicts.length && conflicts.length > 0);

  type FieldDiff = { name: string; label: string; local: string; remote: string; differs: boolean };

  const fields = $derived.by<FieldDiff[]>(() => {
    if (!current) return [];
    const l = current.local;
    const r = current.remote;
    const base: FieldDiff[] = [
      { name: "title", label: "Title", local: l.title, remote: r.title, differs: l.title !== r.title },
      { name: "username", label: "Username", local: l.username, remote: r.username, differs: l.username !== r.username },
      { name: "password", label: "Password", local: l.password, remote: r.password, differs: l.password !== r.password },
      { name: "url", label: "URL", local: l.url, remote: r.url, differs: l.url !== r.url },
      { name: "notes", label: "Notes", local: l.notes, remote: r.notes, differs: l.notes !== r.notes },
      { name: "tags", label: "Tags", local: l.tags, remote: r.tags, differs: l.tags !== r.tags },
      { name: "favorite", label: "Favorite", local: String(l.is_favorite), remote: String(r.is_favorite), differs: l.is_favorite !== r.is_favorite },
    ];
    return base.filter((f) => f.differs);
  });

  function choose(c: ConflictChoice) {
    if (!current) return;
    decisions = { ...decisions, [current.uuid]: c };
    // Auto-advance to next undecided entry so the user lands somewhere useful.
    const nextIdx = findNextUndecided(index);
    if (nextIdx !== -1) index = nextIdx;
  }

  function findNextUndecided(from: number): number {
    for (let i = from + 1; i < conflicts.length; i++) {
      if (!(conflicts[i].uuid in decisions)) return i;
    }
    for (let i = 0; i <= from; i++) {
      if (!(conflicts[i].uuid in decisions)) return i;
    }
    return -1;
  }

  function decisionLabel(c: EntryConflict): string {
    const d = decisions[c.uuid];
    if (!d) return "";
    return d === "keep_local" ? "Local" : "Remote";
  }

  async function applyAll() {
    if (!allDecided) return;
    await onResolve(decisions);
  }

  function quickResolveAll(c: ConflictChoice) {
    const next: Record<string, ConflictChoice> = {};
    for (const conf of conflicts) next[conf.uuid] = c;
    decisions = next;
  }

  function truncate(s: string, n: number): string {
    if (!s) return "(empty)";
    if (s.length <= n) return s;
    return s.slice(0, n) + "…";
  }
</script>

<Dialog bind:open class="max-w-3xl" onOpenChange={(o) => !o && onCancel()}>
  <div class="flex items-center gap-2 mb-3">
    <AlertCircle class="h-5 w-5 text-warning" />
    <h2 class="text-lg font-semibold tracking-tight">Resolve sync conflicts</h2>
    <span class="ml-auto text-xs text-muted-foreground">
      {decidedCount} / {conflicts.length} decided
    </span>
  </div>

  <p class="text-sm text-muted-foreground -mt-1">
    The same entry was edited in two places. Pick which version to keep — per entry, or apply one choice to all.
  </p>

  <div class="flex items-center gap-2 mt-1">
    <Button variant="ghost" size="sm" onclick={() => quickResolveAll("keep_local")}>
      Keep all local
    </Button>
    <Button variant="ghost" size="sm" onclick={() => quickResolveAll("keep_remote")}>
      Keep all remote
    </Button>
  </div>

  <div class="grid grid-cols-[200px_1fr] gap-3 mt-2 max-h-[60vh]">
    <!-- Conflict list -->
    <div class="border rounded-md overflow-y-auto min-h-0">
      {#each conflicts as c, i (c.uuid)}
        {@const active = i === index}
        {@const decided = c.uuid in decisions}
        <button
          type="button"
          class="w-full text-left px-2.5 py-2 border-b last:border-b-0 text-sm hover:bg-accent/50 transition-colors {active ? 'bg-accent/70' : ''}"
          onclick={() => (index = i)}
        >
          <div class="font-medium truncate">{c.local.title || c.remote.title || "(untitled)"}</div>
          <div class="text-2xs text-muted-foreground flex items-center gap-1.5">
            {#if decided}
              <span class="inline-block size-1.5 rounded-full bg-success/70"></span>
              <span>{decisionLabel(c)}</span>
            {:else}
              <span class="inline-block size-1.5 rounded-full bg-warning"></span>
              <span>Pending</span>
            {/if}
          </div>
        </button>
      {/each}
    </div>

    <!-- Diff view -->
    <div class="border rounded-md overflow-y-auto min-h-0 p-3">
      {#if current}
        <div class="flex items-center gap-1 mb-2">
          <Button
            variant="ghost"
            size="icon"
            onclick={() => (index = Math.max(0, index - 1))}
            disabled={index === 0}
            aria-label="Previous conflict"
          >
            <ChevronLeft class="size-4" />
          </Button>
          <div class="text-sm font-medium flex-1 text-center truncate">
            {current.local.title || current.remote.title || "(untitled)"}
          </div>
          <Button
            variant="ghost"
            size="icon"
            onclick={() => (index = Math.min(conflicts.length - 1, index + 1))}
            disabled={index === conflicts.length - 1}
            aria-label="Next conflict"
          >
            <ChevronRight class="size-4" />
          </Button>
        </div>

        <div class="grid grid-cols-2 gap-2 text-xs">
          <Eyebrow>Local (this device)</Eyebrow>
          <Eyebrow>Remote (on disk)</Eyebrow>
          {#each fields as f (f.name)}
            <div class="bg-muted/40 rounded p-2 space-y-0.5">
              <div class="text-2xs text-muted-foreground">{f.label}</div>
              <div class="font-mono break-all whitespace-pre-wrap">{truncate(f.local, 240)}</div>
            </div>
            <div class="bg-muted/40 rounded p-2 space-y-0.5">
              <div class="text-2xs text-muted-foreground">{f.label}</div>
              <div class="font-mono break-all whitespace-pre-wrap">{truncate(f.remote, 240)}</div>
            </div>
          {/each}
          {#if fields.length === 0}
            <div class="col-span-2 text-sm text-muted-foreground italic">
              Only metadata differs (icon, group, custom fields).
            </div>
          {/if}
        </div>

        <div class="flex gap-2 mt-3">
          <Button
            variant={decisions[current.uuid] === "keep_local" ? "default" : "outline"}
            class="flex-1"
            onclick={() => choose("keep_local")}
          >
            Keep local
          </Button>
          <Button
            variant={decisions[current.uuid] === "keep_remote" ? "default" : "outline"}
            class="flex-1"
            onclick={() => choose("keep_remote")}
          >
            Keep remote
          </Button>
        </div>
      {/if}
    </div>
  </div>

  <div class="flex items-center gap-2 mt-3">
    <Button variant="ghost" onclick={onCancel}>Cancel</Button>
    <div class="flex-1"></div>
    <Button onclick={applyAll} disabled={!allDecided}>
      <ArrowDownUp class="size-4" />
      Apply &amp; save
    </Button>
  </div>
</Dialog>
