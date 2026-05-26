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
  Input,
  Label,
  LetterBadge,
  Separator,
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

  // Watch for pending captures (login submissions waiting to be saved).
  useEffect(() => {
    return observePendingCapture(setPending);
  }, []);

  // Default tab: if we have no http(s) tab, go straight to All.
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
    <div className="flex flex-col bg-background text-foreground">
      <Header dbName={status.database_name} domain={domain} loading={domainLoading} />

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
        <div className="border-b border-border bg-background px-3 pb-3 pt-3">
          <div className="relative">
            <Search className="pointer-events-none absolute left-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-muted-foreground" />
            <Input
              value={filter}
              onChange={(e) => setFilter(e.target.value)}
              placeholder="Search entries…"
              className="pl-8 h-8"
              autoFocus
            />
          </div>
        </div>
      )}

      <div className="min-h-[140px] max-h-[360px] overflow-y-auto">
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

      <Tabs view={view} setView={setView} hasDomain={!!domain} />
    </div>
  );
}

// -------------------------- subcomponents --------------------------

function Header(props: {
  dbName: string | null;
  domain: string | null;
  loading: boolean;
}) {
  return (
    <header className="relative border-b border-border/70 bg-gradient-to-b from-card/60 to-card/20 backdrop-blur-sm px-4 py-3">
      <div className="flex items-center justify-between gap-2">
        <div className="flex min-w-0 items-center gap-2.5">
          <div className="relative flex h-8 w-8 items-center justify-center rounded-xl bg-gradient-to-br from-primary/25 to-primary/10 ring-1 ring-primary/20 shadow-sm shadow-primary/10">
            <KeyRound className="h-4 w-4 text-primary" />
            <div className="absolute inset-0 rounded-xl bg-gradient-to-tr from-transparent via-white/5 to-transparent pointer-events-none" />
          </div>
          <div className="min-w-0">
            <div className="truncate text-sm font-semibold tracking-tight">
              {props.dbName ?? "Simple Password Manager"}
            </div>
            <div className="truncate text-[11px] text-muted-foreground/90 font-medium">
              {props.loading
                ? "Connecting…"
                : props.domain ?? "no http(s) tab"}
            </div>
          </div>
        </div>
      </div>
    </header>
  );
}

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
          "relative flex flex-1 flex-col items-center justify-center gap-1 py-2.5 text-[11px] font-medium transition-all duration-200",
          active
            ? "text-primary"
            : "text-muted-foreground hover:text-foreground",
          disabled && "opacity-40 cursor-not-allowed",
        )}
      >
        <Icon
          className={cn(
            "h-3.5 w-3.5 transition-transform duration-200",
            active && "scale-110",
          )}
        />
        <span className="tracking-tight">{label}</span>
        {active && (
          <span className="absolute inset-x-3 top-0 h-[2px] rounded-b-full bg-gradient-to-r from-transparent via-primary to-transparent" />
        )}
      </button>
    );
  };

  return (
    <nav className="flex border-t border-border/70 bg-gradient-to-t from-card/60 to-card/20 backdrop-blur-sm">
      {tab("this-site", "This site", Globe, !props.hasDomain)}
      {tab("all", "All", KeyRound)}
      {tab("generator", "Generate", Sparkles)}
    </nav>
  );
}

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
      <p className="p-4 text-center text-sm text-rose-600 dark:text-rose-400">
        {props.error}
      </p>
    );
  }
  if (props.entries === null) {
    return (
      <p className="p-4 text-center text-sm text-muted-foreground">
        Loading…
      </p>
    );
  }
  if (props.entries.length === 0) {
    return (
      <div className="flex flex-col items-center gap-2 p-8 text-center text-sm text-muted-foreground">
        <Lock className="h-6 w-6" />
        {props.emptyMessage}
      </div>
    );
  }

  return (
    <ul className="divide-y divide-border">
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

  // Lazy-fetch the password when the row expands.
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
    <li className="spm-fade-in">
      <button
        onClick={props.onToggle}
        className={cn(
          "group flex w-full items-center gap-3 px-3.5 py-2.5 text-left transition-all duration-150",
          "hover:bg-accent/40 active:bg-accent/60",
          props.expanded && "bg-accent/30",
        )}
      >
        <LetterBadge title={props.entry.title || props.entry.url || "?"} size="md" />
        <div className="min-w-0 flex-1">
          <div className="truncate text-sm font-medium tracking-tight">
            {props.entry.title || "(untitled)"}
          </div>
          <div className="truncate text-xs text-muted-foreground/90">
            {props.entry.username || "—"}
          </div>
        </div>
        <div
          className={cn(
            "transition-transform duration-200 text-muted-foreground/60",
            props.expanded ? "rotate-90" : "group-hover:translate-x-0.5",
          )}
        >
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
            <polyline points="9 18 15 12 9 6" />
          </svg>
        </div>
      </button>

      {props.expanded && (
        <div className="spm-expand bg-gradient-to-b from-muted/30 to-muted/10 px-3.5 pb-3.5 pt-2 border-y border-border/40">
          {error ? (
            <p className="px-3 py-2 text-xs text-rose-600">{error}</p>
          ) : revealing ? (
            <p className="px-3 py-2 text-xs text-muted-foreground">Decrypting…</p>
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
                    variant="ghost"
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

function TotpField(props: { entryUuid: string }) {
  // null = not yet fetched. undefined = checked, no TOTP. object = present.
  const [totp, setTotp] = useState<
    | null
    | undefined
    | { code: string; period: number; expiresAt: number }
  >(null);
  const [totpError, setTotpError] = useState<string | null>(null);
  const [tick, setTick] = useState(0);
  const [copied, setCopied] = useState(false);

  // Initial + refresh fetch.
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
        // No TOTP configured — silently hide.
        setTotp(undefined);
        setTotpError(null);
      } else if (res.status === 422) {
        setTotp(undefined);
        setTotpError(
          "Stored 2FA value is malformed. Open the entry in the app and re-add your TOTP secret.",
        );
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

  // 1Hz tick to update the countdown / refetch when expired.
  useEffect(() => {
    if (!totp) return;
    const t = setInterval(() => setTick((n) => n + 1), 500);
    return () => clearInterval(t);
  }, [totp]);

  // When the current code expires, refetch.
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
        <p className="rounded-md border border-amber-500/40 bg-amber-500/5 px-2.5 py-1.5 text-xs text-amber-700 dark:text-amber-300">
          {totpError}
        </p>
      </div>
    );
  }
  if (totp === null || totp === undefined) return null;
  const current = totp;
  const remaining = Math.max(0, Math.ceil((current.expiresAt - Date.now()) / 1000));
  const ratio = current.period > 0 ? remaining / current.period : 0;
  const urgent = remaining <= 5;

  async function copy() {
    await navigator.clipboard.writeText(current.code);
    setCopied(true);
    setTimeout(() => setCopied(false), 1200);
  }

  return (
    <div>
      <Label className="mb-1 block">One-time code</Label>
      <div className="flex items-stretch gap-1">
        <div className="flex flex-1 items-center justify-between gap-2 rounded-md border border-input bg-background px-2.5 py-1.5">
          <span className="font-mono text-base tracking-widest">
            {current.code.slice(0, 3)} {current.code.slice(3)}
          </span>
          <CountdownRing
            ratio={ratio}
            urgent={urgent}
            label={String(remaining)}
          />
        </div>
        <Button variant="outline" size="icon" onClick={copy} title="Copy">
          {copied ? <Check className="h-3.5 w-3.5" /> : <Copy className="h-3.5 w-3.5" />}
        </Button>
      </div>
    </div>
  );
}

function CountdownRing(props: { ratio: number; urgent: boolean; label: string }) {
  const radius = 8;
  const circumference = 2 * Math.PI * radius;
  const dash = circumference * Math.max(0, Math.min(1, props.ratio));
  return (
    <div className="relative h-5 w-5">
      <svg viewBox="0 0 20 20" className="h-5 w-5 -rotate-90">
        <circle
          cx="10"
          cy="10"
          r={radius}
          stroke="currentColor"
          className="text-border"
          strokeWidth="2"
          fill="none"
        />
        <circle
          cx="10"
          cy="10"
          r={radius}
          stroke="currentColor"
          className={props.urgent ? "text-rose-500" : "text-primary"}
          strokeWidth="2"
          strokeDasharray={`${dash} ${circumference - dash}`}
          strokeLinecap="round"
          fill="none"
        />
      </svg>
      <span
        className={cn(
          "absolute inset-0 flex items-center justify-center text-[9px] font-medium tabular-nums",
          props.urgent && "text-rose-600",
        )}
      >
        {props.label}
      </span>
    </div>
  );
}

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

  const display = props.hidden ? "•".repeat(Math.min(props.value.length, 14)) : props.value;

  return (
    <div>
      <Label className="mb-1 block">{props.label}</Label>
      <div className="flex items-stretch gap-1">
        <div
          className={cn(
            "flex flex-1 items-center rounded-md border border-input bg-background px-2 py-1.5 text-xs",
            "font-mono break-all",
          )}
        >
          {display || <span className="text-muted-foreground">—</span>}
        </div>
        {props.copyable && (
          <Button
            variant="outline"
            size="icon"
            onClick={copy}
            title="Copy"
          >
            {copied ? <Check className="h-3.5 w-3.5" /> : <Copy className="h-3.5 w-3.5" />}
          </Button>
        )}
        {props.rightSlot}
      </div>
    </div>
  );
}

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
        /* may fail on chrome:// — sendMessage below will give a clearer error */
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
    <div className="pt-1">
      <Button onClick={fill} disabled={busy} className="w-full">
        {busy ? "Filling…" : "Fill login form"}
      </Button>
      {error && <p className="mt-1 text-xs text-rose-600">{error}</p>}
    </div>
  );
}

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
    const res = await generatePassword({ length, uppercase, lowercase, numbers, symbols });
    setBusy(false);
    if (res.ok && res.data) setPassword(res.data.password);
  }

  async function copy() {
    await navigator.clipboard.writeText(password);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  }

  // Auto-generate once on mount.
  useEffect(() => {
    void gen();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return (
    <div className="space-y-4 p-3">
      <div className="flex items-stretch gap-1">
        <div className="flex flex-1 items-center rounded-md border border-input bg-background px-2.5 py-2 font-mono text-xs break-all">
          {password || <span className="text-muted-foreground">…</span>}
        </div>
        <Button variant="outline" size="icon" onClick={gen} disabled={busy} title="Regenerate">
          <RefreshCw className={cn("h-3.5 w-3.5", busy && "animate-spin")} />
        </Button>
        <Button variant="outline" size="icon" onClick={copy} title="Copy" disabled={!password}>
          {copied ? <Check className="h-3.5 w-3.5" /> : <Copy className="h-3.5 w-3.5" />}
        </Button>
      </div>

      <div className="space-y-2.5">
        <div className="flex items-center justify-between">
          <Label>Length</Label>
          <span className="text-xs font-medium text-foreground">{length}</span>
        </div>
        <input
          type="range"
          min={8}
          max={64}
          value={length}
          onChange={(e) => setLength(Number(e.target.value))}
          className="w-full accent-[hsl(var(--primary))]"
        />
      </div>

      <div className="grid grid-cols-2 gap-2">
        <Toggle label="Uppercase A-Z" checked={uppercase} onChange={setUppercase} />
        <Toggle label="Lowercase a-z" checked={lowercase} onChange={setLowercase} />
        <Toggle label="Digits 0-9" checked={numbers} onChange={setNumbers} />
        <Toggle label="Symbols !@#" checked={symbols} onChange={setSymbols} />
      </div>
    </div>
  );
}

function Toggle(props: {
  label: string;
  checked: boolean;
  onChange: (v: boolean) => void;
}) {
  return (
    <label className="flex cursor-pointer items-center gap-2 rounded-md border border-input bg-background px-2 py-1.5 text-xs select-none hover:bg-accent/30">
      <input
        type="checkbox"
        checked={props.checked}
        onChange={(e) => props.onChange(e.target.checked)}
        className="accent-[hsl(var(--primary))]"
      />
      <span>{props.label}</span>
    </label>
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
  // ↓ trigger entrance animation
  const className = "spm-slide-in-right relative overflow-hidden border-b border-primary/30";

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
    <div className={className}>
      <div className="absolute inset-0 bg-gradient-to-br from-primary/[0.08] via-primary/[0.04] to-transparent pointer-events-none" />
      <div className="relative px-3.5 py-3">
      <div className="mb-2.5 flex items-center justify-between">
        <div className="flex items-center gap-2 text-sm font-semibold tracking-tight">
          <div className="flex h-5 w-5 items-center justify-center rounded-md bg-primary/15 ring-1 ring-primary/20">
            <Sparkles className="h-3 w-3 text-primary" />
          </div>
          Save this login?
        </div>
        <Button
          variant="ghost"
          size="icon"
          onClick={props.onDismiss}
          title="Discard"
        >
          <X className="h-3.5 w-3.5" />
        </Button>
      </div>
      <div className="mb-2 text-xs text-muted-foreground">
        On <span className="font-medium text-foreground">{props.pending.domain}</span>{" "}
        as <span className="font-medium text-foreground">{props.pending.username}</span>
      </div>
      <Input
        value={title}
        onChange={(e) => setTitle(e.target.value)}
        placeholder="Title"
        className="mb-2 h-8 text-xs"
      />
      <div className="flex gap-2">
        <Button onClick={save} disabled={busy} className="flex-1">
          {busy ? "Saving…" : "Save to vault"}
        </Button>
        <Button variant="outline" onClick={props.onDismiss}>
          Not now
        </Button>
      </div>
      {error && <p className="mt-2 text-xs text-rose-600">{error}</p>}
      </div>
    </div>
  );
}

// -------------------------- status screens --------------------------

function LoadingScreen() {
  return (
    <div className="flex flex-col items-center justify-center gap-2 p-10 text-center text-sm text-muted-foreground">
      <div className="h-5 w-5 animate-spin rounded-full border-2 border-primary border-t-transparent" />
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
      // Close the popup so Chrome doesn't steal focus back.
      window.close();
    } else {
      setError(res.error ?? "Could not bring the app to the front");
    }
  }

  return (
    <div className="flex flex-col items-center gap-4 p-6 text-center">
      <div className="flex h-12 w-12 items-center justify-center rounded-full bg-primary/10">
        <Lock className="h-5 w-5 text-primary" />
      </div>
      <div>
        <div className="text-sm font-medium">Database locked</div>
        <p className="mt-1 text-xs text-muted-foreground leading-relaxed">
          {props.domain
            ? `Unlock Simple Password Manager to autofill on ${props.domain}.`
            : "Unlock Simple Password Manager to use this extension."}
        </p>
      </div>
      <Button onClick={unlock} disabled={busy} className="w-full">
        {busy ? "Bringing app to the front…" : "Open & unlock"}
      </Button>
      {error && <p className="text-xs text-rose-600">{error}</p>}
    </div>
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
    <div className="space-y-4 p-4">
      <div className="flex items-start gap-3">
        <div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-md bg-amber-500/15">
          <Lock className="h-4 w-4 text-amber-600 dark:text-amber-400" />
        </div>
        <div className="space-y-1">
          <div className="text-sm font-medium">Not connected</div>
          <p className="text-xs text-muted-foreground leading-relaxed">{props.message}</p>
        </div>
      </div>

      <Separator />

      <div>
        <Label className="mb-1.5 block">Your extension ID</Label>
        <div className="flex items-stretch gap-1">
          <div className="flex flex-1 items-center rounded-md border border-input bg-background px-2.5 py-1.5 text-xs font-mono break-all">
            {extId}
          </div>
          <Button variant="outline" size="icon" onClick={copyId} title="Copy">
            {copied ? <Check className="h-3.5 w-3.5" /> : <Copy className="h-3.5 w-3.5" />}
          </Button>
        </div>
        <p className="mt-2 text-xs leading-relaxed text-muted-foreground">
          Open Simple Password Manager →{" "}
          <span className="font-medium text-foreground">
            Settings → Application → Browser Extension
          </span>
          , paste the ID, click <em>Register for all detected browsers</em>, then
          restart this browser.
        </p>
      </div>
    </div>
  );
}
