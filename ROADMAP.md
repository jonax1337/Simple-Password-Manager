# Roadmap — Richtung 1Password

Stand: 2026-05-30. Ziel: Von "lokaler KeePass-Client" zu vollwertiger Password-Suite (Desktop + Browser + Sync). Phasen sind so geordnet, dass jede für sich abgeschlossen ist und keine spätere Phase blockiert.

## Status

- **Phase 1 (Quick Wins): ✅ ausgeliefert** — Close-to-Tray, Autostart, Update-Workflow, signed Bundles.
- **v2.0 (Mai 2026): ✅ ausgeliefert** — kompletter Frontend-Rewrite von Next.js/React → **Svelte 5 + Vite**.
- **Phase 2 (Performance): ✅ Profiling abgeschlossen.**
- **Phase 3 (Browser Extension): ✅ ausgeliefert** — KeePassXC-kompatibler Native-Messaging-Host, Bridge-HTTP-Server, Auto-Fill/Save-Prompt MVP.
- **Phase 4 (Cloud-Ordner-Sync): ✅ ausgeliefert (2026-05-30)** — siehe unten, Implementation in `src-tauri/src/lockfile.rs`, `src-tauri/src/commands/database.rs` (notify-watcher), `src/lib/components/ConflictResolutionDialog.svelte`.
- **Phase 5 (Eigener Sync-Server): ⚠️ MVP-Skelett (2026-05-30)** — `server/` crate (Axum + SQLite + JWT), E2E-Crypto in `src-tauri/src/cloud/`, Cloud-Tab in Settings. Bewusst NICHT produktionsreif: keine Recovery-Codes, keine Rate-Limits, kein automatisches Onboarding für neue Geräte. Siehe `server/README.md` für Threat-Model + Self-Hosting.
- **Dateipfade in den älteren Phasen-Notizen unten** (`components/Settings.tsx`, `lib/storage.ts` etc.) sind aus der React-Ära — die Funktionen leben jetzt unter `src/lib/` und `src/lib/components/`.

---

## Phase 1 — Quick Wins (geschätzt 1-2 Tage)

Drei kleinere, sichtbare Features. Geringes Risiko, kein Architektur-Eingriff.

### 1.1 Close-to-Tray als Default
- **Status heute**: `closeToTray` Setting existiert in `components/Settings.tsx:29`, Default `false`. Tray-Icon ist in `src-tauri/src/main.rs:62-94` schon gebaut.
- **Änderung**:
  - Default in `Settings.tsx` auf `true` ändern (neue User bekommen Tray-Verhalten).
  - **Wichtig**: Bestehende User dürfen nicht überrascht werden. Lösung: neuer Storage-Key `closeToTray_v2` mit Default `true`; alter Key bleibt für Migration einmalig respektiert.
  - Close-Handler im Main-Window-Listener (irgendwo in `components/main-app/`) muss auf das Setting reagieren und `window.hide()` statt `window.close()` aufrufen.
- **Aufwand**: ~30 min.

### 1.2 Autostart bei Installation
- **Mechanismus**: `tauri-plugin-autostart` (offizielles Plugin).
- **Schritte**:
  1. `tauri-plugin-autostart = "2.0"` in `src-tauri/Cargo.toml`.
  2. `@tauri-apps/plugin-autostart` in `package.json`.
  3. Plugin in `main.rs` registrieren mit `MacosLauncher::LaunchAgent` + Args wie `--minimized`.
  4. Neuer Tauri-Command `enable_autostart` / `disable_autostart` / `is_autostart_enabled`.
  5. UI in Settings: Toggle "Bei Windows-Start automatisch starten".
  6. **Bei Installation aktivieren**: In `src-tauri/tauri.conf.json` bundle.windows.nsis.installerHooks (existiert schon, Datei fehlt aber unter `nsis/`) — Hook anlegen, der nach Install den Autostart-Eintrag in der Registry (`HKCU\Software\Microsoft\Windows\CurrentVersion\Run`) setzt. Alternativ: erster App-Start ruft `enable_autostart()` einmal auf, gesteuert über localStorage-Flag `firstLaunchHandled`.
