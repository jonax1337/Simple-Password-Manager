<script lang="ts">
  import { DashboardApi, deriveAuthHash, type VaultSummary } from "./api";

  type Phase = "login" | "loading" | "list";

  const STORAGE_TOKEN = "dashboard:token";
  const STORAGE_SERVER = "dashboard:server";

  const isViteDev =
    typeof window !== "undefined" &&
    (window.location.port === "5173" || window.location.port === "1420");

  let phase = $state<Phase>("login");
  let serverUrl = $state(
    localStorage.getItem(STORAGE_SERVER) ||
      (isViteDev ? "http://localhost:8090" : window.location.origin),
  );
  let email = $state("");
  let password = $state("");
  let error = $state("");

  let vaults = $state<VaultSummary[]>([]);
  let api: DashboardApi | null = null;

  $effect(() => {
    const token = localStorage.getItem(STORAGE_TOKEN);
    if (token) {
      api = new DashboardApi(serverUrl, token);
      void loadVaults();
    }
  });

  async function loadVaults() {
    if (!api) return;
    phase = "loading";
    try {
      vaults = await api.listVaults();
      phase = "list";
    } catch (e) {
      localStorage.removeItem(STORAGE_TOKEN);
      api = null;
      phase = "login";
      error = `Session expired: ${e}`;
    }
  }

  async function handleLogin() {
    if (!serverUrl || !email || !password) {
      error = "All fields required.";
      return;
    }
    error = "";
    phase = "loading";
    try {
      const client = new DashboardApi(serverUrl);
      const params = await client.kdfParams(email);
      const authHash = await deriveAuthHash(
        password,
        params.kdf_salt_b64,
        params.wrapped_master_key_b64,
      );
      const resp = await client.login(email, authHash);
      localStorage.setItem(STORAGE_TOKEN, resp.token);
      localStorage.setItem(STORAGE_SERVER, serverUrl);
      api = client;
      password = "";
      await loadVaults();
    } catch (e) {
      const raw = String(e);
      if (raw.includes("404")) {
        error = `No account named "${email}" on this server. The username is case-sensitive and must match exactly what you used in the desktop app.`;
      } else if (raw.includes("401")) {
        error = "Wrong master password.";
      } else if (raw.includes("Failed to fetch") || raw.includes("NetworkError")) {
        error = `Can't reach ${serverUrl}. Is the server running?`;
      } else {
        error = raw;
      }
      phase = "login";
    }
  }

  function handleLogout() {
    localStorage.removeItem(STORAGE_TOKEN);
    api = null;
    vaults = [];
    phase = "login";
  }

  async function handleRename(v: VaultSummary) {
    const newName = window.prompt("New vault name", v.name);
    if (!newName || newName === v.name) return;
    try {
      await api!.renameVault(v.id, newName);
      await loadVaults();
    } catch (e) {
      error = String(e);
    }
  }

  async function handleDelete(v: VaultSummary) {
    if (!window.confirm(`Delete "${v.name}"? This cannot be undone — make sure you have a local copy.`)) return;
    try {
      await api!.deleteVault(v.id);
      await loadVaults();
    } catch (e) {
      error = String(e);
    }
  }

  function formatDate(seconds: number) {
    return new Date(seconds * 1000).toLocaleString();
  }
</script>

