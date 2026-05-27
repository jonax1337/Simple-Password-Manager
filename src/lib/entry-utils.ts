export function formatTimestamp(timestamp?: string): string {
  if (!timestamp) return "—";
  try {
    const date = new Date(timestamp);
    return new Intl.DateTimeFormat("de-DE", {
      year: "numeric",
      month: "2-digit",
      day: "2-digit",
      hour: "2-digit",
      minute: "2-digit",
      second: "2-digit",
      hour12: false,
    }).format(date);
  } catch {
    return "—";
  }
}

function pad(n: number) {
  return String(n).padStart(2, "0");
}

function toLocalIso(d: Date): string {
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

export function getExpiryDate(preset: string): string {
  const now = new Date();
  const target = new Date();
  switch (preset) {
    case "1day": target.setDate(now.getDate() + 1); break;
    case "1week": target.setDate(now.getDate() + 7); break;
    case "2weeks": target.setDate(now.getDate() + 14); break;
    case "1month": target.setMonth(now.getMonth() + 1); break;
    case "3months": target.setMonth(now.getMonth() + 3); break;
    case "6months": target.setMonth(now.getMonth() + 6); break;
    case "1year": target.setFullYear(now.getFullYear() + 1); break;
  }
  return toLocalIso(target);
}

export function getDefaultExpiryDate(): string {
  const d = new Date();
  d.setFullYear(d.getFullYear() + 1);
  return toLocalIso(d);
}

export function validateUrl(url: string): { isValid: boolean; error: string | null } {
  if (!url || url.trim() === "") return { isValid: true, error: null };

  try {
    const urlToTest = url.match(/^https?:\/\//) ? url : `https://${url}`;
    const u = new URL(urlToTest);
    if (!u.hostname) return { isValid: false, error: "Invalid URL format" };

    const ipv4 = /^(\d{1,3}\.){3}\d{1,3}$/;
    const ipv6 = /^([0-9a-fA-F]{0,4}:){2,7}[0-9a-fA-F]{0,4}$/;
    if (u.hostname === "localhost" || ipv4.test(u.hostname) || ipv6.test(u.hostname)) {
      return { isValid: true, error: null };
    }
    if (!u.hostname.includes(".")) {
      return { isValid: false, error: "URL must be a valid domain (e.g., example.com)" };
    }
    const parts = u.hostname.split(".");
    if (!parts.some((p) => /[a-zA-Z]/.test(p))) {
      return { isValid: false, error: "Domain must contain at least one letter" };
    }
    const domainRegex = /^[a-zA-Z0-9]([a-zA-Z0-9-]*[a-zA-Z0-9])?(\.[a-zA-Z0-9]([a-zA-Z0-9-]*[a-zA-Z0-9])?)*$/;
    if (!domainRegex.test(u.hostname)) {
      return { isValid: false, error: "Invalid domain format" };
    }
    return { isValid: true, error: null };
  } catch {
    return { isValid: false, error: "Invalid URL format" };
  }
}

export function getTotpFieldValue(entry: {
  custom_fields?: { name: string; value: string }[];
}): string | null {
  const f = (entry.custom_fields ?? []).find((f) => f.name.toLowerCase() === "otp");
  return f?.value ?? null;
}

export function entryHasTotp(entry: { custom_fields?: { name: string }[] }): boolean {
  return (entry.custom_fields ?? []).some((f) => f.name.toLowerCase() === "otp");
}
