Implement the TenneCRM audit plan in `AUDIT_PLAN.md` (repo root). Read that file first each iteration — it has the full code map, build protocol, and task specs. Do ONE task per iteration in the suggested order, keep the build green (cargo check clean + cargo test passing) at the end of every iteration.

Order: T1 → T2 → T4 → T5 → T3 → T6 → T9 → T8 → T10.

Build protocol (bash, Windows):
- export PATH="/c/Users/katto/.cargo/bin:$PATH"
- cargo check after edits; fix ALL warnings.
- For tests/build: copy libshlwapi.a into the toolchain self-contained dir first, remove after. If tenne_crm.exe is running (tasklist | grep -i tenne), skip the link/build step — compile+test is enough.

Checklist:
- [x] T1 — One payment surface: year ledger everywhere; Dashboard due row navigates to Members + opens that member's ledger; remove dashboard pay_dialog; delete ui::payment if unused.
- [x] T2 — Registration fee derived from a category='registration' payment, not the boolean; remove the MemberForm checkbox; add repo helpers (has_registration_payment, members_missing_registration); reimplement unpaid_registration_count off payments; update badge/Details/tests.
- [x] T4 — Dashboard "Reg fee due" becomes an actionable list with "Collect joining fee → name" recording a registration payment.
- [x] T5 — After adding a NEW member, focused prompt (not full ledger): this month's fee + optional joining fee, one Save. Editing must not trigger it.
- [x] T3 — Year ledger auto-saves current year's edits before switching year (no data loss).
- [x] T6 — Transactions read-only: remove all edit/delete dialogs, actions, and row buttons; keep the read-only grouped list.
- [x] T9 — Members row: Record payment is the one prominent button; move Details into the ⋯ menu with Edit/Deactivate.
- [x] T8 — Merchandise single screen: remove Products/Sales tabs; +Add product and +Record sale both available; products table then sales section; preserve all dialogs.
- [x] T10 — Settings split (payments + expenses importers; sales import deliberately cut — stock side-effects, .db backup covers full migration) into Preferences vs Data sections; add CSV importers for expenses, payments, sales/products to match existing exporters (round-trip must work); report imported/skipped counts.

When ALL checklist items are done, the build is green with no warnings, and tests pass, output the completion marker instead of calling ralph_done.