- **App-Verhalten beim Autostart**: Direkt minimiert in Tray starten (kein Hauptfenster sichtbar), DB bleibt natürlich gelockt.
- **Risiken**: Antivirus-Tools markieren Autostart-Einträge gelegentlich. Lösung: Signiertes Binary (siehe Phase 1.3).

### 1.3 GitHub-basierter Update-Checker
- **Mechanismus**: `tauri-plugin-updater` (offiziell), Endpoint zeigt auf eine `latest.json` in GitHub Releases.
- **Vorarbeit**: Signing-Key generieren (`npm run tauri signer generate`). **Private Key** lokal sicher aufbewahren (Password-Manager? :) ). Public Key in `tauri.conf.json`.
- **Release-Pipeline** (`.github/workflows/release.yml` — neu):
  1. Trigger auf Git-Tag `v*`.
  2. Build auf `windows-latest` (matrix später erweiterbar auf macOS/Linux).
  3. Signiert mit Private Key (aus GH Secret `TAURI_SIGNING_PRIVATE_KEY`).
  4. Erzeugt Installer + `latest.json` mit Signatur.
  5. Upload als Release-Asset.
- **App-Seite**:
  1. Plugin registrieren, Endpoint = `https://github.com/jonax1337/<repo>/releases/latest/download/latest.json`.
  2. Beim App-Start: silent check, Toast "Update verfügbar". Setting "Updates automatisch installieren" (Default: nur benachrichtigen).
  3. UI-Komponente in Settings: "Auf Updates prüfen" Button + "Installieren & neustarten".
- **Risiken**:
  - Code-Signing-Zertifikat für Windows (nicht das Update-Signing — das ist ein anderes Konzept) ist kostenpflichtig. Ohne wird SmartScreen bei jeder Version warnen. Optional, aber für seriöse Wahrnehmung wichtig.
  - GitHub Repo muss `public` sein, sonst Token nötig.
- **Aufwand**: ~4-6 Stunden (mit erstmaligem CI-Setup).

---

## Phase 2 — Performance (gezielt, datengetrieben)

Bei 100-500 Einträgen sind die typischen Engpässe oft nicht da, wo man denkt. Erst messen, dann fixen.

### 2.1 Profiling
- React DevTools Profiler im Dev-Build: Welche Komponenten rendern bei welchen Aktionen?
- Tauri-Kommando-Timing: in `lib/tauri.ts` einen dünnen Wrapper, der `console.time`/`timeEnd` um jeden invoke legt (nur Dev).
- Konkrete Szenarien zum Messen:
  - Kalter App-Start bis Login-Screen (Ziel: < 500 ms).
  - DB-Öffnen mit ~300 Einträgen (Ziel: < 1.5 s, dominiert von Argon2 — kaum optimierbar ohne KDF-Parameter zu schwächen).
  - Wechsel zwischen Gruppen (Ziel: < 50 ms).
  - Tippen in der Suche (Ziel: < 16 ms pro Keystroke).
  - Entry-Detail öffnen (Ziel: < 100 ms).

### 2.2 Wahrscheinliche Optimierungen
- **Entry-List**: Bei 100-500 reicht React-Virtuell wahrscheinlich noch nicht — aber `useMemo` für sortierte/gefilterte Listen prüfen. Wenn Profile sagt: Virtualisierung mit `@tanstack/react-virtual`.
- **Search-Debounce**: Falls noch nicht vorhanden, 100-150ms Debounce in `useSearch`.
- **Tauri-Roundtrips reduzieren**: Falls beim Gruppen-Wechsel immer ein `get_entries`-Call passiert — Caching im Frontend prüfen.
- **Re-Renders**: `React.memo` auf `EntryListItem`, stabile Props per `useCallback`.
- **Startup**: Next.js initial bundle prüfen (`next build` → Bundle-Analyzer). Framer Motion ist tendenziell schwer; bei Bedarf einzelne Animationen ohne ersetzen.
- **Rust-Seite**: Wenn `get_entries` für 500 Einträge spürbar ist, prüfen ob unnötiges Cloning passiert. `kdbx/database.rs` und `kdbx/entry.rs` checken.

