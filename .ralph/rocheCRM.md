# RocheCRM — Build (Ralph Loop)

Windows-native gym CRM in Rust + egui/eframe, shipping as one portable .exe with a local SQLite DB beside it. Full spec: docs/rocheCRM-mvp-prd.md. Detailed plan: docs/rocheCRM-implementation-plan.md.

Shell note: cargo is at C:\Users\katto\.cargo\bin (not on global PATH). Start every shell with: export PATH="$PATH:/c/Users/katto/.cargo/bin". Toolchain: stable x86_64-pc-windows-gnu (no Visual Studio).

## Goals
- Single self-contained .exe (GNU toolchain), no installer/runtime.
- Portable: roche.db beside the .exe; auto-backups in backups/.
- Replace a Google Sheet: import members, track monthly dues.
- Smooth at 5,000+ members (virtualized tables, indexed queries).
- Every iteration leaves the app compiling and runnable.

## Tech stack (decided)
- GUI: eframe + egui, egui_extras (TableBuilder), egui_plot.
- DB: rusqlite (bundled feature). Support: serde, chrono, csv. Build: winresource.

## Architecture
- core/ — pure Rust, no UI: domain types + Repository over SQLite + derived logic (Paid/Due, dashboard aggregates). Unit-testable headless.
- ui/ — egui views per module, calling core. No business logic.
- main.rs — eframe bootstrap, DB path beside exe, backup-on-exit.

## Checklist

### M0 — Skeleton  ✅ COMPLETE
- [x] Cargo project roche_crm (lib + bin); deps added (eframe/egui/egui_extras/egui_plot 0.34, rusqlite 0.40 bundled, serde, chrono, csv, rfd).
- [x] eframe app boots: 1100x720 window "RocheCRM", dark theme.
- [x] Sidebar nav (Dashboard, Members, Merchandise, Expenses, Settings) with selected state; main panel switches via enum View (src/app.rs).
- [x] DB path next to .exe (current_exe), CWD fallback; open/create roche.db (src/core/db.rs).
- [x] Schema migration runs on startup; app runs with empty DB, no panic.

### M1 — Data layer (tested)  ✅ COMPLETE
- [x] Schema+migrations + all indexes (src/core/db.rs).
- [x] Repository CRUD for members, payments, products, sales+items, expenses, settings (src/core/repo.rs).
- [x] Derived: is_paid, due_members, member_status; + income/expense/merch aggregates for the dashboard.
- [x] Seed settings: default_monthly_fee=1500, currency=Rs, gym_name.
- [x] 11 unit tests (src/core/tests.rs): binary status, partial-pay clears Due, multi-payment month, inactive excluded from dues, search name/phone, sale stock decrement, low-stock, expense range, cascade delete. **cargo test: 11 passed.**
- [x] Added date helpers (src/core/dates.rs: current_month, today, month_of).

### M2 — Members  ✅ COMPLETE
- [x] Virtualized members table via TableBuilder (name/phone/status/join/actions); rows fetched once per dirty cycle.
- [x] DB-level search by name/phone (Repository::search_members, LIKE on indexed cols).
- [x] Add-member dialog: name required; phone/join/notes optional; join defaults to today.
- [x] On save → prefilled payment dialog (default_monthly_fee from settings, month=current); Save logs payment & marks Paid; Skip closes without writing.
- [x] Record-payment action on any member for current month (one-query paid_member_ids drives row status).
- [x] Active/Inactive toggle inline; "Show inactive" checkbox switches roster source.
- [x] Edit/delete member & payment with confirm-delete dialog; cascade verified by M1 tests.

### M3 — Merchandise  ✅ COMPLETE
- [x] Products tab: flat table (name/price/stock/status/actions) with add/edit/delete + confirm-delete dialog.
- [x] Sales tab: record-sale dialog with multi-line items (combo per product + qty), anonymous; live total; transactional write of sales+sale_items; stock decrement.
- [x] Low-stock flag inline (orange "N (low)") and header banner counting products at ≤ 5; dashboard surface deferred to M5.
- [x] Edit sale: restores old stock then re-applies new lines in one tx (Repository::update_sale). Delete sale also restores stock (Repository::delete_sale).

