import { useEffect, useState } from "react";
import {
  fetchAllEntries,
  fetchEntriesForDomain,
  fetchPassword,
  fetchStatus,
  generatePassword,
} from "./bridge-client";
import { useActiveDomain } from "./useActiveDomain";
import type { EntrySummary, StatusResult } from "../common/bridge";

type View = "domain" | "all" | "generator";

export function Popup() {
  const [status, setStatus] = useState<StatusResult | null>(null);
  const [statusError, setStatusError] = useState<string | null>(null);
  const { domain, tabId, loading: domainLoading } = useActiveDomain();
  const [view, setView] = useState<View>("domain");
  const [entries, setEntries] = useState<EntrySummary[] | null>(null);
  const [entriesError, setEntriesError] = useState<string | null>(null);
  const [filter, setFilter] = useState("");

  // Bootstrap: load status. Any failure means the app isn't reachable.
  useEffect(() => {
    fetchStatus().then((res) => {
      if (res.ok && res.data) {
        setStatus(res.data);
      } else {
        setStatusError(res.error ?? "Could not reach Simple Password Manager");
      }
    });
  }, []);

  // Load entries when view or domain changes (once app is unlocked).
  useEffect(() => {
    if (!status?.unlocked) return;
    if (view === "generator") return;
    setEntries(null);
    setEntriesError(null);
    const promise =
      view === "domain" && domain
        ? fetchEntriesForDomain(domain)
        : fetchAllEntries();
    promise.then((res) => {
      if (res.ok && res.data) {
        setEntries(res.data);
      } else {
        setEntriesError(res.error ?? "Failed to load entries");
      }
    });
  }, [view, domain, status?.unlocked]);

  if (statusError) {
    return <SetupScreen message={statusError} />;
  }
  if (!status) {
    return <LoadingScreen />;
  }
  if (!status.unlocked) {
    return (
      <ErrorScreen message="The database is locked. Open Simple Password Manager and unlock it to use this extension." />
    );
  }

  const filtered = entries?.filter((e) => {
    if (!filter.trim()) return true;
    const hay = `${e.title} ${e.username} ${e.url}`.toLowerCase();
    return hay.includes(filter.toLowerCase());
  });

  return (
    <div className="flex flex-col">
      <Header
        dbName={status.database_name}
        domain={domain}
        domainLoading={domainLoading}
      />
      <Tabs view={view} setView={setView} hasDomain={!!domain} />

      {view === "generator" ? (
        <Generator />
      ) : (
        <>
          <SearchInput value={filter} onChange={setFilter} />
          <EntryList
            entries={filtered}
            error={entriesError}
            tabId={tabId}
            view={view}
            domain={domain}
          />
        </>
      )}
    </div>
  );
}

