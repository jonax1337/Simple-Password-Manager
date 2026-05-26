"use client";

import { useEffect, useMemo, useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Check, Copy, KeyRound, Trash2 } from "lucide-react";
import { previewTotp, type TotpPreview } from "@/lib/tauri";
import type { EntryData, CustomField } from "@/lib/tauri";

interface Props {
  formData: EntryData;
  setFormData: React.Dispatch<React.SetStateAction<EntryData>>;
  setHasChanges: React.Dispatch<React.SetStateAction<boolean>>;
}

const OTP_FIELD = "otp";

export function TotpSection({ formData, setFormData, setHasChanges }: Props) {
  // The `otp` field, if it exists. We look it up by lowercased name to
  // match the bridge's case-insensitive lookup.
  const otpField = useMemo(
    () =>
      (formData.custom_fields ?? []).find(
        (f) => f.name.toLowerCase() === OTP_FIELD,
      ) ?? null,
    [formData.custom_fields],
  );

  const [editing, setEditing] = useState<boolean>(!otpField);
  const [input, setInput] = useState<string>(otpField?.value ?? "");
  const [preview, setPreview] = useState<TotpPreview | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [validating, setValidating] = useState(false);
  const [copied, setCopied] = useState(false);
  const [tick, setTick] = useState(0);

  // Re-open the editor if the entry switches and there's no otp field yet.
  useEffect(() => {
    setEditing(!otpField);
    setInput(otpField?.value ?? "");
    setError(null);
    setPreview(null);
  }, [otpField?.value, formData.uuid]);

  // Live-validate the input. Debounced 200 ms so we don't hammer the IPC
  // while the user types/pastes.
  useEffect(() => {
    if (!editing) return;
    if (!input.trim()) {
      setPreview(null);
      setError(null);
      return;
    }
    const t = setTimeout(async () => {
      setValidating(true);
      try {
        const result = await previewTotp(input);
        setPreview(result);
        setError(null);
      } catch (e) {
        setPreview(null);
        setError(String(e));
      } finally {
        setValidating(false);
      }
    }, 200);
    return () => clearTimeout(t);
  }, [input, editing]);

  // 1 Hz ticker so the displayed code/countdown refresh.
  const showingCode = !editing && otpField;
  useEffect(() => {
    if (!showingCode) return;
    const t = setInterval(() => setTick((n) => n + 1), 1000);
    return () => clearInterval(t);
  }, [showingCode]);

  // When showing a saved otp, fetch its current code from the backend.
  const [savedCode, setSavedCode] = useState<TotpPreview | null>(null);
  useEffect(() => {
    if (editing) return;
    if (!otpField) return;
    previewTotp(otpField.value)
      .then(setSavedCode)
      .catch(() => setSavedCode(null));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [editing, otpField?.value, tick]);

  function commit(uri: string) {
    setFormData((prev) => {
      const others = (prev.custom_fields ?? []).filter(
        (f) => f.name.toLowerCase() !== OTP_FIELD,
      );
      const next: CustomField = {
        name: OTP_FIELD,
        value: uri,
        protected: true,
      };
      return { ...prev, custom_fields: [...others, next] };
    });
    setHasChanges(true);
    setEditing(false);
  }

  function remove() {
    setFormData((prev) => ({
      ...prev,
      custom_fields: (prev.custom_fields ?? []).filter(
        (f) => f.name.toLowerCase() !== OTP_FIELD,
      ),
    }));
    setHasChanges(true);
    setEditing(true);
    setInput("");
    setPreview(null);
    setError(null);
  }

  async function copyCode() {
    const code = savedCode?.code ?? preview?.code;
    if (!code) return;
    await navigator.clipboard.writeText(code);
    setCopied(true);
    setTimeout(() => setCopied(false), 1200);
  }

  return (
    <div className="rounded-md border bg-card p-3 space-y-2">
      <div className="flex items-center gap-2">
        <KeyRound className="h-4 w-4 text-muted-foreground" />
        <Label className="text-sm font-medium">Two-factor authentication (TOTP)</Label>
      </div>

      {!editing && otpField && savedCode && (
        <SavedCodeView
          code={savedCode.code}
          remaining={Math.max(
            0,
            Math.ceil((savedCode.remaining_seconds * 1000 - tick * 0) / 1000),
          )}
          period={savedCode.period}
          copied={copied}
          onCopy={copyCode}
          onReconfigure={() => setEditing(true)}
          onRemove={remove}
        />
      )}

      {!editing && otpField && !savedCode && (
        <p className="text-xs text-muted-foreground">Reading current code…</p>
      )}

      {editing && (
        <div className="space-y-2">
          <p className="text-xs text-muted-foreground leading-relaxed">
            Paste an <code className="font-mono">otpauth://</code> URI from
            the site&apos;s 2FA setup screen, or just the secret you got from
            the QR-code &quot;manual entry&quot; option.
          </p>
          <div className="flex items-stretch gap-1">
            <Input
              value={input}
              onChange={(e) => setInput(e.target.value)}
              placeholder="otpauth://totp/... or JBSWY3DPEHPK3PXP"
              className="font-mono text-xs"
              spellCheck={false}
            />
            <Button
              size="sm"
              onClick={() => preview && commit(preview.otpauth_uri)}
              disabled={!preview || validating}
            >
              Save 2FA
            </Button>
            {otpField && (
              <Button
                size="sm"
                variant="outline"
                onClick={() => {
                  setEditing(false);
                  setInput(otpField.value);
                  setError(null);
                }}
              >
                Cancel
              </Button>
            )}
          </div>

          {preview && (
            <div className="flex items-center justify-between rounded border border-emerald-500/30 bg-emerald-500/5 px-2.5 py-2">
              <div className="flex items-center gap-2 text-xs">
                <span className="text-emerald-600 dark:text-emerald-400 font-medium">
                  Looks good:
                </span>
                <span className="font-mono text-base tracking-widest">
                  {preview.code.slice(0, 3)} {preview.code.slice(3)}
                </span>
                <span className="text-muted-foreground">
                  · refreshes in {preview.remaining_seconds}s
                </span>
              </div>
            </div>
          )}

          {error && (
            <p className="text-xs text-rose-600 dark:text-rose-400">
              Not valid: {error}
            </p>
          )}

          {validating && !preview && !error && (
            <p className="text-xs text-muted-foreground">Validating…</p>
          )}
        </div>
      )}
    </div>
  );
}

function SavedCodeView(props: {
  code: string;
  remaining: number;
  period: number;
  copied: boolean;
  onCopy: () => void;
  onReconfigure: () => void;
  onRemove: () => void;
}) {
  const ratio = props.period > 0 ? props.remaining / props.period : 0;
  const urgent = props.remaining <= 5;
  return (
    <div className="flex items-center gap-3">
      <div className="flex flex-1 items-center justify-between rounded-md border bg-background px-3 py-2">
        <span className="font-mono text-lg tracking-widest tabular-nums">
          {props.code.slice(0, 3)} {props.code.slice(3)}
        </span>
        <CountdownRing ratio={ratio} urgent={urgent} label={String(props.remaining)} />
      </div>
      <Button size="sm" variant="outline" onClick={props.onCopy}>
        {props.copied ? <Check className="h-3.5 w-3.5" /> : <Copy className="h-3.5 w-3.5" />}
      </Button>
      <Button size="sm" variant="outline" onClick={props.onReconfigure}>
        Reconfigure
      </Button>
      <Button size="sm" variant="ghost" onClick={props.onRemove} title="Remove 2FA">
        <Trash2 className="h-3.5 w-3.5 text-rose-600" />
      </Button>
    </div>
  );
}

function CountdownRing(props: { ratio: number; urgent: boolean; label: string }) {
  const radius = 10;
  const circumference = 2 * Math.PI * radius;
  const dash = circumference * Math.max(0, Math.min(1, props.ratio));
  return (
    <div className="relative h-6 w-6">
      <svg viewBox="0 0 24 24" className="h-6 w-6 -rotate-90">
        <circle
          cx="12"
          cy="12"
          r={radius}
          stroke="currentColor"
          className="text-border"
          strokeWidth="2"
          fill="none"
        />
        <circle
          cx="12"
          cy="12"
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
        className={
          "absolute inset-0 flex items-center justify-center text-[10px] font-medium tabular-nums " +
          (props.urgent ? "text-rose-600" : "")
        }
      >
        {props.label}
      </span>
    </div>
  );
}
