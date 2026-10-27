# InTime

**Einfache Zeiterfassung für deinen Arbeitstag – direkt auf dem Desktop.**

InTime zeigt dir in einem kleinen Fenster, woran du gerade arbeitest und wie lange schon. Du klickst auf ein Projekt, sobald du die Tätigkeit wechselst, schreibst kurz dazu, was du gemacht hast – und am Ende des Tages siehst du im Bericht genau, wie viel Zeit in welches Projekt geflossen ist.

> InTime ist eine Neuentwicklung in Rust, inspiriert von [KeepTime](https://github.com/doubleSlashde/KeepTime) von doubleSlash.

---

## Inhalt

- [Was InTime kann](#was-intime-kann)
- [Installation](#installation)
- [So benutzt du InTime](#so-benutzt-du-intime)
- [Einstellungen](#einstellungen)
- [Deine Daten](#deine-daten)
- [Häufige Fragen](#häufige-fragen)
- [Geplant für Version 2](#geplant-für-version-2)
- [Für Entwickler](#für-entwickler)

---

## Was InTime kann

- ⏱️ **Live-Zeitanzeige** – laufende Zeit des aktuellen Projekts, dazu die Gesamtzeit und die Arbeitszeit von heute (sekundengenau)
- 🗂️ **Projekte** mit eigener Farbe, Beschreibung und der Angabe, ob sie als Arbeitszeit zählen
- 📝 **Notizen** zu jeder Tätigkeit
- 🔁 **Zeit übertragen** – du hast vergessen umzuschalten? Buche die letzten Minuten einfach auf das neue Projekt
- 📊 **Tagesbericht** mit Kalender, farbiger Zeitleiste und Summen pro Projekt
- ✏️ **Einträge nachträglich bearbeiten**, löschen oder hinzufügen
- 📋 **Zusammenfassung kopieren** – zum Einfügen in Mails, Tickets oder Zeiterfassungssysteme
- 💾 **Automatisches Speichern** alle 15 Sekunden – auch bei einem Absturz geht praktisch nichts verloren
- 🎨 **Anpassbare Farben** und Layout
- 📦 **Export und Import** deiner Daten als JSON-Datei
- 🔔 **Tray-Symbol** in der Taskleiste, das die Farbe des aktiven Projekts annimmt

---

## Installation

> **Voraussetzung:** Windows 10 oder 11. Administratorrechte sind **nicht** nötig.

### Installieren

1. Öffne **PowerShell**: Drücke die `Windows`-Taste, tippe `PowerShell` und drücke Enter.
2. Kopiere diesen Befehl hinein und drücke Enter:

   ```powershell
   irm https://github.com/admirschwab/InTime/releases/latest/download/install.ps1 | iex
   ```

3. Beantworte die Fragen mit `j` (ja) oder `n` (nein) – oder drücke einfach Enter für die Vorgabe:
   - Verknüpfung auf dem Desktop anlegen? *(Vorgabe: nein)*
   - InTime beim Windows-Start automatisch öffnen? *(Vorgabe: ja)*
   - InTime jetzt starten? *(Vorgabe: ja)*

Fertig! InTime findest du ab jetzt im **Startmenü**.

Der Installer lädt die neueste Version von GitHub, legt alles in **`%LOCALAPPDATA%\InTime`** ab und trägt InTime unter *Einstellungen → Apps → Installierte Apps* ein.

> 💡 `irm` lädt das Installationsskript herunter, `iex` führt es aus. Du kannst dir das Skript vorher ansehen: [`installer/install.ps1`](installer/install.ps1).

### Aktualisieren

Einfach denselben Befehl noch einmal ausführen. Eine laufende InTime wird dabei sauber beendet – deine Daten bleiben erhalten.

### Deinstallieren

- **Startmenü → InTime → InTime deinstallieren**, oder
- **Einstellungen → Apps → Installierte Apps → InTime → Deinstallieren**

Der Uninstaller fragt, ob auch deine **Daten** gelöscht werden sollen. Vorgabe ist *nein* – dann bleiben Projekte und Zeiten erhalten und werden bei einer erneuten Installation automatisch wieder verwendet.

<details>
<summary><b>Für Fortgeschrittene: Optionen</b></summary>

```powershell
# Bestimmte Version installieren
& ([scriptblock]::Create((irm https://github.com/admirschwab/InTime/releases/latest/download/install.ps1))) -Version 0.1.1

# Ohne Rückfragen (nur die angegebenen Optionen werden umgesetzt)
& ([scriptblock]::Create((irm https://github.com/admirschwab/InTime/releases/latest/download/install.ps1))) -Silent -Autostart -DesktopShortcut

# Ohne Rückfragen deinstallieren (Daten bleiben erhalten bzw. mit -RemoveData alles löschen)
powershell -NoProfile -ExecutionPolicy Bypass -File "$env:LOCALAPPDATA\InTime\uninstall.ps1" -Silent
powershell -NoProfile -ExecutionPolicy Bypass -File "$env:LOCALAPPDATA\InTime\uninstall.ps1" -Silent -RemoveData
```

</details>

---

## So benutzt du InTime

### Das Hauptfenster

```
┌───────────────────┬──────────────────────────────┐
│ 🔍 Projekt suchen │ ● InTime          📊 ⚙  –  × │
│                   │                              │
│ ● Kundenprojekt   │ Kundenprojekt                │
│ ● Meetings        │ 01:23:45                     │
│ ● Pause           │ Heute 06:12:08 · Arbeit 05:40:02
│                   │ ┌──────────────────────────┐ │
│ + Neues Projekt   │ │ Was machst du gerade?    │ │
│                   │ └──────────────────────────┘ │
└───────────────────┴──────────────────────────────┘
```

- **Links** siehst du deine Projekte mit der heute darauf erfassten Zeit.
- **Rechts** siehst du das aktive Projekt, wie lange es schon läuft, und deine Gesamtzeit von heute.
- Das Fenster bleibt immer im Vordergrund. Du kannst es an einer freien Stelle anfassen und verschieben.

### Der typische Ablauf

1. **InTime starten** – es beginnt automatisch mit dem Standardprojekt (z. B. „Pause“).
2. **Projekt anklicken**, sobald du mit einer Tätigkeit beginnst.
3. **Notiz eintippen**, was du gerade machst (wird automatisch gespeichert).
4. **Nächstes Projekt anklicken**, wenn du wechselst. Die Notiz wird beim alten Eintrag gespeichert und das Feld ist wieder leer.
5. Am Ende des Tages den **Bericht** (📊) öffnen.

### Projekte verwalten

- **Neues Projekt:** Klicke unten in der Liste auf **+ Neues Projekt**.
- **Rechtsklick auf ein Projekt** öffnet ein Menü:
  - **Wechseln** – zu diesem Projekt wechseln
  - **Wechseln mit Zeitübertrag…** – die letzten *n* Minuten werden schon diesem Projekt zugerechnet
  - **Bearbeiten** – Name, Farbe, Beschreibung ändern
  - **Nach oben / Nach unten** – Reihenfolge ändern
  - **Löschen** – das Projekt verschwindet aus der Liste, bereits erfasste Zeiten bleiben im Bericht erhalten
- Das **Standardprojekt** wird beim Start automatisch ausgewählt und kann nicht gelöscht werden. Du kannst aber jedes andere Projekt zum Standard machen.
- Projekte, bei denen **„Zählt als Arbeitszeit“** nicht angehakt ist (z. B. Pause), werden im Bericht getrennt ausgewiesen.

### Tastenkürzel

| Kürzel | Funktion |
| --- | --- |
| `Strg + F` | Projektsuche fokussieren |
| `Enter` (in der Suche) | Zum ersten gefundenen Projekt wechseln |
| `Esc` | Suche leeren bzw. Dialog schließen |

### Der Tagesbericht

Über 📊 im Hauptfenster (oder Rechtsklick auf das Tray-Symbol → **Bericht**):

- **Kalender** links – Tage mit erfasster Zeit sind mit einem Punkt markiert.
- **Zeitleiste** – zeigt farbig, wann du woran gearbeitet hast. Fahre mit der Maus darüber für Details.
- **Projektliste** – Summe und Notizen pro Projekt. Klicke auf ein Projekt, um die einzelnen Einträge zu sehen, zu **bearbeiten** ✏️ oder zu **löschen** 🗑️.
- **+ Eintrag** – nachträglich eine Zeit erfassen.
- **Kopieren** – kopiert eine Zusammenfassung des Tages in die Zwischenablage.

Der gerade laufende Eintrag ist mit **läuft** markiert und kann erst bearbeitet werden, wenn du das Projekt gewechselt hast.

---

## Einstellungen

Über ⚙ im Hauptfenster:

| Bereich | Was du einstellen kannst |
| --- | --- |
| **Farben** | Hintergrund, Text und Hover-Farben – jederzeit auf Standard zurücksetzbar |
| **Darstellung** | Projektliste rechts statt links · Liste ausblenden, wenn die Maus das Fenster verlässt · Fensterposition merken |
| **Verhalten** | Beim Wechseln nach einer Notiz fragen (optional nur bei Arbeitsprojekten) · Vor dem Beenden nachfragen |
| **Heimat API** | Kommt in Version 2 |
| **Daten** | Export und Import |

---

## Deine Daten

- Alles wird **lokal auf deinem PC** gespeichert – nichts wird ins Internet übertragen.
- Alles liegt in einem Ordner: **`%LOCALAPPDATA%\InTime`**
  (Zum Öffnen `Windows + R` drücken, den Pfad einfügen und Enter drücken.)

  ```
  %LOCALAPPDATA%\InTime\
  ├── InTime.exe        Programm
  ├── uninstall.ps1     Uninstaller
  ├── install.json      Infos zur Installation (Version, Datum, Verknüpfungen)
  ├── data\intime.db    deine Projekte, Zeiten und Einstellungen
  ├── logs\             Protokolldateien (hilfreich bei Fehlern)
  └── webview\          Zwischenspeicher der Oberfläche
  ```
- **Backup:** Einstellungen → Daten → **Exportieren** erzeugt eine JSON-Datei.
- **Wiederherstellen:** Einstellungen → Daten → **Importieren**. ⚠️ Dabei werden *alle* aktuellen Daten ersetzt.
- Bei Updates von InTime bleiben deine Daten automatisch erhalten.

---

## Häufige Fragen

**Muss ich InTime vor dem Herunterfahren schließen?**
Nein. InTime speichert alle 15 Sekunden automatisch. Wird der PC einfach ausgeschaltet, fehlen höchstens diese Sekunden.

**Ich habe vergessen, das Projekt zu wechseln. Was nun?**
Rechtsklick auf das richtige Projekt → **Wechseln mit Zeitübertrag…** und die Minuten angeben. Oder später im Bericht den Eintrag bearbeiten.

**Wo ist das Fenster hin?**
Klicke auf das farbige InTime-Symbol unten rechts in der Taskleiste (ggf. unter dem Pfeil `^`).

**Warum kann ich ein Projekt nicht löschen?**
Das Standardprojekt kann nicht gelöscht werden. Markiere zuerst ein anderes Projekt als Standard.

---

## Geplant für Version 2

- 🔗 **Heimat-Anbindung** – Projekte importieren und Zeiten übertragen
- ⌨️ **Globales Tastenkürzel** – Projekt wechseln, ohne das Fenster zu suchen
- 🌐 **REST-API** für eigene Automatisierungen

Ideen oder Fehler gefunden? Erstelle gerne ein [Issue](../../issues).

---

## Für Entwickler

InTime basiert auf [Tauri 2](https://tauri.app) (Rust) mit einer Oberfläche in [Svelte 5](https://svelte.dev) + TypeScript. Daten liegen in einer SQLite-Datenbank.

### Voraussetzungen

- [Rust](https://rustup.rs) (stable, MSVC-Toolchain)
- [Node.js](https://nodejs.org) 20 oder neuer
- [Microsoft C++ Build Tools](https://visualstudio.microsoft.com/de/visual-cpp-build-tools/) („Desktopentwicklung mit C++“)
- WebView2 (bei Windows 10/11 bereits installiert)

### Befehle

```bash
npm install            # Abhängigkeiten installieren
npm run tauri dev      # App im Entwicklungsmodus starten (erster Start dauert einige Minuten)
npm run package        # Release-ZIP lokal bauen → release/InTime-windows.zip
npm run check          # TypeScript/Svelte prüfen
cd src-tauri && cargo test   # Backend-Tests
```

### Projektstruktur

```
src/                    Oberfläche (Svelte)
  views/                Fenster: Haupt, Bericht, Einstellungen, Projekt
  components/           Kalender, Zeitleiste, Dialoge …
  lib/                  API-Aufrufe, Formatierung, Einstellungen
src-tauri/              Backend (Rust)
  src/repo.rs           Datenbanklogik (inkl. Tests)
  src/db.rs             Schema und Migrationen
  src/commands.rs       Schnittstelle zur Oberfläche
  src/windows.rs        Fenster und Tray-Symbol
  src/indicator.rs      Projektfarbe in Tray und Taskleiste
  src/paths.rs          Speicherorte unter %LOCALAPPDATA%\InTime
installer/              PowerShell-Installer und -Uninstaller
scripts/package.mjs     Packt Exe und Uninstaller zur Release-ZIP
.github/workflows/      CI und automatische Releases
```

### Ein Release veröffentlichen

Releases baut **GitHub Actions** automatisch ([`.github/workflows/release.yml`](.github/workflows/release.yml)):

1. Version in `package.json`, `src-tauri/Cargo.toml` und `src-tauri/tauri.conf.json` erhöhen und committen.
2. Tag anlegen und pushen:
   ```bash
   git tag v0.1.1
   git push origin v0.1.1
   ```
3. Nach ca. 10 Minuten steht unter **Releases** die neue Version mit `InTime-windows.zip` und `install.ps1` – der Installationsbefehl liefert ab dann automatisch diese Version.

Zusätzlich prüft [`ci.yml`](.github/workflows/ci.yml) bei jedem Push auf `main` und jedem Pull Request Typen und Tests.
