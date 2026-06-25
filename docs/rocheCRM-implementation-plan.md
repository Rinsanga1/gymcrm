# RocheCRM — Implementation Plan (Ralph Loop)

> Drives an iterative build loop. Companion to
> [rocheCRM-mvp-prd.md](rocheCRM-mvp-prd.md). Work top-to-bottom; each milestone
> is a vertical slice that compiles and runs.

Brief: a Windows-native gym CRM in **Rust + egui/eframe**, shipping as one
portable `.exe` with a local SQLite DB beside it. Tracks members, monthly dues,
merchandise sales, and expenses, with an earnings dashboard.

## Goals
- Single self-contained `.exe` (`x86_64-pc-windows-gnu` — GNU toolchain, no
  Visual Studio needed), no installer/runtime.
- Portable: `roche.db` lives next to the `.exe`; auto-backups in `backups/`.
- Replace the owner's Google Sheet: import members, track monthly dues.
- Smooth at 5,000+ members (virtualized tables, indexed queries).
- Every iteration leaves the app **compiling and runnable** (`cargo run`).

## Tech stack (decided)
- **GUI:** `eframe` + `egui` (immediate mode); `egui_extras` (TableBuilder for
  virtualized rows); `egui_plot` (revenue trend chart).
- **DB:** `rusqlite` with the **bundled** feature (SQLite compiled in, no DLL).
- **Support:** `serde`, `chrono` (dates / `period_month`), `csv` (import/export).
- **Build:** GNU toolchain (`x86_64-pc-windows-gnu`, Rust 1.96, installed);
  `winresource` (app icon + version metadata) for the release `.exe`.
- **PATH note:** `cargo` lives in `C:\Users\katto\.cargo\bin` (not yet on the
  global PATH); prepend it in each shell: `export PATH="$PATH:/c/Users/katto/.cargo/bin"`.

## Architecture
- `core/` — pure Rust, no UI. Domain types (Member, Payment, Product, Sale,
  SaleItem, Expense, Settings) + a `Repository` over SQLite + derived logic
  (Paid/Due for current month, dashboard aggregates). **Unit-testable headless.**
- `ui/` — egui views per module, calling into `core`. No business logic here.
- `main.rs` — eframe bootstrap, DB path resolution (next to exe), backup-on-exit.

---

## Milestones & Checklist

### M0 — Skeleton (compiles, window opens)
- [ ] `cargo init` binary crate `roche_crm`; add deps above; commit `Cargo.toml`.
- [ ] eframe app boots: 1100x720 window titled "RocheCRM", dark theme.
- [ ] Left **sidebar** with logo text + nav items (Dashboard, Members,
      Merchandise, Expenses, Settings) and a selected-state; main panel switches
      view on click via an `enum View`.
- [ ] Resolve DB path **next to the `.exe`** (`std::env::current_exe`), fall back
      to CWD in `cargo run`. Open/create `roche.db`.
- [ ] Run schema migration on startup (all tables, empty). App runs with empty
      DB without panicking.

### M1 — Data layer (headless, tested)
- [ ] SQLite schema + migrations for: `members`, `payments`, `products`,
      `sales`, `sale_items`, `expenses`, `settings`. Indexes on
      `members(name)`, `members(phone)`, `payments(period_month)`,
      `payments(member_id)`, `sales(date)`.
- [ ] `Repository` CRUD for each entity (insert/update/delete/list/search).
- [ ] Derived logic: `is_paid(member, period_month)`, `due_members(month)`,
      `member_status` (Active+Paid / Active+Due / Inactive).
- [ ] Settings row seeded with `default_monthly_fee=1500`, `currency="Rs"`,
      `gym_name`.
- [ ] **Unit tests** for status derivation, multiple-payments-per-month,
      inactive exclusion from dues. `cargo test` green.

### M2 — Members (core slice)
- [ ] Members table view: virtualized (`TableBuilder`), columns name / phone /
      status (Paid·Due) / join date / actions. Smooth with 5k seeded rows.
- [ ] Search box filters by name/phone at the **DB level** (indexed).
- [ ] Add member dialog: **name required**, phone/join-date/notes optional
      (join date prefilled today).
- [ ] On save → **prefilled payment dialog** (amount = Rs 1,500, editable;
      `period_month` = current). Saving logs payment + marks Paid. **Skippable.**
- [ ] Record-payment action on any member (same dialog) for the current month.
- [ ] Active/Inactive toggle; default roster shows Active only; separate
      **Inactive view** to review/reactivate.
- [ ] Edit + delete member/payment with a confirm dialog on delete.

### M3 — Merchandise
- [ ] Products view: flat list (name, price, stock); add/edit/delete.
- [ ] **Record sale** dialog: pick product(s) + qty (line items), anonymous;
      computes total, **decrements stock**, writes `sales`+`sale_items`.
- [ ] **Low-stock flag** (e.g. stock <= threshold) surfaced in list + dashboard.
- [ ] Edit/delete products and sales (delete confirm). Stock adjusts on edit.

### M4 — Expenses
- [ ] Expenses view: flat list (amount, date, note); add/edit/delete (no
      categories). Delete confirm.

### M5 — Dashboard
- [ ] **Time-range chips:** All Time / Today / This Week / **This Month
      (default)** / This Quarter / This Year / Custom — filter all numbers.
- [ ] KPI cards: Total income, Membership income, Merchandise income (+units),
      Pending Dues (count), Total expenses, Net earnings, Total/Active/Due
      members.
- [ ] **Clickable "Due this month" list** → opens payment dialog for that member.
- [ ] **One revenue-trend line chart** (`egui_plot`) over the selected period.
- [ ] Low-stock indicator. Clean empty states when no data.

### M6 — Settings, Import/Export, Backups
- [ ] Settings view: edit gym name, `default_monthly_fee`, currency.
- [ ] **CSV import** (members): file picker → column-mapping (Name, Phone) →
      bulk insert. Payment history NOT imported (fresh tracking).
- [ ] **CSV export** for members, payments, sales, expenses.
- [ ] **Auto-backup on app close:** copy `roche.db` → `backups/roche_<ts>.db`,
      keep last 7. Manual "Backup now" + "Restore" buttons.

### M7 — Release packaging
- [ ] App icon + version metadata via `winresource`; window/taskbar icon set.
- [ ] `cargo build --release` (GNU toolchain) → single `.exe`.
- [ ] **Clean-machine test:** copy just the `.exe` to a fresh folder / USB, run,
      add a member, record a payment, restart → data persists. No prereqs.
- [ ] Write a short `README` (download, double-click, where data lives).

---

## Definition of Done (MVP)
From a single portable `.exe`: import the existing sheet, register a member with
a prefilled payment, see who's Due this month, record monthly payments, log a
merch sale and an expense, view This-Month earnings + net, and export to CSV —
all offline, with auto-backups, on a clean Windows machine.

## Verification
- Per milestone: `cargo build` + `cargo test` green; `cargo run` exercises the
  new slice manually (note what was clicked/observed).
- Record commands run, files touched, and any seeded-data perf checks (5k rows)
  here as evidence.

## Notes
- Keep `core` UI-free so logic stays testable without egui.
- Don't gold-plate: defer per-member fees, partial-balance ledger, attendance,
  receipts, per-member purchase history, and richer charts to v1.1 (see PRD).
- Risk: running from a write-blocked folder (e.g. `Program Files`) breaks the
  portable DB — out of scope for MVP; revisit with `%APPDATA%` fallback if hit.
