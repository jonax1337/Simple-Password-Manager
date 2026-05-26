import { useEffect, useState } from "react";
import {
  Check,
  Copy,
  Eye,
  EyeOff,
  Globe,
  KeyRound,
  Lock,
  RefreshCw,
  Search,
  Sparkles,
  X,
} from "lucide-react";
import {
  createEntry,
  fetchAllEntries,
  fetchEntriesForDomain,
  fetchPassword,
  fetchStatus,
  fetchTotp,
  focusApp,
  generatePassword,
} from "./bridge-client";
import { useActiveDomain } from "./useActiveDomain";
import {
  consumePendingCapture,
  observePendingCapture,
  type PendingCapture,
} from "./pending-capture";
import type { EntrySummary, StatusResult } from "../common/bridge";
import {
  Button,
  Card,
  Checkbox,
  Input,
  Label,
  LetterBadge,
  Separator,
  Slider,
  cn,
} from "./ui";

type View = "this-site" | "all" | "generator";

export function Popup() {
  const [status, setStatus] = useState<StatusResult | null>(null);
  const [statusError, setStatusError] = useState<string | null>(null);
  const { domain, tabId, loading: domainLoading } = useActiveDomain();
  const [view, setView] = useState<View>("this-site");
  const [entries, setEntries] = useState<EntrySummary[] | null>(null);
  const [entriesError, setEntriesError] = useState<string | null>(null);
  const [filter, setFilter] = useState("");
  const [expandedId, setExpandedId] = useState<string | null>(null);
  const [pending, setPending] = useState<PendingCapture | null>(null);

  useEffect(() => {
    fetchStatus().then((res) => {
      if (res.ok && res.data) setStatus(res.data);
      else setStatusError(res.error ?? "Could not reach Simple Password Manager");
    });
  }, []);

  useEffect(() => {
    return observePendingCapture(setPending);
  }, []);

  useEffect(() => {
    if (!domainLoading && !domain) {
      setView("all");
    }
  }, [domainLoading, domain]);

  useEffect(() => {
    if (!status?.unlocked) return;
    if (view === "generator") return;
    setEntries(null);
    setEntriesError(null);
    const promise =
      view === "this-site" && domain
        ? fetchEntriesForDomain(domain)
        : fetchAllEntries();
    promise.then((res) => {
      if (res.ok && res.data) setEntries(res.data);
      else setEntriesError(res.error ?? "Failed to load entries");
    });
  }, [view, domain, status?.unlocked]);

  if (statusError) return <SetupScreen message={statusError} />;
  if (!status) return <LoadingScreen />;
  if (!status.unlocked) return <LockedScreen domain={domain} />;

  const filtered =
    entries?.filter((e) => {
      if (!filter.trim()) return true;
      const hay = `${e.title} ${e.username} ${e.url}`.toLowerCase();
      return hay.includes(filter.toLowerCase());
    }) ?? null;

  return (
    <div className="flex flex-col">
      <Header
        dbName={status.database_name}
        domain={domain}
        loading={domainLoading}
      />

      <Tabs view={view} setView={setView} hasDomain={!!domain} />

      {pending && (
        <SaveBanner
          pending={pending}
          onSaved={() => setPending(null)}
          onDismiss={() => {
            void consumePendingCapture();
            setPending(null);
          }}
        />
      )}

      {view !== "generator" && (
        <div className="border-b bg-background px-3 py-3">
          <div className="relative">
            <Search className="pointer-events-none absolute left-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-muted-foreground" />
            <Input
              value={filter}
              onChange={(e) => setFilter(e.target.value)}
              placeholder="Search entries…"
              className="pl-8 h-9"
              autoFocus
            />
          </div>
        </div>
      )}

      <div className="min-h-[180px] max-h-[380px] overflow-y-auto">
        {view === "generator" ? (
          <Generator />
        ) : (
          <EntryList
            entries={filtered}
            error={entriesError}
            expandedId={expandedId}
            setExpandedId={setExpandedId}
            tabId={tabId}
            emptyMessage={
              view === "this-site" && domain
                ? `No saved logins for ${domain}`
                : "No entries"
            }
          />
        )}
      </div>
    </div>
  );
}