<div class="min-h-full bg-background text-foreground flex flex-col">
  <!-- Header -->
  <header class="border-b border-border-subtle bg-sidebar/80 backdrop-blur">
    <div class="max-w-3xl mx-auto px-6 h-14 flex items-center gap-3">
      <div class="grid place-items-center size-8 rounded-md bg-primary/15 text-primary">
        <!-- Shield icon, matches the app icon style -->
        <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="size-4">
          <path d="M20 13c0 5-3.5 7.5-7.66 8.95a1 1 0 0 1-.67-.01C7.5 20.5 4 18 4 13V6a1 1 0 0 1 1-1c2 0 4.5-1.2 6.24-2.72a1.17 1.17 0 0 1 1.52 0C14.51 3.81 17 5 19 5a1 1 0 0 1 1 1z" />
        </svg>
      </div>
      <h1 class="text-sm font-semibold tracking-tight flex-1">Password Wallet</h1>
      {#if phase === "list"}
        <span class="text-xs text-muted-foreground truncate hidden sm:inline">{email || "signed in"}</span>
        <button
          type="button"
          onclick={handleLogout}
          class="inline-flex items-center h-8 px-3 rounded-md text-xs font-medium border border-border bg-background hover:bg-accent/40 transition-colors"
        >
          Sign out
        </button>
      {/if}
    </div>
  </header>

  <main class="flex-1 max-w-3xl w-full mx-auto px-6 py-8">
    {#if phase === "login"}
      <div class="max-w-md mx-auto">
        <div class="space-y-1 mb-6">
          <h2 class="text-xl font-semibold tracking-tight">Sign in</h2>
          <p class="text-sm text-muted-foreground">
            Use the same credentials as your desktop app. Argon2id runs in the browser — your master password never leaves this tab.
          </p>
        </div>

        <div class="rounded-xl border border-border bg-card p-6 shadow-sm space-y-4">
          <div class="space-y-1.5">
            <label for="srv" class="text-xs font-medium">Server URL</label>
            <input
              id="srv"
              type="url"
              bind:value={serverUrl}
              placeholder="https://vault.example.com"
              class="w-full h-9 px-3 rounded-md border border-input bg-background text-sm shadow-xs focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2 focus:ring-offset-background"
            />
          </div>
          <div class="space-y-1.5">
            <label for="email" class="text-xs font-medium">Username</label>
            <input
              id="email"
              type="email"
              bind:value={email}
              placeholder="you@example.com"
              autocomplete="username"
              class="w-full h-9 px-3 rounded-md border border-input bg-background text-sm shadow-xs focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2 focus:ring-offset-background"
            />
          </div>
          <div class="space-y-1.5">
            <label for="pw" class="text-xs font-medium">Master password</label>
            <input
              id="pw"
              type="password"
              bind:value={password}
              autocomplete="current-password"
              onkeydown={(e) => e.key === "Enter" && handleLogin()}
              class="w-full h-9 px-3 rounded-md border border-input bg-background text-sm shadow-xs focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2 focus:ring-offset-background"
            />
          </div>

          <button
            type="button"
            onclick={handleLogin}
            disabled={!email || !password || !serverUrl}
            class="w-full inline-flex items-center justify-center h-9 px-4 rounded-md bg-primary text-primary-foreground text-sm font-medium shadow-xs hover:bg-primary/90 disabled:opacity-50 disabled:cursor-not-allowed active:scale-[0.98] transition-all"
          >
            Sign in
          </button>

          {#if error}
            <p class="text-xs text-destructive break-words">{error}</p>
          {/if}
        </div>

        <p class="text-2xs text-muted-foreground mt-4 text-center">
          The dashboard can list, rename, and delete vaults — it cannot view their contents.
        </p>
      </div>
    {:else if phase === "loading"}
      <div class="rounded-xl border border-border bg-card p-12 text-center text-sm text-muted-foreground">
        Loading…
      </div>
    {:else}
      <div class="space-y-3">
        <div class="flex items-baseline justify-between px-1">
          <h2 class="text-lg font-semibold tracking-tight">Vaults</h2>
          <span class="text-2xs uppercase tracking-wider text-muted-foreground">
            {vaults.length} total
          </span>
        </div>

        <div class="rounded-xl border border-border bg-card overflow-hidden">
          {#each vaults as v (v.id)}
            <div class="flex items-center gap-3 px-4 py-3 border-b border-border-subtle last:border-b-0 hover:bg-accent/20 transition-colors">
              <div class="grid place-items-center size-8 rounded-md bg-primary/10 text-primary shrink-0">
                <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="size-4">
                  <ellipse cx="12" cy="5" rx="9" ry="3" />
                  <path d="M3 5v14a9 3 0 0 0 18 0V5" />
                  <path d="M3 12a9 3 0 0 0 18 0" />
                </svg>
              </div>
              <div class="flex-1 min-w-0">
                <div class="text-sm font-medium truncate">{v.name}</div>
                <div class="text-2xs text-muted-foreground">
                  Updated {formatDate(v.updated_at)}
                </div>
              </div>
              <span
                class="text-2xs uppercase tracking-wider text-muted-foreground px-2 py-0.5 rounded-full border border-border-subtle bg-muted/40"
              >
                {v.role}
              </span>
              {#if v.role === "owner"}
                <div class="flex gap-1">
                  <button
                    type="button"
                    onclick={() => handleRename(v)}
                    class="h-8 px-2.5 rounded-md text-xs font-medium border border-border bg-background hover:bg-accent/40 transition-colors"
                  >
                    Rename
                  </button>
                  <button
                    type="button"
                    onclick={() => handleDelete(v)}
                    class="h-8 px-2.5 rounded-md text-xs font-medium border border-border bg-background hover:bg-destructive/10 hover:text-destructive hover:border-destructive/40 transition-colors"
                  >
                    Delete
                  </button>
                </div>
              {/if}
            </div>
          {:else}
            <div class="px-4 py-12 text-center text-sm text-muted-foreground">
              No vaults yet. Use the desktop app to create one.
            </div>
          {/each}
        </div>

        {#if error}
          <p class="text-xs text-destructive break-words px-1">{error}</p>
        {/if}
      </div>
    {/if}
  </main>

  <footer class="border-t border-border-subtle py-3">
    <div class="max-w-3xl mx-auto px-6 text-2xs text-muted-foreground flex items-center gap-2">
      <span class="size-1.5 rounded-full bg-success/70"></span>
      End-to-end encrypted · vault contents never leave your devices
    </div>
  </footer>
</div>
