# TenneCRM — UX/copy overhaul (37dhh implementation)

Implement the accepted design plan + copy pass. DHH-flavored: one cohesive app,
reuse existing repo methods, DB does atomicity, delete ceremony, code reads like
prose. Verify every group with a clean MSVC build.

## Environment / build
- The GNU toolchain is BROKEN here (no `dlltool.exe`). Do NOT use `cargo build`.
- Build with MSVC: `cargo +stable-x86_64-pc-windows-msvc build`
- Debug binary lands at `target/debug/tenne_crm.exe`.

## Goals
- One money format everywhere: `Rs 1,500` (thousands sep, 0 decimals) via one helper.
- Joining fee mandatory: single "New member" dialog; delete the reg-due apparatus.
- Transactions rows drill to their source (GPay-style); no fake hover affordance.
- Cut ceremony: drop the "Gray" theme; drop payments/sales/expenses CSV import.
- Unify row actions (⋯ menu + one primary verb) and empty states across tables.
- Full copy pass: front-desk language, not database language.

## Checklist

### A. Money format (fixes currency bug) — DONE
- [x] Add `ui::money(currency, amount) -> String` in `src/ui/mod.rs`: `Rs 1,500` (thousands sep, 0 decimals).
- [x] Route money strings through it: dashboard (deleted local `money`), members Details, merchandise (deleted hardcoded `Rs`; tables + sale dialog show currency), expenses tables + recurring, transactions amount.
- [x] Remaining `{:.0}` in members.rs are fee-INPUT defaults / reg-checkbox strings — removed/reworked by B & C, not display money.

### B. Merged "New member" dialog + mandatory joining fee — DONE
- [x] Repo `create_member(&mut self, m, joining_fee, first_month, month, date)`: member + registration + optional membership in ONE `conn.transaction()`.
- [x] One dialog: Full name / Phone / Joined on / Notes + (new only) Joining fee (required) + This month (editable) + "Collecting Rs X today". Save calls `create_member`.
- [x] Edit path keeps the plain form (fee rows hidden via `!form.editing`).
- [x] Removed `NewMemberPrompt` struct, `Dialog::NewMember` variant + render, and the `new_member_for` handoff. Build clean, 23.00s.

### C. Remove reg-due apparatus — DONE
- [x] Deleted `MemberFilter::RegPaid`/`RegDue` (+ ALL, labels, matches arm, `reg_missing` param).
- [x] Removed `reg_missing` state field, the `members_missing_registration` reload call, the `badge` fn, the "Reg due" cell, and the Flags column (6→5 cols).
- [x] Removed `reg_paid`/`reg_fee` fields + checkbox + `toggle_reg` handler from `PaymentsEditor`; `new()` no longer takes `reg_fee`.
- [x] Details now shows READ-ONLY "Joining fee": `Rs X · <date>` or `—`.
- [x] Repo methods KEPT (not dead): they are `pub` + exercised by tests.rs; registration data model unchanged (create_member still writes it). Build clean, no warnings.
- [x] Bonus copy here: status `Inactive`→`Past member`, badge ≥2 → `N months behind`, `Record payment`→`Payments`, menu → `Mark as left/active`, filters → `Due`/`Paid this month`, Details status → `Paid/Due this month`, `Reg. fee`→`Joining fee`, `Name *`→`Full name`, `Join date`→`Joined on`.

### D. Transactions drill-through — DONE
- [x] Added `pub enum TxnNav { Member/Sale/Expense }`; `transactions.show` returns `Option<TxnNav>`.
- [x] `app.rs` unifies both nav types via `enum Nav { Dash, Txn }`; Txn arm routes to the right tab + focus method.
- [x] Payment → `members.focus_member(id)` (member_id via `repo.get_payment`) opens the payment book; Sale → `merchandise.focus_sale(id)` (EditSale falls back to `repo.list_sales` so out-of-filter sales open); Expense → `expenses.focus_expense(id)`.
- [x] Rows use `ui.interact(..Sense::click())` + PointingHand — hover is now real. Subtitle copy updated. Build clean, no warnings.

### E. Cut ceremony — DONE
- [x] Removed `Mode::Gray`, `gray()` palette, orphaned `BRONZE` const, and the Gray selectable in Settings. `from_str` else-branch already maps stored "gray" → Dark, so old DBs migrate silently.
- [x] Removed Import payments/sales/expenses buttons + `import_payments_csv`/`import_sales_csv`/`import_expenses_csv` + the now-unused `read_csv` helper (~220 lines). Kept Members import + all four exports. Import section retitled "Import members" with plain copy. Trimmed unused `Expense`/`Payment` imports. Build clean, no warnings.

