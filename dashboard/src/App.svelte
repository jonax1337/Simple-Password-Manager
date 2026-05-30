<script lang="ts">
  import { DashboardApi, deriveAuthHash, type VaultSummary } from "./api";

  type Phase = "login" | "loading" | "list";

  const STORAGE_TOKEN = "dashboard:token";
  const STORAGE_SERVER = "dashboard:server";

  let phase = $state<Phase>("login");
  let serverUrl = $state(localStorage.getItem(STORAGE_SERVER) || window.location.origin);
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
      // Stale token → bounce back to login.
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
      error = String(e);
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

<header class="header">
  <h1>🔐 Password Wallet</h1>
  {#if phase === "list"}
    <span class="meta">{email || "signed in"}</span>
    <button class="button-outline" onclick={handleLogout}>Sign out</button>
  {/if}
</header>

{#if phase === "login"}
  <div class="card">
    <h2 style="margin-bottom:1rem; font-size:1rem;">Sign in to your vault server</h2>
    <div class="field">
      <label for="srv">Server URL</label>
      <input id="srv" class="input" bind:value={serverUrl} placeholder="https://vault.example.com" />
    </div>
    <div class="field">
      <label for="email">Username</label>
      <input id="email" class="input" bind:value={email} placeholder="you@example.com" />
    </div>
    <div class="field">
      <label for="pw">Master password</label>
      <input
        id="pw"
        class="input"
        type="password"
        bind:value={password}
        onkeydown={(e) => e.key === "Enter" && handleLogin()}
      />
    </div>
    <button class="button" onclick={handleLogin} disabled={!email || !password || !serverUrl}>
      Sign in
    </button>
    {#if error}
      <p class="error">{error}</p>
    {/if}
    <p style="margin-top:1.25rem; font-size:0.75rem; color:rgb(100 116 139);">
      The dashboard cannot view vault contents — only the desktop app can.
      Argon2id runs locally; your password never leaves the browser.
    </p>
  </div>
{:else if phase === "loading"}
  <div class="card">
    <div class="empty">Loading…</div>
  </div>
{:else}
  <div class="card" style="padding:0;">
    {#each vaults as v (v.id)}
      <div class="vault-row">
        <div style="display:flex; flex-direction:column; gap:0.125rem; flex:1;">
          <div class="name">{v.name}</div>
          <div class="updated">Updated {formatDate(v.updated_at)}</div>
        </div>
        <span class="role">{v.role}</span>
        {#if v.role === "owner"}
          <button class="button-outline" onclick={() => handleRename(v)}>Rename</button>
          <button class="button-outline" onclick={() => handleDelete(v)}>Delete</button>
        {/if}
      </div>
    {:else}
      <div class="empty">No vaults yet. Use the desktop app to create one.</div>
    {/each}
  </div>
  {#if error}
    <p class="error">{error}</p>
  {/if}
{/if}
