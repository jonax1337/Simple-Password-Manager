# Browser Extension — Architektur

Stand: 2026-05-26. Entscheidungen und Begründungen für die Browser-Integration.

---

## Ziele für MVP

1. **Autofill** in Login-Formularen (Username + Password) via Button-Klick im Extension-Popup.
2. **Suche** nach Einträgen im Popup (filtert nach Domain der aktiven Tab).
3. **Password-Generator** im Popup.
4. **Save-Prompt** nach erfolgreichem Login (Phase 3.2 — optional fürs MVP).

## Nicht-Ziele MVP

- Auto-Detection von Login-Formularen (User klickt aktiv im Popup, kein Inline-Overlay).
- TOTP-Codes (Phase 3.3).
- HTTP Basic Auth Handling.
- KeePassXC-Browser-Protokoll-Kompatibilität (klar dokumentiert als Phase 3.4, wenn überhaupt).

---

## Architektur-Entscheidung

### Drei Wege, die geprüft wurden

| | Pro | Kontra | Verdict |
|---|---|---|---|
| **A. KeePassXC-Browser-Protokoll** | Open Standard, kompatibel mit bestehender KeePassXC-Browser Extension, libsodium-verschlüsselt | Komplexes Protokoll (curve25519 KX, NaCl boxes), libsodium-Binding nötig, viel undocumented Verhalten | ❌ zu viel Aufwand fürs MVP |
| **B. Native Messaging direkt** | Browser-Standard, kein offener Port, sandbox-gesichert | Host-Binary Registration ist OS-spezifisch und brittle; IPC zwischen Host und Tauri-App nötig (Host ist eigener Prozess) | ❌ unverhältnismäßiger Aufwand |
| **C. Lokaler HTTP-Server + Native-Messaging-Host nur als Discovery** | Pragmatisch, einfach zu debuggen, gut zu testen mit curl, klares Trust-Modell | Loopback-Server (Firewall könnte fragen, je nach OS) | ✅ **Gewählt** |

### Gewählter Ansatz im Detail

```
┌────────────────────┐         ┌─────────────────────────────────┐
│  Browser Extension │         │  Tauri App (running, DB unlocked)│
│  (Chrome / Edge /  │         │                                  │
│   Firefox)         │         │  ┌────────────────────────────┐ │
│                    │  HTTPS  │  │ Embedded HTTP server       │ │
│  ┌──────────────┐  │ ───────▶│  │ 127.0.0.1:<random-port>    │ │
│  │   Popup UI   │  │  Token  │  │ Token-Auth, CORS locked    │ │
│  └──────────────┘  │  Auth   │  └────────────────────────────┘ │
│        │           │         │              │                   │
│        ▼           │         │              ▼                   │
│  ┌──────────────┐  │         │  ┌────────────────────────────┐ │
│  │ Native Msg   │  │  stdin  │  │ Native Messaging Host      │ │
│  │ client (chrome.│ ◀──────▶│  │ binary (im Tauri-Bundle)   │ │
│  │ runtime.send  │  │ stdout │  │ → liest port+token aus     │ │
│  │ Native...)   │  │  JSON   │  │   tempfile, gibt's an      │ │
│  └──────────────┘  │         │  │   Extension                │ │
└────────────────────┘         │  └────────────────────────────┘ │
                               │              ▲                   │
                               │              │                   │
                               │  schreibt    │                   │
                               │  port+token  │                   │
                               │              │                   │
                               │  ┌────────────────────────────┐ │
                               │  │ Token-File in              │ │
                               │  │ %APPDATA%/passwordwallet/  │ │
                               │  │   bridge.json              │ │
                               │  │ (mode 600, nur user lesbar)│ │
                               │  └────────────────────────────┘ │
                               └─────────────────────────────────┘
```

### Warum Hybrid statt nur einem Mechanismus