### 2.3 Was NICHT machen
- Keine spekulativen Refactorings ohne Messung.
- Keine globalen State-Libs (Zustand/Redux) "weil schneller" — der React-State-Ansatz funktioniert für diese App-Größe.
- KDF-Parameter NICHT schwächen, um Öffnen zu beschleunigen.

---

## Phase 3 — Browser Extension (Chrome + Edge + Firefox)

Eigenständiges Subprojekt. Schätzung: 1-2 Wochen für solides MVP.

### 3.1 Architektur-Entscheidung: Wie kommuniziert die Extension mit der Desktop-App?

Drei Optionen:

**Option A — KeePassXC-Browser-Protokoll (empfohlen)**
- Open-Standard, von KeePassXC etabliert. Native Messaging via Browser-Host-Binary.
- Verschlüsselter Channel mit libsodium (curve25519 key exchange).
- Vorteil: Bestehende Browser-Extensions (`KeePassXC-Browser`) funktionieren potenziell out of the box, wenn unser Backend das Protokoll implementiert.
- Nachteil: Komplexes Protokoll, libsodium-Binding nötig.

**Option B — Lokaler HTTP-Server in der App**
- Tauri-App startet einen HTTP-Server auf `127.0.0.1:<random-port>` mit Token-Auth.
- Extension fragt den Port via Native-Messaging-Host oder festen Range ab.
- Vorteil: Einfacher zu debuggen.
- Nachteil: Firewall-Prompts, Port-Konflikte, weniger sicher gegen lokale Prozesse.

**Option C — Native Messaging direkt**
- Browser ruft ein kleines Host-Binary auf (per stdin/stdout), das mit der Haupt-App per IPC redet.
- Vorteil: Browser-Standard, kein offener Port.
- Nachteil: Host-Binary registrieren in `HKCU\Software\Google\Chrome\NativeMessagingHosts\...` — komplex bei Installation/Update.

**Vorschlag**: Option A. Das macht uns automatisch kompatibel mit KeePassXC-Browser-Extension; eigene Extension kann später folgen.

### 3.2 Extension Features (MVP)
- Auto-Fill in Login-Formularen (Username + Password).
- Save-Prompt nach erfolgreichem Login auf neuer Seite ("Diesen Login speichern?").
- Suche im Extension-Popup, Klick → fill or copy.
- Password-Generator im Popup.
- TOTP-Codes anzeigen/kopieren (falls Eintrag ein `otp:` Feld hat).
- HTTP Basic Auth Handling.

### 3.3 Wichtige Designentscheidungen
- **DB muss in der App unlocked sein**, Extension lockt nicht selbst. Wenn locked → Extension zeigt "App entsperren" + öffnet Hauptfenster.
- **Per-Site-Approval**: Erste Anfrage einer Domain muss in der App bestätigt werden ("Soll example.com auf Logins zugreifen?"). Wird pro DB-Eintrag gespeichert.
- **TLS für Communication-Channel**: Auch lokal, weil lokaler Browser bei einigen Browsern keine `http://localhost` ohne Weiteres erlaubt.

### 3.4 Schritte
1. Manifest V3 Boilerplate (TypeScript + Vite).
2. Native-Messaging-Host-Binary (kleines Rust-Crate in `extension-host/`).
3. Tauri-Plugin oder Command-Handler für die KeePassXC-Protokoll-Implementierung.
4. Popup-UI mit derselben shadcn/ui-Optik wie die App.
5. Veröffentlichung: Chrome Web Store ($5 einmalig), Edge Add-ons (kostenlos), Firefox AMO (kostenlos).

---

## Phase 4 — Sync via Cloud-Ordner (WebDAV/Dropbox/Nextcloud)

