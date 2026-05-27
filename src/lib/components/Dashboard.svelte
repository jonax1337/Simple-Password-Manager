<script lang="ts">
  import { Card, CardHeader, CardTitle, CardDescription, CardContent, toast } from "$lib/ui";
  import { Shield, Key, AlertTriangle, Clock, Star, TrendingUp, Copy } from "@lucide/svelte";
  import { getDashboardStats, type DashboardStats } from "$lib/tauri";

  type Props = { refreshTrigger?: number };
  let { refreshTrigger = 0 }: Props = $props();

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
</script>

<div class="h-full overflow-auto">
  <div class="space-y-4 p-4">
    <div>
      <h1 class="text-3xl font-bold tracking-tight">Dashboard</h1>
      <p class="text-muted-foreground mt-1">Overview of your password database health and statistics</p>
    </div>

    <div class="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
      <Card>
        <CardHeader class="flex flex-row items-center justify-between space-y-0 pb-2">
          <CardTitle class="text-sm font-medium">Total Entries</CardTitle>
          <Key class="h-4 w-4 text-muted-foreground" />
        </CardHeader>
        <CardContent>
          <div class="text-2xl font-bold">{stats?.total_entries ?? "…"}</div>
          <p class="text-xs text-muted-foreground mt-1">Across {stats?.total_groups ?? "…"} groups</p>
        </CardContent>
      </Card>

      <Card>
        <CardHeader class="flex flex-row items-center justify-between space-y-0 pb-2">
          <CardTitle class="text-sm font-medium">Health Score</CardTitle>
          <Shield class="h-4 w-4 text-muted-foreground" />
        </CardHeader>
        <CardContent>
          <div class="text-2xl font-bold">{stats ? Math.round(healthScore) : "…"}%</div>
          <p class="text-xs text-muted-foreground mt-1">Database security rating</p>
        </CardContent>
      </Card>

      <Card>
        <CardHeader class="flex flex-row items-center justify-between space-y-0 pb-2">
          <CardTitle class="text-sm font-medium">Avg. Strength</CardTitle>
          <TrendingUp class="h-4 w-4 text-muted-foreground" />
        </CardHeader>
        <CardContent>
          <div class="text-2xl font-bold {strength.cls}">{strength.label}</div>
          <p class="text-xs text-muted-foreground mt-1">
            {stats ? Math.round(stats.average_password_strength) : "…"} bits entropy
          </p>
        </CardContent>
      </Card>

      <Card>
        <CardHeader class="flex flex-row items-center justify-between space-y-0 pb-2">
          <CardTitle class="text-sm font-medium">Favorites</CardTitle>
          <Star class="h-4 w-4 text-muted-foreground" />
        </CardHeader>
        <CardContent>
          <div class="text-2xl font-bold">{stats?.favorite_entries ?? "…"}</div>
          <p class="text-xs text-muted-foreground mt-1">Marked as favorite</p>
        </CardContent>
      </Card>
    </div>

    <div class="grid gap-4 md:grid-cols-2">
      <Card>
        <CardHeader>
          <CardTitle>Security Issues</CardTitle>
          <CardDescription>Passwords that need attention</CardDescription>
        </CardHeader>
        <CardContent class="space-y-4">
          <div class="flex items-center justify-between">
            <div class="flex items-center gap-2">
              <AlertTriangle class="h-4 w-4 text-destructive" />
              <span class="text-sm font-medium">Weak Passwords</span>
            </div>
            <span class="text-lg font-bold {stats && stats.weak_passwords > 0 ? 'text-destructive' : 'text-success'}">
              {stats?.weak_passwords ?? "…"}
            </span>
          </div>
          <div class="flex items-center justify-between">
            <div class="flex items-center gap-2">
              <Copy class="h-4 w-4 text-warning" />
              <span class="text-sm font-medium">Reused Passwords</span>
            </div>
            <span class="text-lg font-bold {stats && stats.reused_passwords > 0 ? 'text-warning' : 'text-success'}">
              {stats?.reused_passwords ?? "…"}
            </span>
          </div>
          <div class="flex items-center justify-between">
            <div class="flex items-center gap-2">
              <Clock class="h-4 w-4 text-warning" />
              <span class="text-sm font-medium">Old Passwords</span>
            </div>
            <span class="text-lg font-bold {stats && stats.old_passwords > 0 ? 'text-warning' : 'text-success'}">
              {stats?.old_passwords ?? "…"}
            </span>
          </div>
          <div class="flex items-center justify-between">
            <div class="flex items-center gap-2">
              <AlertTriangle class="h-4 w-4 text-destructive" />
              <span class="text-sm font-medium">Expired Entries</span>
            </div>
            <span class="text-lg font-bold {stats && stats.expired_entries > 0 ? 'text-destructive' : 'text-success'}">
              {stats?.expired_entries ?? "…"}
            </span>
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Recommendations</CardTitle>
          <CardDescription>Actions to improve your security</CardDescription>
        </CardHeader>
        <CardContent class="space-y-3">
          {#if !stats}
            <div class="text-sm text-muted-foreground">Loading recommendations…</div>
          {:else if stats.weak_passwords === 0 && stats.reused_passwords === 0 && stats.old_passwords === 0 && stats.expired_entries === 0}
            <div class="flex items-start gap-2 text-sm">
              <Shield class="h-4 w-4 text-success mt-0.5 shrink-0" />
              <div>
                <p class="font-medium text-success">All good!</p>
                <p class="text-muted-foreground text-xs">No security issues detected</p>
              </div>
            </div>
          {:else}
            {#if stats.weak_passwords > 0}
              <div class="flex items-start gap-2 text-sm">
                <AlertTriangle class="h-4 w-4 text-destructive mt-0.5 shrink-0" />
                <div>
                  <p class="font-medium">Update weak passwords</p>
                  <p class="text-muted-foreground text-xs">{stats.weak_passwords} with less than 40 bits of entropy</p>
                </div>
              </div>
            {/if}
            {#if stats.reused_passwords > 0}
              <div class="flex items-start gap-2 text-sm">
                <Copy class="h-4 w-4 text-warning mt-0.5 shrink-0" />
                <div>
                  <p class="font-medium">Replace reused passwords</p>
                  <p class="text-muted-foreground text-xs">{stats.reused_passwords} used multiple times</p>
                </div>
              </div>
            {/if}
            {#if stats.old_passwords > 0}
              <div class="flex items-start gap-2 text-sm">
                <Clock class="h-4 w-4 text-warning mt-0.5 shrink-0" />
                <div>
                  <p class="font-medium">Refresh old passwords</p>
                  <p class="text-muted-foreground text-xs">{stats.old_passwords} older than 90 days</p>
                </div>
              </div>
            {/if}
            {#if stats.expired_entries > 0}
              <div class="flex items-start gap-2 text-sm">
                <AlertTriangle class="h-4 w-4 text-destructive mt-0.5 shrink-0" />
                <div>
                  <p class="font-medium">Review expired entries</p>
                  <p class="text-muted-foreground text-xs">{stats.expired_entries} past expiration</p>
                </div>
              </div>
            {/if}
          {/if}
        </CardContent>
      </Card>
    </div>
  </div>
</div>