// -------------------------- header --------------------------

function Header(props: {
  dbName: string | null;
  domain: string | null;
  loading: boolean;
}) {
  return (
    <header className="flex items-center gap-2.5 border-b bg-background px-3 py-3">
      <div className="flex h-8 w-8 shrink-0 items-center justify-center rounded-md bg-primary/10">
        <KeyRound className="h-4 w-4 text-primary" />
      </div>
      <div className="min-w-0 flex-1">
        <div className="truncate text-sm font-semibold leading-tight">
          {props.dbName ?? "Simple Password Manager"}
        </div>
        <div className="truncate text-xs text-muted-foreground leading-tight mt-0.5">
          {props.loading
            ? "Connecting…"
            : props.domain ?? "No active site"}
        </div>
      </div>
    </header>
  );
}

// -------------------------- tabs --------------------------

function Tabs(props: {
  view: View;
  setView: (v: View) => void;
  hasDomain: boolean;
}) {
  const tab = (
    id: View,
    label: string,
    Icon: React.ComponentType<{ className?: string }>,
    disabled = false,
  ) => {
    const active = props.view === id;
    return (
      <button
        disabled={disabled}
        onClick={() => props.setView(id)}
        className={cn(
          "relative flex flex-1 items-center justify-center gap-1.5 px-3 py-2 text-xs font-medium transition-colors",
          active
            ? "text-foreground"
            : "text-muted-foreground hover:text-foreground",
          disabled && "opacity-40 cursor-not-allowed",
        )}
      >
        <Icon className="h-3.5 w-3.5" />
        <span>{label}</span>
        {active && (
          <span className="absolute inset-x-0 -bottom-px h-0.5 bg-primary" />
        )}
      </button>
    );
  };

  return (
    <nav className="flex border-b bg-background">
      {tab("this-site", "This site", Globe, !props.hasDomain)}
      {tab("all", "All", KeyRound)}
      {tab("generator", "Generate", Sparkles)}
    </nav>
  );
}

// -------------------------- save banner --------------------------

function SaveBanner(props: {
  pending: PendingCapture;
  onSaved: () => void;
  onDismiss: () => void;
}) {
  const [title, setTitle] = useState<string>(props.pending.domain);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const isSignup = props.pending.intent === "signup";

  async function save() {
    setBusy(true);
    setError(null);
    const res = await createEntry({
      title: title.trim() || props.pending.domain,
      username: props.pending.username,
      password: props.pending.password,
      url: props.pending.url,
    });
    setBusy(false);
    if (res.ok) {
      await consumePendingCapture();
      props.onSaved();
    } else {
      setError(res.error ?? "Could not save");
    }
  }

  return (
    <div className="border-b bg-muted/30 px-3 py-3">
      <div className="mb-2 flex items-center justify-between">
        <div className="flex items-center gap-2 text-sm font-medium">
          <Sparkles className="h-3.5 w-3.5 text-primary" />
          {isSignup ? "Save this new account?" : "Save this login?"}
        </div>
        <Button
          variant="ghost"
          size="icon"
          onClick={props.onDismiss}
          title="Discard"
          className="h-6 w-6"
        >
          <X className="h-3.5 w-3.5" />
        </Button>
      </div>
      <p className="mb-2 text-xs text-muted-foreground">
        <span className="font-medium text-foreground">{props.pending.domain}</span>
        {" · "}
        <span className="font-medium text-foreground">
          {props.pending.username || "(no username)"}
        </span>
      </p>
      <Input
        value={title}
        onChange={(e) => setTitle(e.target.value)}
        placeholder="Entry title"
        className="mb-2 h-8 text-xs"
      />
      <div className="flex gap-2">
        <Button onClick={save} disabled={busy} size="sm" className="flex-1">
          {busy ? "Saving…" : "Save to vault"}
        </Button>
        <Button variant="outline" onClick={props.onDismiss} size="sm">
          Not now
        </Button>
      </div>
      {error && <p className="mt-2 text-xs text-destructive">{error}</p>}
    </div>
  );
}