- **HTTP-Server statt direkter Native-Messaging-IPC**: HTTP ist trivial zu debuggen (curl, browser devtools), klar definiert, hat eingebaute Methods/Status-Codes. Native Messaging bringt nur 1MB-Message-Limit und einen umständlichen length-prefixed-JSON-Stream, ohne handfesten Vorteil.
- **Native Messaging Host nur für Discovery**: Browser-Extensions können nicht raten, welcher Port verwendet wird oder wie der Token lautet. Der Native-Messaging-Host ist die einzige offizielle Brücke vom Extension-Sandbox zum lokalen Dateisystem. Er liest die `bridge.json`, gibt port+token an die Extension, fertig. **Keine eigene Business-Logik im Host.**
- **Token-File statt UI-Pairing**: User soll nicht jedes Mal beim App-Start einen 6-stelligen Code in die Extension tippen müssen. Der Token wird beim App-Start einmal generiert (256-bit random) und in die User-Data-Dir geschrieben (mode 0600 / Windows ACL). Wer Lese-Zugriff auf die Datei hat, hat eh schon User-Account-Zugriff — kein zusätzliches Risiko.

---

## Sicherheits-Modell

### Was die Extension darf
- Eintrag lesen (Title, Username, Password, URL) — nur wenn DB unlocked
- Liste von Einträgen filtern nach Domain
- Passwort generieren

### Was die Extension NICHT darf
- DB öffnen / unlocken (das macht weiter der User in der App)
- DB schreiben **im MVP** (Save-Prompt kommt in Phase 3.2)
- Master-Passwort sehen
- Andere Domains' Einträge ohne explizite User-Action

### Per-Domain-Approval
Erste Anfrage einer neuen Domain triggert in der App einen Dialog: "Soll example.com auf gespeicherte Logins zugreifen?". Ja/Nein wird in der DB als Custom-Field pro Eintrag oder global im `meta.custom_data` gespeichert. Damit funktioniert die Berechtigung auch nach DB-Reload.

### Token-Lifecycle
- Generiert beim **App-Start** (nicht beim DB-Open), rotiert nicht innerhalb einer Session.
- Wenn die App schließt: `bridge.json` wird gelöscht.
- Wenn die DB locked: Server gibt 423 Locked zurück, Extension zeigt "Bitte App entsperren".

### CORS und Origin-Check
- `Access-Control-Allow-Origin` nur für `chrome-extension://<unsere-id>` und `moz-extension://<unsere-id>`.
- Server prüft `Origin`-Header explizit und lehnt unbekannte ab.

### Loopback-Binding
- Server bindet auf `127.0.0.1` (nicht `0.0.0.0`). Damit kommt nichts aus dem Netzwerk durch, nur lokale Prozesse.

---

## API zwischen Extension und App

REST über HTTP. Alle Requests brauchen `Authorization: Bearer <token>` Header.

```
GET  /v1/status                 → { unlocked: bool, dbName: string }
GET  /v1/entries?domain=ex.com  → [{ id, title, username, url }, ...]
GET  /v1/entries/:id/password   → { password: string }   (separat, auditierbar)
POST /v1/password/generate      → { password: string }   body: { length, charsets }
POST /v1/entries                → erstellt neuen Eintrag (Phase 3.2)
```

Fehler:
- `401 Unauthorized` — Token fehlt/falsch
- `403 Forbidden` — Domain nicht approved
- `423 Locked` — DB locked

---

## Code-Struktur

```
extension/                          ← Browser-Extension (neuer Subdir)
  manifest.json                     ← MV3, Background+Popup+Content-Script
  src/
    background.ts                   ← Service Worker, Native-Msg-Discovery
    popup/
      Popup.tsx                     ← shadcn/ui-Style passend zur App
      hooks/useBridge.ts            ← HTTP-Client, Token-Refresh
    content/
      autofill.ts                   ← Form-Fill bei Klick
  package.json                      ← eigenes npm-Projekt (Vite + React)
  vite.config.ts                    ← MV3-Build
  ARCHITECTURE.md                   ← dieses Dokument

src-tauri/src/
  bridge/                           ← neuer Rust-Modul
    server.rs                       ← axum HTTP-Server, läuft im Tauri-Thread
    auth.rs                         ← Token-Generierung, bridge.json schreiben
    handlers.rs                     ← die /v1/... Endpunkte
  bin/
    native_messaging_host.rs        ← separates Binary, einfacher stdio-JSON-Loop
```

