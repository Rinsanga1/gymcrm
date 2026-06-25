# RocheCRM — Product Requirements Document (PRD)

> Owner: dev@mustbesocial.com
> Status: Draft v1
> Last updated: 2026-06-25

---

## 1. Summary

RocheCRM is a **Windows-native desktop CRM for gym owners**, inspired by the
layout and feature set of Timeline CRM (sidebar navigation + dashboard with KPI
cards and charts) but purpose-built for running a gym instead of generic B2B
sales.

It replaces the spreadsheet ("Google Sheets is getting hard to manage") with a
single application that the owner **downloads as one `.exe` and starts using
immediately** — no installer, no account, no cloud, no setup. All data lives in
a local database file on the owner's machine.

### One-line pitch
> A free, offline, single-file Windows app that turns a gym's messy member
> spreadsheet into a real CRM — members, memberships, check-ins, payments, and
> reports — built in Rust so it's fast and dependency-free.

---

## 2. Goals & Non-Goals

### Goals
- Ship a **single self-contained `.exe`** (no installer, no .NET/VC++ runtime,
  no Python, no browser engine required) that runs on a clean Windows 10/11 box.
- Let a non-technical gym owner go from download → tracking members in **under 5
  minutes**.
- Cover the daily gym workflows currently done in spreadsheets: member roster,
  membership plans & expiry, attendance/check-ins, payments due, and leads
  (trial sign-ups).
- Be **fast and lightweight** (low RAM, instant startup) — a key reason for Rust
  over Electron.
- Work **fully offline**; data never leaves the machine unless the user exports.

### Non-Goals (v1)
- No cloud sync / multi-device / web access.
- No multi-user accounts or role permissions (single owner-operator assumed).
- No built-in payment processing (Stripe etc.) — we *record* payments, not take
  them.
- No mobile app.
- No email/SMS marketing automation (may come later).
- No member-facing booking portal.

---

## 3. Target User & Context

- **Primary user:** a single gym owner / front-desk operator.
- **Current tool:** Google Sheets, becoming unmanageable as member count grows.
- **Technical level:** comfortable with spreadsheets, *not* with installers,
  servers, or databases. Must be "double-click and it works."
- **Environment:** one Windows PC at the gym front desk. Possibly no reliable
  internet.

### Core jobs-to-be-done
1. "Who are my members and what plan are they on?"
2. "Whose membership is expiring / has expired?"
3. "Who owes me money this month?"
4. "Did this person check in today?" / "How often do they come?"
5. "How many trial leads converted to paying members?"
6. "How is my revenue trending?"

---

## 4. Product Overview & Navigation

Mirror the reference layout: a **dark left sidebar** with the RocheCRM logo, a
list of modules, and an account block at the bottom; a **light main content
area** with a page title, filter chips (All Time / Today / This Week / This
Month / This Quarter / This Year / Custom), KPI cards, and charts.

### Sidebar modules (gym-adapted)

| # | Module | Replaces Timeline's | Purpose |
|---|--------|---------------------|---------|
| 1 | **Dashboard** | Dashboard | KPIs + charts overview |
| 2 | **Leads** | Leads | Trial/walk-in prospects to convert |
| 3 | **Members** | Customers | The member roster (core) |
| 4 | **Memberships** | Deals/Products | Plans (Monthly, Quarterly, Annual, PT packs) + active subscriptions |
| 5 | **Check-ins** | (new) | Daily attendance log |
| 6 | **Payments** | Invoices | Money owed / received per member |
| 7 | **Tasks** | Tasks | Follow-ups, call-backs, reminders |
| 8 | **Reports** | Reports | Revenue, attendance, retention, expiries |
| 9 | **Settings** | Settings | Gym name, currency, plans, backup/export |
| 10 | **About** | About | App/version info |

> Note: Timeline's *Quotations* module is dropped for v1 (not relevant to a
> gym). *Products & Services* is folded into **Memberships**.

---

## 5. Functional Requirements