// -------------------------- entry list --------------------------

function EntryList(props: {
  entries: EntrySummary[] | null;
  error: string | null;
  expandedId: string | null;
  setExpandedId: (id: string | null) => void;
  tabId: number | null;
  emptyMessage: string;
}) {
  if (props.error) {
    return (
      <p className="p-4 text-center text-xs text-destructive">{props.error}</p>
    );
  }
  if (props.entries === null) {
    return (
      <p className="p-4 text-center text-xs text-muted-foreground">Loading…</p>
    );
  }
  if (props.entries.length === 0) {
    return (
      <div className="flex flex-col items-center gap-2 px-4 py-10 text-center">
        <Search className="h-5 w-5 text-muted-foreground/60" />
        <p className="text-xs text-muted-foreground max-w-[220px]">
          {props.emptyMessage}
        </p>
      </div>
    );
  }

  return (
    <ul className="divide-y">
      {props.entries.map((e) => (
        <EntryRow
          key={e.uuid}
          entry={e}
          expanded={props.expandedId === e.uuid}
          onToggle={() =>
            props.setExpandedId(props.expandedId === e.uuid ? null : e.uuid)
          }
          tabId={props.tabId}
        />
      ))}
    </ul>
  );
}

function EntryRow(props: {
  entry: EntrySummary;
  expanded: boolean;
  onToggle: () => void;
  tabId: number | null;
}) {
  const [details, setDetails] = useState<{
    username: string;
    password: string;
  } | null>(null);
  const [revealing, setRevealing] = useState(false);
  const [reveal, setReveal] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!props.expanded || details) return;
    setRevealing(true);
    fetchPassword(props.entry.uuid).then((res) => {
      setRevealing(false);
      if (res.ok && res.data) {
        setDetails({ username: res.data.username, password: res.data.password });
      } else {
        setError(res.error ?? "Could not load entry");
      }
    });
  }, [props.expanded, details, props.entry.uuid]);

  return (
    <li>
      <button
        onClick={props.onToggle}
        className={cn(
          "flex w-full items-center gap-3 px-3 py-2.5 text-left transition-colors hover:bg-accent",
          props.expanded && "bg-accent",
        )}
      >
        <LetterBadge title={props.entry.title || props.entry.url || "?"} size="md" />
        <div className="min-w-0 flex-1">
          <div className="truncate text-sm font-medium">
            {props.entry.title || "(untitled)"}
          </div>
          <div className="truncate text-xs text-muted-foreground">
            {props.entry.username || "—"}
          </div>
        </div>
      </button>

      {props.expanded && (
        <div className="bg-muted/40 px-3 pb-3 pt-2">
          {error ? (
            <p className="px-1 py-2 text-xs text-destructive">{error}</p>
          ) : revealing ? (
            <p className="px-1 py-2 text-xs text-muted-foreground">
              Decrypting…
            </p>
          ) : details ? (
            <div className="space-y-2.5">
              <FieldRow
                label="Username"
                value={details.username || "—"}
                copyable={!!details.username}
              />
              <FieldRow
                label="Password"
                value={details.password}
                copyable
                hidden={!reveal}
                rightSlot={
                  <Button
                    variant="outline"
                    size="icon"
                    onClick={() => setReveal((v) => !v)}
                    aria-label={reveal ? "Hide password" : "Show password"}
                    title={reveal ? "Hide password" : "Show password"}
                  >
                    {reveal ? (
                      <EyeOff className="h-3.5 w-3.5" />
                    ) : (
                      <Eye className="h-3.5 w-3.5" />
                    )}
                  </Button>
                }
              />
              <TotpField entryUuid={props.entry.uuid} />
              <FillButton
                entry={props.entry}
                username={details.username}
                password={details.password}
                tabId={props.tabId}
              />
            </div>
          ) : null}
        </div>
      )}
    </li>
  );
}

