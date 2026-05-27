<script lang="ts">
  import { Card, CardHeader, CardTitle, CardDescription, CardContent, toast } from "$lib/ui";
  import { Shield, Key, AlertTriangle, Clock, Star, TrendingUp, Copy, KeyRound } from "@lucide/svelte";
  import { getDashboardStats, type DashboardStats } from "$lib/tauri";
  import { getHibpEnabled } from "$lib/storage";
  import { appState } from "$lib/app-state.svelte";
  import BreachedPasswordsCard from "./BreachedPasswordsCard.svelte";
  import { onMount } from "svelte";

  type Props = { refreshTrigger?: number; onJumpToEntry?: (uuid: string) => void };
  let { refreshTrigger = 0, onJumpToEntry }: Props = $props();

  let hibpEnabled = $state(false);
  onMount(() => {
    hibpEnabled = getHibpEnabled();
  });

  let stats = $state<DashboardStats | null>(null);

  $effect(() => {
    void refreshTrigger;
    void load();
  });

  async function load() {
    try {
      stats = await getDashboardStats();
    } catch (e) {
      toast.error("Error", "Failed to load dashboard statistics");
    }
  }

  function strengthLabel(bits: number): { label: string; cls: string } {
    if (bits < 40) return { label: "Weak", cls: "text-destructive" };
    if (bits < 64) return { label: "Fair", cls: "text-warning" };
    if (bits < 80) return { label: "Good", cls: "text-warning" };
    if (bits < 112) return { label: "Strong", cls: "text-primary" };
    return { label: "Excellent", cls: "text-success" };
  }

  const strength = $derived(stats ? strengthLabel(stats.average_password_strength) : { label: "…", cls: "text-muted-foreground" });
  const healthScore = $derived(
    stats
      ? Math.max(
          0,
          100 -
            (stats.weak_passwords / Math.max(stats.total_entries, 1)) * 30 -
            (stats.reused_passwords / Math.max(stats.total_entries, 1)) * 25 -
            (stats.old_passwords / Math.max(stats.total_entries, 1)) * 20 -
            (stats.expired_entries / Math.max(stats.total_entries, 1)) * 25,
        )
      : 0,
  );
  const healthScoreInt = $derived(stats ? Math.round(healthScore) : null);
  const healthTone = $derived(
    healthScoreInt === null
      ? "text-muted-foreground"
      : healthScoreInt >= 85
        ? "text-success"
        : healthScoreInt >= 60
          ? "text-warning"
          : "text-destructive",
  );
</script>

