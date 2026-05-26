// Domain comparison that handles subdomain navigations during login flows.
//
// Many providers send the user across subdomains during a sign-in:
//   github.com → github.com/sessions/two-factor   (same host)
//   auth.example.com → app.example.com            (different sub, same eTLD+1)
//   accounts.google.com → mail.google.com         (different sub, same eTLD+1)
//
// We use a heuristic eTLD+1 extraction: take the last two dot-separated
// labels, but for the small set of well-known two-level TLDs we take three.
// Good enough for every real flow we'd want to catch; a full public-suffix
// list would be 30 kB+ of data we don't need.

const TWO_LEVEL_TLDS = new Set([
  "co.uk",
  "co.jp",
  "co.za",
  "co.nz",
  "co.kr",
  "co.in",
  "com.au",
  "com.br",
  "com.cn",
  "com.mx",
  "com.tr",
  "com.ar",
  "com.sg",
  "com.hk",
  "com.tw",
  "com.my",
  "ne.jp",
  "or.jp",
  "ac.uk",
  "ac.jp",
  "gov.uk",
  "org.uk",
]);

export function baseDomain(host: string): string {
  const clean = host.replace(/^www\./, "").toLowerCase();
  const parts = clean.split(".");
  if (parts.length <= 2) return clean;
  const last2 = parts.slice(-2).join(".");
  if (TWO_LEVEL_TLDS.has(last2) && parts.length >= 3) {
    return parts.slice(-3).join(".");
  }
  return last2;
}

/** True if two hosts should be treated as the same site for save prompts. */
export function isSameSite(a: string, b: string): boolean {
  return baseDomain(a) === baseDomain(b);
}
