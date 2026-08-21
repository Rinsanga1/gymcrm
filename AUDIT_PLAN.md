# TenneCRM — Audit Implementation Plan

Gym-manager app (Rust + egui). Implementing the approved findings from the
Fried / Singer / DHH audit. Skipped: #7 (nav regrouping).

## Build / test protocol (every iteration)
- `export PATH="/c/Users/katto/.cargo/bin:$PATH"`
- `cargo check` after edits; fix all warnings.
- Tests + debug build (needs libshlwapi workaround):
  ```
  SC=/c/Users/katto/.rustup/toolchains/stable-x86_64-pc-windows-gnu/lib/rustlib/x86_64-pc-windows-gnu/lib/self-contained
  cp libshlwapi.a "$SC/"; cargo test 2>&1 | grep -E 'test result|FAILED'; cargo build 2>&1 | tail -3; rm -f "$SC/libshlwapi.a"
  ```
- Building fails with "Access is denied" if `tenne_crm.exe` is running — check `tasklist | grep -i tenne`; if running, skip the build step (compile + test are enough to verify).
- Keep the build green at the end of every iteration. Never leave it broken.

## Code map (grounded)
- `src/app.rs` — `View` enum, sidebar nav, invalidate-on-nav. Owns all `*State`.
- `src/ui/dashboard.rs` — `DashboardState` (has `pay_dialog: Option<payment::PaymentForm>`), period chips, heroes (Net earnings / Members due), KPIs, low-stock, revenue chart, Due list with `Record payment → {name}` buttons, `reg_due` count (dead-end).
- `src/ui/members.rs` — `MembersState`; `PaymentsEditor` = year Jan–Dec ledger (the payment surface); `MemberForm` with `registration_fee_paid` checkbox → `repo.ensure_registration_payment`; `Details` dialog; table columns Name/Phone/Status/Flags/Joined/Actions; row actions Record payment / Details / ⋯(Edit, Deactivate).
- `src/ui/payment.rs` — shared `PaymentForm` modal. After #1 only dashboard uses it; likely deletable.
- `src/ui/merchandise.rs` — `Tab` {Products, Sales}; `SaleForm` (multi-line); products + sales tables.
- `src/ui/expenses.rs` — `ExpensesState` add/edit/delete.
- `src/ui/transactions.rs` — unified money ledger, currently also edits/deletes (EditPayment, EditExpense, ConfirmDelete).
- `src/ui/settings.rs` — Preferences (gym name, fees, currency, theme) + CSV import (members only) + exports (members/payments/sales/expenses) + backups/restore.
- `src/core/repo.rs` — full data API. Registration: `ensure_registration_payment`, `unpaid_registration_count` (reads `registration_fee_paid` col), `registration_fee()`.
- `src/core/db.rs` — schema + `add_missing_columns` migrations.
- `src/core/models.rs` — `Member.registration_fee_paid: bool`; `Payment.category` (`membership|trial|registration`).

---

## Tasks

### T1 — One payment surface: year ledger everywhere; Dashboard only shows dues
- The year ledger (`PaymentsEditor` in members.rs) is the ONLY way to record a payment.
- Dashboard Due list: keep it informational. `Record payment → {name}` should navigate to Members and open that member's year ledger (not the old inline form).
- Plumbing: `DashboardState::show` returns an action (e.g. `Option<i64>` = member id to open payments for). In `app.rs`, on that: set `view = Members` and tell `MembersState` to open the ledger for that id (add `MembersState::open_payments(repo, id)` or a pending field consumed on next `show`).
- Remove `pay_dialog` + `draw_payment_dialog` from dashboard.
- If `ui::payment` module is now unused, delete it (module + `pub mod payment;` in mod.rs). Verify dashboard was its only remaining user.
- Verify: dashboard due row → click → lands on Members with ledger open for the right member.