### 5.1 Dashboard
- KPI cards (adapt the 8 cards from the reference):
  - **Total Members** (and # new this period)
  - **Active Memberships** (currently valid)
  - **Expiring Soon** (next 7 days)
  - **Revenue (period)**
  - **Outstanding / Money Owed**
  - **Check-ins (period)**
  - **Lead Conversion Rate**
  - **Active Leads / Trials**
- Time-range filter chips (All Time / Today / Week / Month / Quarter / Year /
  Custom).
- Charts: **Revenue trend** (daily/weekly/monthly toggle), **Check-ins over
  time**, **Memberships by plan**, **Leads by source**.
- Empty states ("No data yet") matching the reference look.

### 5.2 Members (core module)
- Table view: name, phone, email, plan, status (Active / Expiring / Expired /
  Frozen), join date, expiry date, balance owed.
- Add / edit / delete member. Fields: full name, phone, email, gender, DOB,
  emergency contact, join date, notes, photo (optional), assigned plan.
- Search + filter (by status, plan, expiry).
- Member detail view: profile, current membership, payment history, check-in
  history, tasks.
- **Status auto-computed** from membership expiry date.

### 5.3 Memberships / Plans
- Define plans: name, price, duration (days/months), type (recurring vs.
  one-off PT pack).
- Assign a plan to a member → creates an active membership with start/expiry.
- Renew / freeze / cancel membership.
- Expiry tracking feeds Dashboard "Expiring Soon" and Member status.

### 5.4 Check-ins
- Quick check-in: search member → one click to log attendance (timestamped).
- Daily attendance list.
- Per-member attendance history + frequency stats (for retention insight).

### 5.5 Payments
- Record a payment against a member/membership: amount, date, method (cash/card/
  transfer), note.
- Track outstanding balance per member.
- Mark membership paid/unpaid; "Money Owed" KPI is the sum of outstanding.
- Generate a simple printable/PDF **receipt** (stretch goal).

### 5.6 Leads
- Capture trial/walk-in prospects: name, contact, source (Walk-in, Instagram,
  Referral, Google, etc.), status (New / Contacted / Trial / Won / Lost).
- Convert a Lead → Member (carries over their info).
- Conversion rate feeds Dashboard.

### 5.7 Tasks
- Simple to-do list with due dates, optionally linked to a member ("call about
  renewal"). Pending tasks count on Dashboard.

### 5.8 Reports
- Revenue report (by period, by plan).
- Attendance report.
- Membership expiry report (export list of expiring/expired members).
- Retention / churn snapshot.
- **Export to CSV/PDF** for any report.

### 5.9 Settings & Data
- Gym profile: name, logo, currency, contact.
- Manage plans, lead sources, payment methods.
- **Import from CSV** (critical: migrate the existing Google Sheet on day one).
- **Export all data to CSV**.
- **Backup / Restore** the database file (one-click copy to a chosen folder).
- Light/dark theme (optional).

---

## 6. Non-Functional Requirements

| Area | Requirement |
|------|-------------|
| **Distribution** | Single `.exe`, no installer, no external runtime. User downloads and double-clicks. |
| **Footprint** | App + DB file only. Target startup < 1s, idle RAM < 150 MB. |
| **Offline** | 100% functional with no internet. |
| **Data safety** | Local DB with automatic periodic backups + manual backup/restore. No silent data loss. |
| **Persistence** | Data stored in a single file under `%APPDATA%\RocheCRM\` (or next to the exe in portable mode). |
| **Privacy** | No telemetry, no accounts, no data leaves the device by default. |
| **Platform** | Windows 10 & 11 (x64). |
| **Resilience** | Survive force-close without corrupting data (transactional writes). |

---

## 7. Technical Approach

> Background research lives in the sibling docs: see
> [areweguiyet.md](areweguiyet.md),
> [2025-survey-rust-gui-boringcactus.md](2025-survey-rust-gui-boringcactus.md),
> [next-dozen-rust-guis-raphlinus.md](next-dozen-rust-guis-raphlinus.md),
> [rust-for-windows-msdocs.md](rust-for-windows-msdocs.md), and
> [native-windows-gui.md](native-windows-gui.md).

### 7.1 Language & distribution
- **Rust**, compiled to a single statically-linked Windows `.exe`.
- Target `x86_64-pc-windows-msvc`. Embed an app icon + version metadata.
- No installer for v1: distribute the raw `.exe` (plus optional zip). The DB is
  created on first run.

### 7.2 GUI library — recommendation: **egui (via eframe)**
Rationale for a solo build + single-exe goal:
- Pure Rust, **compiles to one self-contained exe** with no system GUI deps —
  exactly the "download and run" requirement.
- Immediate-mode: very fast to build data tables, forms, and dashboards; low
  ceremony for a single developer.
- The 2025 survey calls it the choice if you want to "write only regular Rust"
  with no DSL/macros.
- Trade-off: look is functional rather than fully OS-native, and accessibility
  is weaker. Acceptable for an internal front-desk tool.

**Alternatives considered:**
- **Slint** — nicest polished/declarative UI (closest to the reference
  screenshot's look) with good tooling; trade-off is its own DSL. Strong second
  choice if visual polish is the priority.
- **iced** — Elm-style, clean native-ish feel; good but more boilerplate and an
  open accessibility issue.
- **Tauri** — would hit the look easily but bundles a webview ("Diet
  Electron") and complicates the single-exe/offline story. Rejected for v1.
- **native-windows-gui (NWG)** — truly native Win32 widgets, but lower-level and
  more tedious for a rich dashboard. Rejected for v1.

> Decision: **start with egui/eframe** for speed-to-MVP and the clean single-exe
> story. Re-evaluate Slint if visual polish becomes a priority.

### 7.3 Data storage
- **SQLite** embedded (via `rusqlite`, bundled feature so SQLite is compiled
  in — no external DLL).
- Single `.db` file = trivially backup-able and matches the "just a file" mental
  model the user already has from Sheets.
- Schema migrations handled in-app on startup.

### 7.4 Suggested crates
- `eframe` / `egui` — GUI (or `slint`).
- `rusqlite` (bundled) — database.
- `serde` — serialization.
- `chrono` / `time` — dates & expiry math.
- `csv` — import/export from the existing spreadsheet.
- `rust_xlsxwriter` or `printpdf` — receipts/report export (stretch).
- `egui_plot` — dashboard charts.

### 7.5 Architecture sketch
- **Core layer** (pure Rust, no UI): domain models (Member, Membership, Plan,
  Payment, CheckIn, Lead, Task) + a repository over SQLite. Unit-testable
  without the GUI.
- **UI layer**: egui views per module, calling into the core repository.
- Keep business logic (expiry status, balances, conversion rate) in core so it's
  testable and reusable.

---

## 8. Data Model (high level)

- **Member**: id, name, phone, email, gender, dob, emergency_contact, join_date,
  photo_path, notes, status (derived).
- **Plan**: id, name, price, duration_days, type (recurring | pack), active.
- **Membership**: id, member_id, plan_id, start_date, end_date, state (active |
  frozen | cancelled | expired).
- **Payment**: id, member_id, membership_id, amount, date, method, note.
- **CheckIn**: id, member_id, timestamp.
- **Lead**: id, name, contact, source, status, created_date, converted_member_id.
- **Task**: id, title, due_date, done, member_id (nullable).
- **Settings**: gym_name, currency, logo_path, theme, sources[], methods[].

---

## 9. Milestones / Phasing

### Phase 0 — Skeleton (foundation)
- Rust project, egui window, sidebar + dashboard shell matching the reference
  layout. SQLite wired up, DB file created on first run. Builds to one `.exe`.

### Phase 1 — MVP (the spreadsheet replacement)
- **Members** CRUD + table + search.
- **Plans/Memberships** with expiry + auto status.
- **Payments** + outstanding balance.
- **CSV import** (migrate the existing Google Sheet).
- Dashboard KPI cards wired to real data.
- Backup/restore + CSV export.
- ✅ *Success: owner imports their sheet and manages members entirely in-app.*

### Phase 2 — Daily operations
- **Check-ins** + attendance history.
- **Leads** + convert-to-member + conversion rate.
- **Tasks** + dashboard pending count.
- Dashboard charts (revenue, check-ins, plans).

### Phase 3 — Reporting & polish
- **Reports** module + PDF/CSV exports.
- Printable receipts.
- Expiring-soon notifications/reminders.
- Theming, icon, version metadata, polish pass.

### Phase 4 (later / optional)
- Multi-user roles, cloud backup, member self-check-in kiosk mode, SMS/email
  reminders.

---

## 10. Success Metrics
- Time from download to first member tracked: **< 5 min**.
- Existing spreadsheet fully imported with **zero data loss**.
- App startup **< 1s**; single `.exe` runs on a clean Windows install with **no
  prerequisites**.
- Owner stops using Google Sheets for member management.

---

## 11. Open Questions
1. **Visual fidelity:** how close must v1 look to the Timeline screenshot? If
   "very close," lean toward **Slint** over egui.
2. **Currency / locale:** single currency in Settings, or per-member?
3. **Portable vs. installed:** DB next to the `.exe` (portable, runs from USB) or
   in `%APPDATA%`? (Recommend offering both.)
4. **Receipts:** needed in MVP or Phase 3?
5. **Multiple gyms / branches:** ever needed, or strictly one location?
6. **Branding:** logo and color palette for RocheCRM (the reference uses a
   blue/dark-navy theme).