Pragmatischer Sync ohne eigenen Server. KeePassXC-Modell.

### 4.1 Grundannahme
DB-Datei liegt in einem User-verwalteten Sync-Ordner (Dropbox, OneDrive, Nextcloud, Syncthing, Google Drive Desktop). Sync passiert auf Datei-Ebene durch den Cloud-Client. **Unsere App muss nur**:
1. Externe Änderungen erkennen (File-Watcher).
2. Eigene Änderungen schreiben, ohne fremde Änderungen zu überschreiben → Merge.

### 4.2 Was schon da ist
- `lib/storage.ts` hat `liveUpdates` per DB.
- Es gibt `merge_database` und `check_database_changes` Commands (in `commands/database.rs`). Erste Implementierung scheint zu existieren — Tiefe prüfen.

### 4.3 Was fehlt (vermutlich)
- **Robuster File-Watcher**: `notify` crate in Rust, der bei Modifikation der DB-Datei automatisch `check_database_changes` triggert.
- **Konflikt-UI**: Wenn Merge nicht clean ist (gleicher Eintrag, beide Seiten geändert) → Dialog "Conflict in Eintrag X — Behalten: lokal / remote / neu kombinieren".
- **Lock-Datei-Awareness**: Vor Schreibvorgang `<db>.lock` erzeugen (wie KeePass es macht), prüfen ob fremder Lock existiert (anderes Gerät schreibt gerade).
- **Native WebDAV-Support (optional)**: Für User ohne Cloud-Sync-Client — App kann direkt mit WebDAV-Server reden. `reqwest` ist schon Dependency. Niedrige Priorität, da Dropbox/OneDrive Desktop-Clients der 99%-Use-Case sind.

### 4.4 Empfohlene Reihenfolge
1. File-Watcher → automatisches Reload bei externer Änderung.
2. Konflikt-Handling-UI verbessern.
3. Status-Indicator: "Synchronisiert vor 5 Sek" / "Konflikt — bitte lösen".
4. Optional: WebDAV-Client direkt im Backend.

### 4.5 Nicht-Ziele
- **Kein eigener Sync-Server**. Wenn das später kommt, ist es Phase 5.
- **Keine Cloud-Account-Integration in der App** (Dropbox OAuth etc.). User soll Cloud-Client separat installieren.

---

## Phase 5 (optional, weit weg) — Eigener Sync-Server

Nur wenn Phase 4 nicht reicht und du wirklich 1Password-Style willst. Massiver Aufwand:
- Server-Komponente (Rust, Axum?) mit End-to-End-Verschlüsselung.
- Account-System.
- Hosting + Monitoring + Backups.
- ToS, Datenschutzerklärung, evtl. Auftragsverarbeitungsverträge.
- Mobile-Apps (iOS, Android) werden dann fast erzwungen, sonst macht der Server wenig Sinn.

**Empfehlung**: Erst Phase 1-4 abschließen und 6 Monate live testen, bevor das überhaupt angedacht wird.

---

## Mobile (nicht in Roadmap, aber relevant)

Tauri 2.0 unterstützt iOS und Android. Nach Phase 4 (Sync funktioniert) könnte eine Read-Only-Mobile-App mit demselben Codebase sinnvoll sein — das ist der natürliche Pfad. Vor Phase 4 macht Mobile keinen Sinn, weil sonst nichts zum Synchronisieren da ist.

---

## Was als nächstes konkret zu tun ist

Nach deiner Freigabe von Phase 1 wäre die Implementierungsreihenfolge:
1. Close-to-Tray Default flippen + Migration (~30 min).
2. Autostart-Plugin integrieren + Settings-Toggle + Installer-Hook (~3 h).
3. Signing-Keys generieren + GitHub Actions Release-Workflow + Updater-Plugin (~4 h).
4. Manuell mit einem 1.0.1-Test-Release verifizieren, dass Update-Flow läuft.

Dann Phase 2 (Profiling). Dann Phase 3 (Extension). Dann Phase 4 (Sync).