### T2 — Registration fee is a payment, not a boolean
- Stop treating `registration_fee_paid` as source of truth. Derive "reg fee paid" from existence of a `category='registration'` payment for the member.
- Add repo helpers: `has_registration_payment(member_id) -> bool` and `members_missing_registration(active_only) -> Vec<Member>`. Reimplement `unpaid_registration_count` off payments (COUNT members with no registration payment).
- Members table "Reg due" badge + Details "Reg. fee" line: compute from payments, not the bool.
- Remove the `registration_fee_paid` checkbox from `MemberForm`. Collecting the joining fee happens via T5 (new-member prompt) and T4 (dashboard reg-due list). Keep `ensure_registration_payment` as the write path.
- Keep the DB column (harmless) OR drop reads of it; do NOT require a destructive migration. Update `models.rs`/`repo` reads so the bool is no longer load-bearing. If `Member.registration_fee_paid` becomes fully unused, remove the field and its column reads (and adjust `member_from_row`, insert/update SQL) — but only if clean.
- Update tests referencing `registration_fee_paid`.

### T3 — Year ledger: no data loss on year switch
- In `PaymentsEditor` flow (members.rs), when the year dropdown changes, auto-save the current year's edits (run the same reconcile-against-repo used by "Save changes") BEFORE rebuilding entries for the new year.
- After auto-save, reload payments from repo so the new year reflects persisted state. Surface errors via `ed.error` and abort the switch if save failed.

### T4 — Dashboard "Reg fee due" becomes an actionable list
- Replace the dead `reg_due` count with (or augment it by) an expandable list of members missing the registration payment (`members_missing_registration`).
- Each row: `Collect joining fee → {name}` button that records a registration payment via `ensure_registration_payment` (amount = `repo.registration_fee()`, month = current, date = today). Refresh after.

### T5 — Focused first-payment prompt after adding a member
- After a new member is inserted (members.rs add-member success path), open a small focused dialog (NOT the full year ledger): two pre-filled amounts — this month's membership fee (`default_monthly_fee`, current month) and the joining fee (`registration_fee`) with a checkbox/toggle to include it — and one Save.
- Save records the membership payment (category `membership`) and, if included, the registration payment (`ensure_registration_payment`). Then reload.
- Editing an existing member must NOT trigger this prompt.

### T6 — Transactions is read-only (audit ledger)
- Remove edit + delete from transactions.rs: drop `Dialog::EditPayment`, `Dialog::EditExpense`, `Dialog::ConfirmDelete`, the `Action` variants, `handle_action`, `draw_dialog`, and the row Edit/Delete buttons.
- Keep the grouped, hover-highlighted read-only list. Editing lives in Members (payments), Expenses, Merchandise (sales).
- Remove now-unused repo calls / imports from transactions.rs only (do not delete repo methods still used elsewhere).

### T8 — Merchandise: one screen, no Products/Sales tabs
- Remove the `Tab` enum and tab selector. Single screen: `+ Add product` and `+ Record sale` both available at top; Products table, then a Sales history section below (or a clear divider).
- Preserve all existing dialogs/actions (add/edit/delete product, record/edit/delete sale) and low-stock display.

### T9 — Members row: one obvious action, rest demoted
- Make `Record payment` the single prominent per-row button.
- Move `Details` into the `⋯` menu alongside `Edit` and `Deactivate/Reactivate` (row-click-for-details is not easily supported by egui_extras TableBuilder — menu is the pragmatic demotion).
- Result: each active row shows `Record payment` + `⋯`.

### T10 — Settings split + full CSV import/export parity
- Split Settings into two clear sections (sub-tabs or headed groups): **Preferences** (gym name, default monthly fee, registration fee, currency, theme) and **Data** (CSV import/export, backups, restore).
- Import parity: today only members import exists, but export covers members/payments/sales/expenses. Add importers so anything exported can be re-imported:
  - Import expenses (name, amount, date, note).
  - Import payments (member reference, period_month, amount, date, note, category).
  - Import products and/or sales as feasible (match the export columns).
- Match importer column headers to the existing exporters' output. Look at the export functions in settings.rs / repo (or wherever CSV is written) to mirror headers exactly, so a round-trip (export → import) works.
- Add repo insert paths as needed; reuse existing `insert_*`. For payments import, resolve member by id or name/phone as the export encodes it.
- Report a summary (rows imported / skipped) like the members import does.

---

## Suggested order
T1 → T2 → T4 → T5 (payment/registration cluster) → T3 → T6 → T9 → T8 → T10.
Do one task per iteration (or a tightly related pair). Compile + keep green each time.

## Definition of done
- All tasks above implemented, `cargo check` clean (no warnings), `cargo test` green.
- No dual edit paths for the same record; one payment surface; joining fee is a payment; Transactions read-only; Merchandise single screen; Settings split with full import parity.