### F. Unify tables — DONE
- [x] Products, Sales, and Expenses rows now use the `⋯` menu (Edit/Delete). Recurring keeps its primary verb (`Record this month`) + `⋯` menu. One row-action pattern app-wide.
- [x] Expenses empty state now offers `+ New expense` (via new `Action::NewExpense`).
- [x] Inline reasons on disabled Save: product (`Name required; price and stock must be numbers`), sale (`Add at least one line with a quantity`), expense (`Name, date, and a numeric amount required`), recurring (`Name and a numeric amount required`).
- [x] Dashboard title routed through `ui.heading()` (was hand-rolled 24px) — all page titles one size.
- [x] Bonus copy here: tab `Recurring`→`Monthly bills`, `+ Add expense`→`+ New expense`, `+ Add recurring`→`+ New bill`, `Log expense`→`Record this month`, `Custom (write your own)`→`One-off (type it in)`, product `Active`→`Available`, `qty`→`Qty`. Build clean, no warnings.

### G. Copy pass (front-desk language) — DONE
- [x] Leaving vocabulary (C): `Inactive`→`Past member`, menu → `Mark as left/active`, checkbox → `Include past members`.
- [x] One word for owing `Due` (C): filter `Owes payment`→`Due`, Details → `Due this month`, badge ≥2 → `N months behind`. Hero stays "Members due" (reads well as a count).
- [x] `Covered`→`Paid ahead` (toggle button, hint text, both hover tooltips).
- [x] One `Save`: killed `Save changes` + `Save settings`; `Backup now`→`Back up now`; creation `+ New …`; `Record payment`→`Payments`.
- [x] `Merchandise`→`Shop`: nav label (app.rs), heading, dashboard KPI.
- [x] Dashboard KPIs: `Membership`, `Total members`, `See the breakdown`.
- [x] Members: dropped `Search:` label; hint `Search name or phone`. Fields `Full name`/`Joined on`.
- [x] Shop `Qty`/`Available` + product form `Name` + status col `Available/Hidden`. Expenses copy in F.
- [x] Settings: `Monthly membership fee`, `Joining fee (one-time)`, plain import copy + `running low`, restore msg → `Restore ready … Close and reopen the app to finish.`, `N backups`. Build clean, no warnings.

### H. Finish — DONE
- [x] Full recompile (`touch src/main.rs src/lib.rs`) `cargo +stable-x86_64-pc-windows-msvc build` — `Finished dev profile ... in 12.30s`, no errors.
- [x] Zero warnings across the crate (grep of build output is empty).
- [x] `cargo +stable-x86_64-pc-windows-msvc test` — 21 passed, 0 failed.
- [x] Debug binary present: `target/debug/tenne_crm.exe` (45 MB).

## Verification
- Per-group: `cargo +stable-x86_64-pc-windows-msvc build 2>&1 | tail -5` after each section.
- Section A: build clean, `Finished dev profile ... in 10.80s`.
- Section B: build clean, `Finished dev profile ... in 23.00s`.
- Section C: build clean, no warnings, `Finished dev profile ... in 23.38s`.
- Section D: build clean, no warnings, `Finished dev profile ... in 29.12s`.
- Section E: build clean, no warnings, `Finished dev profile ... in 23.61s`.
- Section F: build clean, no warnings, `Finished dev profile ... in 19.10s`.
- Section G: build clean, no warnings, `Finished dev profile ... in 11.61s`. Grep sweep for old copy strings returns nothing.

## Final Verification
- Command: `cargo +stable-x86_64-pc-windows-msvc build`
- Working dir: `C:\Users\katto\Documents\software\tmp\tenne_rust`
- Env: uses the MSVC toolchain (VS 18); the default GNU toolchain FAILS here (no dlltool.exe).
- Artifact preserved: `target/debug/tenne_crm.exe` (present, 45 MB).
- Result: `Finished \`dev\` profile [unoptimized + debuginfo] target(s)` — clean, no warnings.
- Also verified: `cargo +stable-x86_64-pc-windows-msvc test` → `test result: ok. 21 passed; 0 failed`.

## Notes
- Design vocabulary locked: "Due" = owes money; "Paid ahead" = settled without money this month; "Past member" = left.
- Keep primary/valued views intact: per-member year payment book, GPay-style Transactions tab, dashboard heroes. Do not cut these.
- 37dhh calls: money() is conceptual compression (one place money reads); member creation is one repo method in a DB transaction, not a service; drill-through resolves member at click time (no denormalization).
- Merchandise heading already renamed to "Shop" during A.