Tauri-seitig kommen drei neue Crates dazu: `axum` (HTTP), `tokio` (async, schon transitiv da), `dirs` (User-Data-Dir, schon da).

---

## Implementierungs-Reihenfolge

1. **Backend HTTP-Server** in der Tauri-App, mit `/v1/status` als einzigem Endpunkt. Token+bridge.json geschrieben beim Start. Manuell mit curl testen.
2. **Native Messaging Host** (~50 Zeilen Rust): liest `bridge.json`, antwortet auf `{"action":"discover"}` mit `{"port":..., "token":"..."}`.
3. **Extension Skelett**: Vite + MV3 Manifest, leeres Popup. Lokal in Chrome laden (`chrome://extensions` → Developer Mode → Load unpacked).
4. **Native-Messaging-Host-Manifest** für Chrome (per Setup-Skript registriert) — Phase wo's "real" wird.
5. **Popup UI**: Suchfeld, Liste der Einträge der aktuellen Domain, Copy-Buttons.
6. **Content-Script**: Bei Klick auf "Fill" im Popup → Tabs-API sendet message an Content-Script → Form-Felder gefüllt.
7. **Per-Domain-Approval-Dialog** in der App.
8. **Password-Generator-Endpunkt** + UI im Popup.
9. **Edge / Firefox**: gleiche Extension, separate Host-Manifeste.

Phase 3.2 dann später: Save-Prompt, Edit-Funktionen, TOTP.

---

## Was als nächstes konkret passiert

Wenn du den Plan absegnest:
1. axum + tokio in `src-tauri/Cargo.toml`
2. `bridge/server.rs` mit `/v1/status` und Token-Logik
3. Manuell mit curl testen
4. Dann erst weiter mit dem Native Messaging Host und der Extension selbst

Dauert grob: Backend-Skelett ~3-4h, Extension-Skelett ~3h, Popup-Funktionalität ~5-6h, Content-Script ~3-4h, Polish/Edge-Cases ~5h. Realistisch ~3 produktive Tage.

---

## Stand der Implementierung (2026-05-26)

MVP ist gebaut und smoke-getestet:

| Component | Status |
|---|---|
| `bridge/` Rust Module (axum HTTP server, bearer auth, CORS) | ✅ |
| `/v1/status`, `/v1/entries`, `/v1/entries/:id/password`, `/v1/password/generate` | ✅ |
| `bridge.json` write/cleanup mit File-Permissions | ✅ |
| `browser-bridge-host` Native Messaging Host Binary | ✅ |
| Extension Skelett (Vite + MV3 + React + TS) | ✅ |
| Popup UI: This-site / All-entries / Generator Tabs | ✅ |
| Content Script Form-Fill mit React-safe Native Setter | ✅ |
| `scripts/install-host.mjs` für Chrome/Edge/Firefox auf Win/Mac/Linux | ✅ |
| Per-Domain-Approval-Dialog | ❌ deferred to Phase 3.2 |
| Auto-Save Detection | ❌ deferred to Phase 3.2 |
| TOTP-Integration | ❌ deferred to Phase 3.3 |

Verified end-to-end via curl + Node-Test-Harness:
- HTTP-Server bindet 127.0.0.1:&lt;random&gt;, bearer-auth, CORS open für `chrome-extension://`
- NMH liefert `{status:"ok", port, token, pid}` wenn App läuft, `{status:"no-app"}` sonst
- PID-Check verhindert stale-token-Probleme nach App-Crash

Was noch nicht in dieser Session getestet wurde (braucht echten Browser):
- Extension in Chrome/Edge/Firefox geladen und Popup gerendert
- Native-Host-Registration via `install-host.mjs`
- Echter Login-Form-Fill auf einer realen Webseite

Workflow für E2E-Test ist in `extension/README.md` dokumentiert.