// -------------------------- field row + totp --------------------------

function FieldRow(props: {
  label: string;
  value: string;
  copyable?: boolean;
  hidden?: boolean;
  rightSlot?: React.ReactNode;
}) {
  const [copied, setCopied] = useState(false);

  async function copy() {
    if (!props.copyable) return;
    await navigator.clipboard.writeText(props.value);
    setCopied(true);
    setTimeout(() => setCopied(false), 1200);
  }

  const display = props.hidden
    ? "•".repeat(Math.min(props.value.length, 14))
    : props.value;

  return (
    <div>
      <Label className="mb-1 block">{props.label}</Label>
      <div className="flex items-stretch gap-1">
        <div className="flex flex-1 items-center rounded-md border bg-background px-2.5 py-1.5 text-xs font-mono break-all">
          {display || <span className="text-muted-foreground">—</span>}
        </div>
        {props.copyable && (
          <Button variant="outline" size="icon" onClick={copy} title="Copy">
            {copied ? (
              <Check className="h-3.5 w-3.5" />
            ) : (
              <Copy className="h-3.5 w-3.5" />
            )}
          </Button>
        )}
        {props.rightSlot}
      </div>
    </div>
  );
}

function TotpField(props: { entryUuid: string }) {
  const [totp, setTotp] = useState<
    | null
    | undefined
    | { code: string; period: number; expiresAt: number }
  >(null);
  const [totpError, setTotpError] = useState<string | null>(null);
  const [tick, setTick] = useState(0);
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    let cancelled = false;
    async function load() {
      const res = await fetchTotp(props.entryUuid);
      if (cancelled) return;
      if (res.ok && res.data) {
        setTotp({
          code: res.data.code,
          period: res.data.period,
          expiresAt: Date.now() + res.data.remaining_seconds * 1000,
        });
        setTotpError(null);
      } else if (res.status === 404) {
        setTotp(undefined);
        setTotpError(null);
      } else if (res.status === 422) {
        setTotp(undefined);
        setTotpError("Stored 2FA value is malformed.");
      } else {
        setTotp(undefined);
        setTotpError(res.error ?? "Could not read 2FA code");
      }
    }
    void load();
    return () => {
      cancelled = true;
    };
  }, [props.entryUuid]);

  useEffect(() => {
    if (!totp) return;
    const t = setInterval(() => setTick((n) => n + 1), 1000);
    return () => clearInterval(t);
  }, [totp]);

  useEffect(() => {
    if (!totp) return;
    const current = totp;
    if (Date.now() < current.expiresAt) return;
    fetchTotp(props.entryUuid).then((res) => {
      if (res.ok && res.data) {
        setTotp({
          code: res.data.code,
          period: res.data.period,
          expiresAt: Date.now() + res.data.remaining_seconds * 1000,
        });
      }
    });
  }, [tick, totp, props.entryUuid]);

  if (totpError) {
    return (
      <div>
        <Label className="mb-1 block">One-time code</Label>
        <p className="rounded-md border bg-background px-2.5 py-1.5 text-xs text-destructive">
          {totpError}
        </p>
      </div>
    );
  }
  if (totp === null || totp === undefined) return null;
  const current = totp;
  const remaining = Math.max(0, Math.ceil((current.expiresAt - Date.now()) / 1000));

  async function copy() {
    await navigator.clipboard.writeText(current.code);
    setCopied(true);
    setTimeout(() => setCopied(false), 1200);
  }

  return (
    <div>
      <Label className="mb-1 block">One-time code</Label>
      <div className="flex items-stretch gap-1">
        <div className="flex flex-1 items-center justify-between rounded-md border bg-background px-2.5 py-1.5">
          <span className="font-mono text-sm tabular-nums tracking-wider">
            {current.code.slice(0, 3)} {current.code.slice(3)}
          </span>
          <span className="text-[10px] font-medium tabular-nums text-muted-foreground">
            {remaining}s
          </span>
        </div>
        <Button variant="outline" size="icon" onClick={copy} title="Copy">
          {copied ? (
            <Check className="h-3.5 w-3.5" />
          ) : (
            <Copy className="h-3.5 w-3.5" />
          )}
        </Button>
      </div>
    </div>
  );
}