### M4 — Expenses  ✅ COMPLETE
- [x] Expenses view: flat table (date/amount/note/actions) with add/edit/delete + confirm-delete dialog. Same dirty-flag cache pattern as Members/Merchandise.

### M5 — Dashboard
- [x] Time chips: All Time/Today/This Week/This Month(default)/This Quarter/This Year/Custom; Custom shows YYYY-MM-DD inputs. Range from `dates::Period::range()`.
- [x] KPI cards: total income, membership income, merch income(+units), expenses, net earnings, total/active members, pending dues.
- [x] Clickable "Due this month" list → payment dialog (inline on dashboard, prefilled with default_monthly_fee + current month; saves payment, member drops off the list next frame).
- [x] Revenue-trend line chart over selected period. **Switched from egui_plot to a hand-painted polyline** — egui_plot 0.34.1 transitively pulls egui 0.33, conflicting with our egui 0.34. Removed `egui_plot` from Cargo.toml. Painter-based chart shows daily totals + summary (Total / Peak day / day count).
- [x] Low-stock indicator on dashboard; empty states for chart ("No data in this range") and dues list ("No members currently due").

### M6 — Settings / Import / Export / Backups  ✅ COMPLETE
- [x] Settings: edit gym name, default_monthly_fee, currency; Save writes to settings table with conflict-update.
- [x] CSV import (members): rfd file picker → case-insensitive Name/Phone column match → transactional bulk insert; reports imported/skipped counts.
- [x] CSV export: members, payments (joined with member name), sales (one row per line item), expenses. Each writes via `csv::Writer` and reports row count.
- [x] Auto-backup on close: `App::on_exit` checkpoints WAL + copies roche.db → backups/roche_<YYYYMMDD-HHMMSS>.db; prune keeps last 7. Settings has "Backup now" + a backup list + Restore-from-file (queued via `roche.db.pending-restore`, applied on next startup so we don't fight the live connection).

### M7 — Release
- [ ] App icon + version metadata (winresource); window/taskbar icon. **Deferred:** `winresource` is not in the offline crates cache and network to crates.io is flaky per the toolchain notes; needs a connected build to land. Window title is set correctly via `ViewportBuilder::with_title`.
- [x] `cargo build --release --offline` → **single 13 MB `target/release/roche_crm.exe`** (statically links rusqlite; no runtime deps).
- [x] Clean-folder test: copied just the .exe to a fresh empty dir, launched twice. First run created `roche.db` + WAL + SHM next to the .exe. Verified DB persistence + settings seed + payment-status derivation by inserting a member + payment via a one-shot `roche_crm::core::Repository` smoke binary against the same DB file; member count went 1 → 2 across runs, `is_paid` returned true.
- [x] README written (download/run, data location, restore flow, build-from-source notes).

## Verification
- Per milestone: cargo build + cargo test green; cargo run exercises the new slice. Record commands, files touched, perf checks (5k rows) below.

## Notes
### Toolchain setup (done)
- Installed Rust 1.96 **GNU** toolchain (rustup, no VS). GNU self-contained dir
  lacked an assembler, so dlltool failed. Fixed by installing full **MinGW-w64**
  (winlibs GCC 16.1.0 UCRT) at `C:\Users\katto\tools\mingw64`.
- **Every build/test/run shell MUST prepend both dirs to PATH:**
  `export PATH="/c/Users/katto/tools/mingw64/bin:/c/Users/katto/.cargo/bin:$PATH"`
- Network to crates.io is flaky; deps are fetched. Use `cargo build --offline`.

### M0 verification
- `cargo build --offline` → Finished, **0 warnings**.
- Timed run created `target/debug/roche.db` + WAL (migrations + seed ran). App
  window opened without panic.

### M1 verification
- `cargo test --offline` → **11 passed; 0 failed**. Build clean.
- Repo is UI-free; tests run headless against an in-memory SQLite.

### M2 verification
- `cargo build --offline` → Finished, **0 warnings**.
- `cargo test --offline` → 11 passed (no regressions; M2 is UI, exercised via existing repo tests).
- MembersState holds search/show_inactive/dialog/cached rows/paid set; reload is dirty-flag driven so a 5k roster doesn't re-query every frame.

### M3 verification
- `cargo build --offline` → Finished, **0 warnings**.
- `cargo test --offline` → **11 passed; 0 failed** (no regressions; existing sale_decrements_stock_and_totals + low_stock tests cover the new repo paths' core behaviour).
- MerchandiseState mirrors MembersState: Products/Sales tabs, dirty-flag cache, dialog enum (Product/Sale/ConfirmDelete×2).
- Edit/delete sale stock adjustment lives in repo (transactional) so it stays testable; the UI just calls it.

### M4 + M5 (partial) verification
- `cargo build --offline` → Finished, **0 warnings**.
- `cargo test --offline` → **11 passed; 0 failed**.
- Dropped `egui_plot` dependency (forced egui 0.33/0.34 split). Chart now uses `egui::Painter`.
- New repo helper: `daily_revenue(start,end)` aggregates payments+sales by day in one query each.
- New `dates::Period` enum + `range()` + `days_inclusive()` + `months_ago()`.

### M5 final + M6 (partial) verification
- `cargo build --offline` → Finished, **0 warnings**.
- `cargo test --offline` → **11 passed; 0 failed**.
- Dashboard now owns an inline payment dialog (no cross-view dispatch needed).
- SettingsState loads once, edits gym_name/default_monthly_fee/currency, persists via repo.set_setting.
- CSV import uses `csv` crate already in Cargo.toml + a single rusqlite transaction over Repository::conn for bulk insert.

### M6 final verification
- `cargo build --offline` → Finished, **0 warnings**.
- `cargo test --offline` → **11 passed; 0 failed**.
- New `core::backup` module: backups_dir/backup_now/list_backups/restore + KEEP_LAST=7 prune by mtime.
- New `Repository::checkpoint()` (`PRAGMA wal_checkpoint(FULL)`) so the file copy captures committed data.
- Restore is two-step (queued file with `.pending-restore`, applied at startup before opening the DB) to avoid stomping the live connection.
- eframe's `on_exit` is the no-arg variant in our build (no glow feature on this dep config) — fixed the signature.

### M7 verification
- `cargo build --release --offline` → Finished in 6m11s; binary at `target/release/roche_crm.exe` is **13 MB**.
- Portable smoke test: copied only the .exe to a clean folder, ran twice; `roche.db` materialised next to it on first launch; member-insert via the lib persisted to the second run.
- README.md added at the repo root.
- Outstanding for v1.1: icon + winresource version metadata (offline cache miss).

### 5k-row perf check (release build, fresh DB, single run)
Seeded 5,000 members + ~1,667 payments via one rusqlite transaction, then timed the
hot paths the UI exercises every frame:
- seed (5k members + 1.6k payments): **22.7 ms**
- `list_members(active_only=true)` over 5,000 rows: **3.8 ms**
- `search_members("00042")` (substring, returns 111): **0.8 ms**
- `paid_member_ids(this_month)` (1,667 entries): **0.4 ms**
- `due_members(this_month)` (3,333 rows): **3.2 ms**
- `membership_income(this_month)`: **0.2 ms**
- `daily_revenue(this_month)` aggregate: **0.3 ms**

All under 5 ms, so the dirty-flag UI cache only reloads when needed and a redraw
is cheap. Indexes on `members(name)`, `members(phone)`, `payments(period_month)`,
`payments(member_id)`, `sales(date)` are doing their job.

- Keep core UI-free (testable without egui).
- Defer to v1.1: per-member fees, partial-balance ledger, attendance, receipts, per-member purchase history, richer charts.
- Risk: write-blocked folder (Program Files) breaks portable DB — out of scope.
- (Update with progress, decisions, blockers each iteration.)