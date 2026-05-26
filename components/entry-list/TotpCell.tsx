"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { Check, Copy, Eye, KeyRound } from "lucide-react";
import { previewTotp } from "@/lib/tauri";
import { useToast } from "@/components/ui/use-toast";

interface TotpCellProps {
  otpValue: string;
}

interface CodeState {
  code: string;
  period: number;
  /** Wall-clock millis at which the current code rolls over. */
  expiresAt: number;
}

export function TotpCell({ otpValue }: TotpCellProps) {
  const [state, setState] = useState<CodeState | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [revealed, setRevealed] = useState(false);
  const [tick, setTick] = useState(0);
  const [copied, setCopied] = useState(false);
  const { toast } = useToast();
  const fetchingRef = useRef(false);

  const refresh = useCallback(async () => {
    if (fetchingRef.current) return;
    fetchingRef.current = true;
    try {
      const result = await previewTotp(otpValue);
      setState({
        code: result.code,
        period: result.period,
        expiresAt: Date.now() + result.remaining_seconds * 1000,
      });
      setError(null);
    } catch (e) {
      setError(String(e));
      setState(null);
    } finally {
      fetchingRef.current = false;
    }
  }, [otpValue]);

  // Fetch lazily — only when the user clicks "Show".
  useEffect(() => {
    if (!revealed) return;
    if (state) return;
    void refresh();
  }, [revealed, state, refresh]);

  // 1Hz tick + refetch on rollover, while revealed.
  useEffect(() => {
    if (!revealed) return;
    const t = setInterval(() => setTick((n) => n + 1), 500);
    return () => clearInterval(t);
  }, [revealed]);

  useEffect(() => {
    if (!revealed || !state) return;
    if (Date.now() < state.expiresAt) return;
    void refresh();
  }, [tick, revealed, state, refresh]);

  if (!revealed) {
    return (
      <button
        type="button"
        onClick={(e) => {
          e.stopPropagation();
          setRevealed(true);
        }}
        className="flex items-center gap-1 rounded px-1.5 py-1 text-xs text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
        title="Reveal 2FA code"
      >
        <KeyRound className="h-3 w-3" />
        <span>Show 2FA</span>
        <Eye className="h-3 w-3 opacity-70" />
      </button>
    );
  }

  if (error) {
    return (
      <span
        className="truncate px-1 text-xs text-amber-600 dark:text-amber-400"
        title={error}
      >
        2FA invalid
      </span>
    );
  }

  if (!state) {
    return <span className="px-1 text-xs text-muted-foreground">…</span>;
  }

  const remaining = Math.max(0, Math.ceil((state.expiresAt - Date.now()) / 1000));
  const ratio = state.period > 0 ? remaining / state.period : 0;
  const urgent = remaining <= 5;

  async function copy() {
    await navigator.clipboard.writeText(state!.code);
    setCopied(true);
    setTimeout(() => setCopied(false), 1200);
    toast({ title: "2FA code copied", variant: "success" });
  }

  return (
    <button
      type="button"
      onClick={(e) => {
        e.stopPropagation();
        void copy();
      }}
      className="flex items-center gap-1.5 rounded px-1 py-0.5 text-xs transition-colors hover:bg-muted"
      title="Click to copy"
    >
      <CountdownRing ratio={ratio} urgent={urgent} label={String(remaining)} />
      <span className="font-mono text-sm tabular-nums tracking-wider">
        {state.code.slice(0, 3)} {state.code.slice(3)}
      </span>
      {copied ? (
        <Check className="h-3 w-3 text-emerald-600" />
      ) : (
        <Copy className="h-3 w-3 text-muted-foreground" />
      )}
    </button>
  );
}

function CountdownRing(props: { ratio: number; urgent: boolean; label: string }) {
  const radius = 7;
  const circumference = 2 * Math.PI * radius;
  const dash = circumference * Math.max(0, Math.min(1, props.ratio));
  return (
    <div className="relative h-4 w-4 shrink-0">
      <svg viewBox="0 0 16 16" className="h-4 w-4 -rotate-90">
        <circle
          cx="8"
          cy="8"
          r={radius}
          stroke="currentColor"
          className="text-border"
          strokeWidth="1.5"
          fill="none"
        />
        <circle
          cx="8"
          cy="8"
          r={radius}
          stroke="currentColor"
          className={props.urgent ? "text-rose-500" : "text-primary"}
          strokeWidth="1.5"
          strokeDasharray={`${dash} ${circumference - dash}`}
          strokeLinecap="round"
          fill="none"
        />
      </svg>
      <span
        className={
          "absolute inset-0 flex items-center justify-center text-[8px] font-medium tabular-nums " +
          (props.urgent ? "text-rose-600" : "")
        }
      >
        {props.label}
      </span>
    </div>
  );
}