<div class="h-full overflow-auto bg-background">
  <div class="max-w-[1100px] mx-auto px-8 pt-8 pb-10 space-y-6">
    <!-- Hero header -->
    <header class="flex items-center gap-4">
      <div class="grid place-items-center size-12 rounded-xl bg-primary/10 text-primary shadow-xs">
        <Shield class="size-6" />
      </div>
      <div class="min-w-0 flex-1">
        <h1 class="text-[24px] font-semibold tracking-tight leading-tight">Home</h1>
        <p class="text-[13px] text-muted-foreground mt-0.5">
          Vault overview — health, items, and recommendations.
        </p>
      </div>
      {#if healthScoreInt !== null}
        <div class="text-right">
          <div class="text-[11px] text-muted-foreground uppercase tracking-wider">Vault health</div>
          <div class="text-[28px] font-semibold tracking-tight {healthTone}">{healthScoreInt}%</div>
        </div>
      {/if}
    </header>

    {#if hibpEnabled}
      <BreachedPasswordsCard
        {refreshTrigger}
        databasePath={appState.dbPath}
        isDirty={appState.isDirty}
        {onJumpToEntry}
      />
    {/if}

    <!-- Stat tiles -->
    <div class="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
      <div class="rounded-xl border bg-card p-4 shadow-xs">
        <div class="flex items-center justify-between">
          <span class="text-[11px] font-medium uppercase tracking-wider text-muted-foreground">Items</span>
          <KeyRound class="size-3.5 text-muted-foreground" />
        </div>
        <div class="text-[24px] font-semibold mt-1.5">{stats?.total_entries ?? "—"}</div>
        <p class="text-[11px] text-muted-foreground mt-0.5">Across {stats?.total_groups ?? "—"} folders</p>
      </div>

      <div class="rounded-xl border bg-card p-4 shadow-xs">
        <div class="flex items-center justify-between">
          <span class="text-[11px] font-medium uppercase tracking-wider text-muted-foreground">Strength</span>
          <TrendingUp class="size-3.5 text-muted-foreground" />
        </div>
        <div class="text-[24px] font-semibold mt-1.5 {strength.cls}">{strength.label}</div>
        <p class="text-[11px] text-muted-foreground mt-0.5">
          {stats ? Math.round(stats.average_password_strength) : "—"} bits average
        </p>
      </div>

      <div class="rounded-xl border bg-card p-4 shadow-xs">
        <div class="flex items-center justify-between">
          <span class="text-[11px] font-medium uppercase tracking-wider text-muted-foreground">Favorites</span>
          <Star class="size-3.5 text-muted-foreground" />
        </div>
        <div class="text-[24px] font-semibold mt-1.5">{stats?.favorite_entries ?? "—"}</div>
        <p class="text-[11px] text-muted-foreground mt-0.5">Marked for quick access</p>
      </div>

      <div class="rounded-xl border bg-card p-4 shadow-xs">
        <div class="flex items-center justify-between">
          <span class="text-[11px] font-medium uppercase tracking-wider text-muted-foreground">Issues</span>
          <AlertTriangle class="size-3.5 text-muted-foreground" />
        </div>
        <div class="text-[24px] font-semibold mt-1.5">
          {stats ? stats.weak_passwords + stats.reused_passwords + stats.old_passwords + stats.expired_entries : "—"}
        </div>
        <p class="text-[11px] text-muted-foreground mt-0.5">Across all items</p>
      </div>
    </div>

    <div class="grid gap-4 lg:grid-cols-2">
      <Card>
        <CardHeader>
          <CardTitle>Security issues</CardTitle>
          <CardDescription>Items that need attention</CardDescription>
        </CardHeader>
        <CardContent class="space-y-3">
          <div class="flex items-center justify-between rounded-md px-3 py-2 hover:bg-accent/40 transition-colors">
            <div class="flex items-center gap-2">
              <AlertTriangle class="size-4 text-destructive" />
              <span class="text-[13px] font-medium">Weak passwords</span>
            </div>
            <span class="text-[15px] font-semibold tabular-nums {stats && stats.weak_passwords > 0 ? 'text-destructive' : 'text-success'}">
              {stats?.weak_passwords ?? "—"}
            </span>
          </div>
          <div class="flex items-center justify-between rounded-md px-3 py-2 hover:bg-accent/40 transition-colors">
            <div class="flex items-center gap-2">
              <Copy class="size-4 text-warning" />
              <span class="text-[13px] font-medium">Reused passwords</span>
            </div>
            <span class="text-[15px] font-semibold tabular-nums {stats && stats.reused_passwords > 0 ? 'text-warning' : 'text-success'}">
              {stats?.reused_passwords ?? "—"}
            </span>
          </div>
          <div class="flex items-center justify-between rounded-md px-3 py-2 hover:bg-accent/40 transition-colors">
            <div class="flex items-center gap-2">
              <Clock class="size-4 text-warning" />
              <span class="text-[13px] font-medium">Old passwords</span>
            </div>
            <span class="text-[15px] font-semibold tabular-nums {stats && stats.old_passwords > 0 ? 'text-warning' : 'text-success'}">
              {stats?.old_passwords ?? "—"}
            </span>
          </div>
          <div class="flex items-center justify-between rounded-md px-3 py-2 hover:bg-accent/40 transition-colors">
            <div class="flex items-center gap-2">
              <AlertTriangle class="size-4 text-destructive" />
              <span class="text-[13px] font-medium">Expired entries</span>
            </div>
            <span class="text-[15px] font-semibold tabular-nums {stats && stats.expired_entries > 0 ? 'text-destructive' : 'text-success'}">
              {stats?.expired_entries ?? "—"}
            </span>
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Recommendations</CardTitle>
          <CardDescription>What to do next</CardDescription>
        </CardHeader>
        <CardContent class="space-y-3">
          {#if !stats}
            <div class="text-[13px] text-muted-foreground">Loading recommendations…</div>
          {:else if stats.weak_passwords === 0 && stats.reused_passwords === 0 && stats.old_passwords === 0 && stats.expired_entries === 0}
            <div class="flex items-start gap-2.5 text-[13px] rounded-md p-3 bg-success/5 border border-success/15">
              <Shield class="size-4 text-success mt-0.5 shrink-0" />
              <div>
                <p class="font-medium text-success">All clear</p>
                <p class="text-muted-foreground text-[12px]">No security issues detected.</p>
              </div>
            </div>
          {:else}
            {#if stats.weak_passwords > 0}
              <div class="flex items-start gap-2.5 text-[13px]">
                <AlertTriangle class="size-4 text-destructive mt-0.5 shrink-0" />
                <div>
                  <p class="font-medium">Update weak passwords</p>
                  <p class="text-muted-foreground text-[12px]">{stats.weak_passwords} with less than 40 bits of entropy</p>
                </div>
              </div>
            {/if}
            {#if stats.reused_passwords > 0}
              <div class="flex items-start gap-2.5 text-[13px]">
                <Copy class="size-4 text-warning mt-0.5 shrink-0" />
                <div>
                  <p class="font-medium">Replace reused passwords</p>
                  <p class="text-muted-foreground text-[12px]">{stats.reused_passwords} used in multiple items</p>
                </div>
              </div>
            {/if}
            {#if stats.old_passwords > 0}
              <div class="flex items-start gap-2.5 text-[13px]">
                <Clock class="size-4 text-warning mt-0.5 shrink-0" />
                <div>
                  <p class="font-medium">Refresh old passwords</p>
                  <p class="text-muted-foreground text-[12px]">{stats.old_passwords} older than 90 days</p>
                </div>
              </div>
            {/if}
            {#if stats.expired_entries > 0}
              <div class="flex items-start gap-2.5 text-[13px]">
                <AlertTriangle class="size-4 text-destructive mt-0.5 shrink-0" />
                <div>
                  <p class="font-medium">Review expired entries</p>
                  <p class="text-muted-foreground text-[12px]">{stats.expired_entries} past expiration</p>
                </div>
              </div>
            {/if}
          {/if}
        </CardContent>
      </Card>
    </div>
  </div>
</div>