function Header(props: {
  dbName: string | null;
  domain: string | null;
  domainLoading: boolean;
}) {
  return (
    <header className="border-b border-zinc-200 dark:border-zinc-800 px-3 py-2">
      <div className="flex items-baseline justify-between gap-2">
        <div className="font-medium truncate">
          {props.dbName ?? "Simple Password Manager"}
        </div>
        <div className="text-xs text-zinc-500 dark:text-zinc-400 truncate">
          {props.domainLoading ? "…" : props.domain ?? "no http(s) tab"}
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
  const tab = (id: View, label: string, disabled = false) => {
    const active = props.view === id;
    return (
      <button
        disabled={disabled}
        onClick={() => props.setView(id)}
        className={[
          "flex-1 px-3 py-2 text-xs font-medium border-b-2 transition-colors",
          active
            ? "border-indigo-500 text-indigo-600 dark:text-indigo-400"
            : "border-transparent text-zinc-600 dark:text-zinc-400 hover:text-zinc-900 dark:hover:text-zinc-100",
          disabled ? "opacity-40 cursor-not-allowed" : "cursor-pointer",
        ].join(" ")}
      >
        {label}
      </button>
    );
  };
  return (
    <nav className="flex border-b border-zinc-200 dark:border-zinc-800">
      {tab("domain", "This site", !props.hasDomain)}
      {tab("all", "All entries")}
      {tab("generator", "Generate")}
    </nav>
  );
}

function SearchInput(props: { value: string; onChange: (s: string) => void }) {
  return (
    <div className="p-2 border-b border-zinc-200 dark:border-zinc-800">
      <input
        type="text"
        placeholder="Filter…"
        value={props.value}
        onChange={(e) => props.onChange(e.target.value)}
        className="w-full px-2 py-1.5 text-sm bg-zinc-100 dark:bg-zinc-900 border border-zinc-200 dark:border-zinc-800 rounded focus:outline-none focus:ring-2 focus:ring-indigo-500"
      />
    </div>
  );
}

function EntryList(props: {
  entries: EntrySummary[] | undefined;
  error: string | null;
  tabId: number | null;
  view: View;
  domain: string | null;
}) {
  if (props.error) {
    return <p className="p-3 text-sm text-rose-600">{props.error}</p>;
  }
  if (!props.entries) {
    return <p className="p-3 text-sm text-zinc-500">Loading…</p>;
  }
  if (props.entries.length === 0) {
    const msg =
      props.view === "domain" && props.domain
        ? `No entries for ${props.domain}`
        : "No entries";
    return <p className="p-3 text-sm text-zinc-500">{msg}</p>;
  }
  return (
    <ul className="divide-y divide-zinc-100 dark:divide-zinc-900 max-h-[300px] overflow-y-auto">
      {props.entries.map((e) => (
        <EntryRow key={e.uuid} entry={e} tabId={props.tabId} />
      ))}
    </ul>
  );
}

function EntryRow(props: { entry: EntrySummary; tabId: number | null }) {
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  async function reveal(action: "fill" | "copy-user" | "copy-pass") {
    setBusy(action);
    setError(null);
    try {
      const res = await fetchPassword(props.entry.uuid);
      if (!res.ok || !res.data) {
        setError(res.error ?? "Could not fetch password");
        return;
      }
      if (action === "copy-user") {
        await navigator.clipboard.writeText(res.data.username);
      } else if (action === "copy-pass") {
        await navigator.clipboard.writeText(res.data.password);
      } else if (action === "fill") {
        if (props.tabId === null) {
          setError("No active tab to fill into");
          return;
        }
        // Ensure the content script is loaded — on page-load races MV3 may
        // not have run it yet, so inject defensively.
        try {
          await chrome.scripting.executeScript({
            target: { tabId: props.tabId },
            files: ["content.js"],
          });
        } catch {
          // Permission may forbid scripting on chrome:// pages etc. — that's
          // fine, the sendMessage call below will fail with a clear error.
        }
        await chrome.tabs.sendMessage(props.tabId, {
          kind: "fill",
          username: res.data.username,
          password: res.data.password,
        });
        // Close popup so the user lands back on the page.
        window.close();
      }
    } catch (e) {
      setError((e as Error).message);
    } finally {
      setBusy(null);
    }
  }

  return (
    <li className="p-3 hover:bg-zinc-50 dark:hover:bg-zinc-900/40">
      <div className="flex flex-col gap-0.5 min-w-0">
        <div className="font-medium truncate">
          {props.entry.title || "(untitled)"}
        </div>
        <div className="text-xs text-zinc-500 dark:text-zinc-400 truncate">
          {props.entry.username || "—"}
        </div>
      </div>
      <div className="mt-2 flex flex-wrap gap-1">
        <ActionButton
          onClick={() => reveal("fill")}
          busy={busy === "fill"}
          variant="primary"
        >
          Fill
        </ActionButton>
        <ActionButton onClick={() => reveal("copy-user")} busy={busy === "copy-user"}>
          Copy user
        </ActionButton>
        <ActionButton onClick={() => reveal("copy-pass")} busy={busy === "copy-pass"}>
          Copy pass
        </ActionButton>
      </div>
      {error && <p className="text-xs text-rose-600 mt-1">{error}</p>}
    </li>
  );
}

function ActionButton(props: {
  onClick: () => void;
  busy?: boolean;
  variant?: "primary";
  children: React.ReactNode;
}) {
  const base =
    "px-2 py-1 text-xs rounded font-medium transition-colors disabled:opacity-50";
  const variant =
    props.variant === "primary"
      ? "bg-indigo-600 text-white hover:bg-indigo-500"
      : "bg-zinc-100 dark:bg-zinc-800 text-zinc-700 dark:text-zinc-200 hover:bg-zinc-200 dark:hover:bg-zinc-700";
  return (
    <button
      onClick={props.onClick}
      disabled={!!props.busy}
      className={`${base} ${variant}`}
    >
      {props.busy ? "…" : props.children}
    </button>
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
  const [error, setError] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);

  async function gen() {
    setBusy(true);
    setError(null);
    setCopied(false);
    const res = await generatePassword({
      length,
      uppercase,
      lowercase,
      numbers,
      symbols,
    });
    setBusy(false);
    if (res.ok && res.data) {
      setPassword(res.data.password);
    } else {
      setError(res.error ?? "Failed to generate password");
    }
  }

  async function copy() {
    await navigator.clipboard.writeText(password);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  }

  return (
    <div className="p-3 flex flex-col gap-3">
      <div className="flex items-center justify-between gap-2">
        <label htmlFor="len" className="text-xs">
          Length: {length}
        </label>
        <input
          id="len"
          type="range"
          min={8}
          max={64}
          value={length}
          onChange={(e) => setLength(Number(e.target.value))}
          className="flex-1"
        />
      </div>
      <div className="grid grid-cols-2 gap-2 text-xs">
        <Toggle label="ABC" checked={uppercase} onChange={setUppercase} />
        <Toggle label="abc" checked={lowercase} onChange={setLowercase} />
        <Toggle label="123" checked={numbers} onChange={setNumbers} />
        <Toggle label="!@#" checked={symbols} onChange={setSymbols} />
      </div>
      <button
        onClick={gen}
        disabled={busy}
        className="px-3 py-2 text-sm bg-indigo-600 text-white rounded font-medium hover:bg-indigo-500 disabled:opacity-50"
      >
        {busy ? "Generating…" : "Generate"}
      </button>
      {password && (
        <div className="flex items-stretch gap-1">
          <code className="flex-1 px-2 py-1.5 text-xs bg-zinc-100 dark:bg-zinc-900 rounded font-mono break-all">
            {password}
          </code>
          <button
            onClick={copy}
            className="px-2 text-xs bg-zinc-100 dark:bg-zinc-800 rounded hover:bg-zinc-200 dark:hover:bg-zinc-700"
          >
            {copied ? "✓" : "Copy"}
          </button>
        </div>
      )}
      {error && <p className="text-xs text-rose-600">{error}</p>}
    </div>
  );
}

function Toggle(props: {
  label: string;
  checked: boolean;
  onChange: (b: boolean) => void;
}) {
  return (
    <label className="flex items-center gap-2 cursor-pointer select-none">
      <input
        type="checkbox"
        checked={props.checked}
        onChange={(e) => props.onChange(e.target.checked)}
      />
      <span>{props.label}</span>
    </label>
  );
}

function LoadingScreen() {
  return (
    <div className="p-6 text-center text-sm text-zinc-500">
      Connecting to Simple Password Manager…
    </div>
  );
}

function ErrorScreen(props: { message: string }) {
  return (
    <div className="p-4">
      <p className="text-sm text-zinc-700 dark:text-zinc-300">{props.message}</p>
    </div>
  );
}

// Shown when the extension can't reach the desktop app. The most common
// cause is that the user hasn't registered the native messaging host yet,
// so we display the extension ID prominently — they paste it into the
// desktop app's Settings → Browser Extension card.
function SetupScreen(props: { message: string }) {
  const extId = chrome.runtime.id;
  const [copied, setCopied] = useState(false);

  async function copyId() {
    await navigator.clipboard.writeText(extId);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  }

  return (
    <div className="p-4 space-y-3">
      <p className="text-sm text-zinc-700 dark:text-zinc-300">{props.message}</p>
      <div className="border-t border-zinc-200 dark:border-zinc-800 pt-3">
        <p className="text-xs text-zinc-500 mb-1">Your extension ID:</p>
        <div className="flex items-stretch gap-1">
          <code className="flex-1 px-2 py-1.5 text-xs bg-zinc-100 dark:bg-zinc-900 rounded font-mono break-all">
            {extId}
          </code>
          <button
            onClick={copyId}
            className="px-2 text-xs bg-zinc-100 dark:bg-zinc-800 rounded hover:bg-zinc-200 dark:hover:bg-zinc-700"
          >
            {copied ? "✓" : "Copy"}
          </button>
        </div>
        <p className="text-xs text-zinc-500 mt-2 leading-relaxed">
          Paste this ID into Simple Password Manager →{" "}
          <strong>Settings → Application → Browser Extension</strong>, then
          click <em>Register for all detected browsers</em>. Restart this
          browser afterwards.
        </p>
      </div>
    </div>
  );
}
