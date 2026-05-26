// Heuristic: is this form a sign-up (new account) or a sign-in (existing one)?
//
// Used so the inline banner can say "Save this new account" vs "Save this
// login" and so the saved entry title can default sensibly.
//
// We deliberately don't try to be perfect — wrong-guessing only affects
// wording, the save itself is identical either way.

export type Intent = "login" | "signup";

const SIGNUP_RE =
  /sign\s*up|register|create\s+account|join\s+now|registrieren|konto\s+erstellen/i;
const LOGIN_RE =
  /sign\s*in|log\s*in|anmelden|einloggen|sich\s+anmelden/i;
const SIGNUP_URL_RE =
  /signup|register|sign-up|create-account|join|registrieren/i;
const LOGIN_URL_RE = /signin|login|anmelden|sign-in|log-in/i;

export function detectIntent(passwordField: HTMLInputElement | null): Intent {
  let score = 0; // positive → signup, negative → login

  // 1. Two or more visible password inputs → almost certainly signup
  const pwInputs = Array.from(
    document.querySelectorAll<HTMLInputElement>("input[type=password]"),
  ).filter(isVisible);
  if (pwInputs.length >= 2) score += 3;

  // 2. Names of nearby inputs
  if (passwordField) {
    const form = passwordField.closest("form");
    const scope = form ?? document;
    const inputs = Array.from(
      scope.querySelectorAll<HTMLInputElement>("input"),
    ).filter(isVisible);

    // Many text inputs → likely a registration form with name/dob/etc.
    if (inputs.length >= 4) score += 1;

    // Look for tell-tale field names
    const hayInputs = inputs
      .map((el) =>
        [
          el.autocomplete ?? "",
          el.name ?? "",
          el.id ?? "",
          el.placeholder ?? "",
          el.getAttribute("aria-label") ?? "",
        ]
          .join(" ")
          .toLowerCase(),
      )
      .join(" | ");
    if (/firstname|lastname|fullname|first-name|last-name|vorname|nachname/.test(hayInputs)) {
      score += 2;
    }
    if (/new-password|new_password/.test(hayInputs)) score += 2;
    if (/current-password|current_password/.test(hayInputs)) score -= 2;
  }

  // 3. Nearby submit button text
  const buttons = Array.from(
    document.querySelectorAll<HTMLElement>(
      "button, input[type=submit], [role=button]",
    ),
  ).filter(isVisible);
  for (const btn of buttons) {
    const label = (
      btn.innerText ??
      btn.getAttribute("aria-label") ??
      btn.getAttribute("value") ??
      ""
    ).trim();
    if (!label) continue;
    if (SIGNUP_RE.test(label)) {
      score += 2;
      break;
    }
    if (LOGIN_RE.test(label)) {
      score -= 2;
      break;
    }
  }

  // 4. URL path
  const path = window.location.pathname + window.location.search;
  if (SIGNUP_URL_RE.test(path)) score += 2;
  else if (LOGIN_URL_RE.test(path)) score -= 1;

  // 5. Page title
  if (SIGNUP_RE.test(document.title)) score += 1;
  else if (LOGIN_RE.test(document.title)) score -= 1;

  return score >= 2 ? "signup" : "login";
}

function isVisible(el: HTMLElement): boolean {
  if (el.hidden) return false;
  const rect = el.getBoundingClientRect();
  if (rect.width === 0 || rect.height === 0) return false;
  const style = window.getComputedStyle(el);
  if (style.visibility === "hidden" || style.display === "none") return false;
  if (parseFloat(style.opacity) < 0.1) return false;
  return true;
}