// -------------------------- fill button --------------------------

function FillButton(props: {
  entry: EntrySummary;
  username: string;
  password: string;
  tabId: number | null;
}) {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function fill() {
    if (props.tabId === null) {
      setError("No active tab to fill into");
      return;
    }
    setBusy(true);
    setError(null);
    try {
      try {
        await chrome.scripting.executeScript({
          target: { tabId: props.tabId },
          files: ["content.js"],
        });
      } catch {
        /* may fail on chrome:// */
      }
      await chrome.tabs.sendMessage(props.tabId, {
        kind: "fill",
        username: props.username,
        password: props.password,
      });
      window.close();
    } catch (e) {
      setError((e as Error).message);
    } finally {
      setBusy(false);
    }
  }

  return (
    <div>
      <Button onClick={fill} disabled={busy} size="sm" className="w-full">
        {busy ? "Filling…" : "Fill login form"}
      </Button>
      {error && <p className="mt-1 text-xs text-destructive">{error}</p>}
    </div>
  );
}

// -------------------------- generator --------------------------

function Generator() {
  const [length, setLength] = useState(20);
  const [uppercase, setUppercase] = useState(true);
  const [lowercase, setLowercase] = useState(true);
  const [numbers, setNumbers] = useState(true);
  const [symbols, setSymbols] = useState(true);
  const [password, setPassword] = useState("");
  const [busy, setBusy] = useState(false);
  const [copied, setCopied] = useState(false);

  async function gen() {
    setBusy(true);
    setCopied(false);
    const res = await generatePassword({
      length,
      uppercase,
      lowercase,
      numbers,
      symbols,
    });
    setBusy(false);
    if (res.ok && res.data) setPassword(res.data.password);
  }

  async function copy() {
    await navigator.clipboard.writeText(password);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  }

  useEffect(() => {
    void gen();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [length, uppercase, lowercase, numbers, symbols]);

  return (
    <div className="space-y-4 p-3">
      <div className="flex items-stretch gap-1">
        <div className="flex flex-1 items-center rounded-md border bg-background px-2.5 py-2 font-mono text-xs break-all">
          {password || <span className="text-muted-foreground">…</span>}
        </div>
        <Button
          variant="outline"
          size="icon"
          onClick={gen}
          disabled={busy}
          title="Regenerate"
        >
          <RefreshCw className={cn("h-3.5 w-3.5", busy && "animate-spin")} />
        </Button>
        <Button
          variant="outline"
          size="icon"
          onClick={copy}
          title="Copy"
          disabled={!password}
        >
          {copied ? (
            <Check className="h-3.5 w-3.5" />
          ) : (
            <Copy className="h-3.5 w-3.5" />
          )}
        </Button>
      </div>

      <div className="space-y-3">
        <div className="flex items-center justify-between">
          <Label>Length</Label>
          <span className="text-xs font-semibold tabular-nums text-foreground">
            {length}
          </span>
        </div>
        <Slider value={length} min={8} max={64} onChange={setLength} />
      </div>

      <Separator />

      <div className="space-y-2">
        <Label>Characters</Label>
        <div className="grid grid-cols-2 gap-2">
          <CharsetToggle label="ABC" checked={uppercase} onChange={setUppercase} />
          <CharsetToggle label="abc" checked={lowercase} onChange={setLowercase} />
          <CharsetToggle label="123" checked={numbers} onChange={setNumbers} />
          <CharsetToggle label="!@#" checked={symbols} onChange={setSymbols} />
        </div>
      </div>
    </div>
  );
}

function CharsetToggle(props: {
  label: string;
  checked: boolean;
  onChange: (v: boolean) => void;
}) {
  return (
    <button
      type="button"
      onClick={() => props.onChange(!props.checked)}
      className="flex items-center gap-2.5 rounded-md border bg-background px-2.5 py-2 text-xs transition-colors hover:bg-accent"
    >
      <Checkbox checked={props.checked} onCheckedChange={props.onChange} />
      <span className="font-mono">{props.label}</span>
    </button>
  );
}

// -------------------------- status screens --------------------------

function LoadingScreen() {
  return (
    <div className="flex flex-col items-center justify-center gap-3 p-10 text-center text-xs text-muted-foreground">
      <div className="h-5 w-5 animate-spin rounded-full border-2 border-muted-foreground/30 border-t-foreground" />
      Connecting…
    </div>
  );
}

function LockedScreen(props: { domain: string | null }) {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function unlock() {
    setBusy(true);
    setError(null);
    const res = await focusApp();
    setBusy(false);
    if (res.ok) {
      window.close();
    } else {
      setError(res.error ?? "Could not bring the app to the front");
    }
  }

  return (
    <Card className="m-3 p-5">
      <div className="flex flex-col items-center gap-3 text-center">
        <div className="flex h-10 w-10 items-center justify-center rounded-full bg-muted">
          <Lock className="h-4 w-4 text-muted-foreground" />
        </div>
        <div className="space-y-1">
          <div className="text-sm font-semibold">Vault is locked</div>
          <p className="text-xs text-muted-foreground">
            {props.domain
              ? `Unlock Simple Password Manager to autofill on ${props.domain}.`
              : "Unlock Simple Password Manager to use this extension."}
          </p>
        </div>
        <Button onClick={unlock} disabled={busy} className="w-full mt-1">
          {busy ? "Bringing app forward…" : "Open and unlock"}
        </Button>
        {error && <p className="text-xs text-destructive">{error}</p>}
      </div>
    </Card>
  );
}

function SetupScreen(props: { message: string }) {
  const extId = chrome.runtime.id;
  const [copied, setCopied] = useState(false);

  async function copyId() {
    await navigator.clipboard.writeText(extId);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  }

  return (
    <div className="space-y-3 p-3">
      <Card className="p-3">
        <div className="flex items-start gap-2.5">
          <Lock className="h-4 w-4 shrink-0 mt-0.5 text-muted-foreground" />
          <div className="space-y-1">
            <div className="text-sm font-semibold">Not connected</div>
            <p className="text-xs text-muted-foreground">{props.message}</p>
          </div>
        </div>
      </Card>

      <div>
        <Label className="mb-1.5 block">Your extension ID</Label>
        <div className="flex items-stretch gap-1">
          <div className="flex flex-1 items-center rounded-md border bg-background px-2.5 py-1.5 text-xs font-mono break-all">
            {extId}
          </div>
          <Button variant="outline" size="icon" onClick={copyId} title="Copy">
            {copied ? (
              <Check className="h-3.5 w-3.5" />
            ) : (
              <Copy className="h-3.5 w-3.5" />
            )}
          </Button>
        </div>
        <p className="mt-2 text-xs text-muted-foreground">
          Open Simple Password Manager →{" "}
          <span className="font-medium text-foreground">
            Settings → Application → Browser Extension
          </span>
          , paste the ID, click <em>Register for all detected browsers</em>,
          then restart this browser.
        </p>
      </div>
    </div>
  );
}
