# RocheCRM — MVP PRD

> Owner: dev@mustbesocial.com
> Status: MVP scope · v1
> Last updated: 2026-06-25
> Companion to the full vision doc: [rocheCRM-prd.md](rocheCRM-prd.md)

---

## 1. Goal

Build a **very easy-to-use, Windows-native gym CRM** that lets the owner track
members, record payments manually, sell merchandise, and see earnings — with no
accounting knowledge required. v1 should feel like a **digital notebook with a
clean dashboard**, not a business ERP.

Ships as a **single downloadable `.exe`** (Rust, native Windows) — download,
double-click, start using. All data is local, offline, in one database file.

---

## 2. Users

- **Primary:** gym owner.
- **Secondary:** front-desk staff who record payments/sales.
- **Skill level:** very low technical ability → the UI must be simple, obvious,
  and guided.
- **Device:** a **Windows laptop or PC at the front desk** (mobile is explicitly
  out — this is a native `.exe`). Optimized for fast data entry.

---

## 3. Problem

Membership and merchandise money currently gets lost across notebooks, memory,
and WhatsApp. RocheCRM gives one place to record **who paid, how much, what they
bought, and what's still owed** — plus a clear view of earnings.

---

## 4. Scope

### In scope (MVP)
- Member creation/editing + quick search.
- Manual payment entry + partial payments + due-balance tracking.
- Merchandise: editable product list (e.g. gym shirts, protein, glucose).
- Merchandise sales entry (to a member **or** walk-in) with stock decrement.
- Basic expense entry.
- Earnings dashboard: today / week / month, **split by membership vs.
  merchandise**, plus pending dues and net earnings.
- CSV import (migrate the existing Google Sheet) + CSV export/backup.

### Out of scope (MVP)
- Online payment gateways / automatic recurring billing.
- Mobile app, multi-branch, multi-user roles.
- Barcode scanning, attendance/check-ins, trainer scheduling, notifications.
- Complex accounting.

> Note: **Attendance/check-ins is the #1 candidate for v1.1** — deferred, not
> forgotten. (See Section 11.)

---

## 5. Key product decisions (reconciled)

1. **Native Windows `.exe`, built in Rust.** Single self-contained file, no
   installer, no runtime deps, fully offline. *(Confirmed — mobile dropped.)*
2. **Members and Merchandise are independent.** A sale can attach to a member or
   be a walk-in. If the gym sells nothing, merch modules sit empty and the
   dashboard shows zero merch revenue — no special-casing.
3. **Merchandise is freely editable** (add/edit/remove products anytime); it may
   or may not exist for a given gym.
4. **Scale target: 5,000+ members, designed with headroom to ~50,000.** SQLite
   handles this easily; the engineering work is on the UI side (Section 9).

---

## 6. Core workflows
1. Add a new member.
2. Record a manual payment for a member (full or partial).
3. Sell merchandise (shirt / protein / glucose…) to a member or walk-in.
4. See total collected, pending dues, and merch revenue at a glance.
5. Review net earnings = income − expenses.

**Success targets:** record a payment in **< 30s**, record a sale in **< 30s**,
see today's earnings on **one screen**, spot overdue members **without manual
searching**, export everything **anytime**.

---

## 7. Feature requirements

### 7.1 Members
- Fields: **name (required)**, phone, join date (auto-set to today), notes —
  **everything except name is optional**. *(No stored fee — amounts live on
  payments.)*
- Quick search by name or phone.
- **Active/Inactive flag** (stored, default Active). Marking a member **Inactive**
  (quit) removes them from the dues list, the dashboard "due" count, and the
  default roster. A separate **Inactive view** lets the owner see who quit and
  reactivate them if they return.
- For **Active** members, status is **derived** (not stored): **Paid** (paid for
  current month) or **Due** (not yet paid for current month). Inactive members
  are never shown as Due.
- **Registration flow is guided:** saving a new member immediately opens a
  prefilled payment dialog (default Rs 1,500). The Rs 500 registration fee is
  **not** a separate record — the owner just edits the first payment up to e.g.
  Rs 2,000 and saves. That single payment marks the member Paid for the month.
  The payment dialog is **skippable** — register someone who hasn't paid yet and
  they simply start as Due.

### 7.2 Membership & Payments (monthly calendar model)
- Membership is **monthly by calendar month**. On the 1st of each new month,
  **every member becomes Due**.
- The owner clears it by **manually recording a payment** for the current month;
  that member then shows **Paid/Active** for the month.
- **Status is derived, never toggled:** Paid-this-month → Active, otherwise Due.
- Record a **payment**: month, amount, date, optional note; linked to a member.
  **No payment-method field** (it's mostly cash — just the amount matters).
  The **amount field is prefilled with the default monthly fee (Rs 1,500)** and
  is fully editable for outliers (half payments, registration-inclusive first
  payment, etc.).
- **Multiple payments per month are allowed** (e.g. pay half now, half later —
  both count toward collected revenue).
- **Pure binary status:** *any* payment for the month clears **Due** — even a
  partial Rs 750 counts as fully Paid for that month. No amber/partial state. The
  owner eyeballs the amount column to spot short payments. Remaining-balance
  tracking is **deferred to v1.1**.
- "Pending Dues" on the dashboard = **count** of members not yet paid for the
  current month (optionally × default fee for a rough rupee estimate).

### 7.3 Merchandise (Products)
- **Flat editable product list** (no categories): name, selling price, stock
  count. *(Decided: flat list, the usual fields.)*
- Add / edit / remove products at any time.

### 7.4 Merchandise Sales
- Record a sale with **line items** (product + quantity + price-at-sale), so
  stock math and revenue are correct.
- **Sales are anonymous** — no member link in MVP (fastest counter flow: pick
  product → qty → done). Per-member purchase history is a v1.1 option.
- **Decrement stock** on sale.
- Merch contributes to **revenue** only (no cost/profit tracking — buying stock
  is logged as an expense). Profit-per-item deferred to v1.1.

### 7.5 Expenses
- Simple **flat** entry: amount, date, note (free text). **No categories.**
  Feeds net-earnings as a single total.

### 7.6 Earnings Dashboard
- **Time-range chips:** All Time / Today / This Week / **This Month (default)** /
  This Quarter / This Year / Custom — filters every number on the page.
- KPI cards (period-filtered):
  - **Total income** (= membership + merchandise)
  - **Membership income**
  - **Merchandise income** (revenue, + units sold)
  - **Pending dues**
  - **Total expenses**
  - **Net earnings** (income − expenses)
  - **Total / active / due members**
- **Clickable "Due this month" list** — the primary actionable widget: tap a
  member to record their payment and chase dues.
- **One revenue-trend line chart** (the only graphic in MVP).
- **Low-stock indicator** for merch (simple flag, not a chart).
- Clean empty states when there's no data yet.
- *Deferred to v1.1:* membership-vs-merch split chart, top-selling-products
  chart, and richer analytics.

### 7.7 Import / Export
- **CSV import** for members (day-one Google Sheet migration). The sheet is
  simple (members + payment), so import maps **Name + Phone**; the owner picks
  which CSV column is which at import time. **Payment history is not
  back-imported** — tracking starts fresh from the launch month (everyone begins
  Due in month one).
- **CSV export** for members, payments, sales, expenses — usable for external
  accounting.
- **Automatic local backups:** on app close, silently copy `roche.db` into a
  `backups/` folder (timestamped), keeping the last ~7. Protects the single
  portable file against corruption / accidental deletion.
- One-click manual **backup/restore** in addition to the automatic ones.

---

## 8. Data model (MVP)

- **Member**: id, **name (required)**, phone?, join_date (default today),
  active (bool, default true), notes?.
  *(Only name is required. Paid/Due is derived from payments, not stored; no fee
  field. Inactive members are excluded from dues.)*
- **Settings**: default_monthly_fee (Rs 1,500), currency ("Rs"), gym_name.
- **Payment**: id, member_id, period_month (e.g. `2026-06`), amount, date, note.
  *(No method field.)* A member is **Paid** for a month if a payment exists for
  that `period_month`.
- **Product**: id, name, price, stock, active. *(No cost price — revenue only.)*
- **Sale**: id, date, total. *(Anonymous — no member link in MVP.)*
- **SaleItem**: id, sale_id, product_id, qty, unit_price.
- **Expense**: id, amount, date, note. *(No category — flat.)*

> Charge + (Sale/SaleItem line items) are the two additions that make "dues" and
> "stock/profit" actually computable. Everything else stays minimal.

---

## 9. Non-functional requirements

| Area | Requirement |
|------|-------------|
| **Distribution** | Single Windows `.exe`, no installer, no runtime deps, offline. |
| **Simplicity** | Minimal fields per form; guided, obvious UI for low-tech users. |
| **Forgiveness** | All records freely editable/deletable; delete asks for confirmation; auto-backups protect against mistakes. |
| **Scale** | Smooth with **5,000+ members**, designed for ~50,000 headroom. |
| **Performance** | Fast on low-end PCs; instant search; startup < 1s. |
| **UI rendering** | **Virtualized tables** (render only visible rows) — never draw all members at once. |
| **Search/filter** | Done at the **DB level with indexes**, not by scanning in memory. |
| **Data safety** | Transactional writes; **automatic timestamped backups (last ~7) on close** + manual backup/restore + CSV export; no silent loss. |
| **Storage** | **Portable:** one SQLite `roche.db` file **next to the `.exe`** (carry on USB). Fall back to `%APPDATA%` only if a write-blocked location forces it later. |
| **Privacy** | No accounts, no telemetry, no network by default. |

---

## 10. Tech approach (MVP)
- **Rust** → single static `.exe` (`x86_64-pc-windows-msvc`).
- **GUI: egui/eframe — DECIDED.** Pure Rust, single-exe, fastest to build
  forms/tables. Use a **virtualized table** for member/sales lists to hit the
  5k+ target. Plainer look than Timeline accepted in exchange for speed; a Slint
  re-skin is a possible v2.
- **DB: SQLite via `rusqlite`** (bundled — no external DLL). Indexed columns on
  member name/phone and sale/payment dates.
- Crates: `eframe`/`egui`, `egui_plot` (charts), `rusqlite`, `serde`, `chrono`,
  `csv`.
- **Core/UI split:** money logic (dues, balances, profit, net earnings) lives in
  a pure-Rust, testable core layer; egui calls into it.

---

## 11. Priority order

**Must have (MVP):** Members · Charges + Manual payments · Due tracking ·
Products + Merchandise sales (with stock) · Earnings dashboard (membership vs.
merch split + net) · CSV import/export.

**Should have:** Expense tracking · low-stock alerts · search/filters · notes per
member.

**v1.1 (next):** **Attendance/check-ins** · receipts · overdue reminders.

**Later:** user roles · multi-location · automated/recurring billing · cloud
backup.

---

## 12. Definition of done (first release)

The MVP is done when the owner can:
1. Add a member (and import the existing sheet via CSV).
2. Enter a manual/partial payment and see the remaining due.
3. Sell a product (shirt / protein / glucose) to a member or walk-in, with stock
   decrementing.
4. See income, dues, and **membership-vs-merchandise revenue** on one dashboard.
5. Export all records.

…all from a **single `.exe`** that runs on a clean Windows machine with no setup.

---

## 13. Resolved & open decisions

**Resolved:**
- **Membership = monthly calendar model.** Everyone goes Due on the 1st; a
  recorded payment for the current month clears them. Status is derived.
- **Fee:** global default **Rs 1,500** prefilled on the payment form, editable
  per payment. No per-member fee. Currency = **Rs**.
- **Registration fee (Rs 500):** no separate record — folded into the first
  payment by editing the amount. New-member save → prefilled payment dialog.
- **Pure binary paid/Due** — any payment (even half) = Paid for the month, no
  partial/amber state. Multiple payments per month allowed; remaining-balance
  tracking deferred to v1.1.
- **No payment-method tracking** — mostly cash; only the amount is recorded.
- **Expenses tracked in MVP** (flat: amount/date/note, **no categories**) →
  dashboard shows Net earnings = income − expenses as a single total. (Merch
  restock cost is logged here, since merch itself is revenue-only.)
- **Data safety = automatic local backups** (timestamped, last ~7, on close) +
  manual backup/restore + CSV export. Critical because the DB is a single
  portable file with no cloud copy.
- **Inactive flag** (manual) retires quit members from dues/roster; separate
  Inactive view to review/reactivate. No auto-inactivation in MVP.
- **Member fields minimal; only name required** (phone/join date/notes optional).
  Post-registration payment dialog is skippable.
- **Dashboard period chips:** All Time / Today / This Week / This Month
  (default) / This Quarter / This Year / Custom.
- **Dashboard depth (MVP):** KPI cards + clickable Due-this-month list + one
  revenue-trend chart + low-stock flag. Richer charts deferred to v1.1.
- **GUI toolkit = egui/eframe** (decided). Speed over polish; Slint re-skin is a
  possible v2.
- **Free edit/delete** on members, payments, sales, expenses, with a confirm
  dialog on delete. No soft-delete, no audit trail (auto-backups cover mishaps).
- Merchandise = **flat list** (name, price, stock). No categories, **revenue
  only** (no cost/profit; restock cost is logged under Expenses).
- **Merch sales are anonymous** (no member link in MVP).
- Behavior goal: **double-click the `.exe` and use it immediately**, Timeline-CRM
  style — no setup screen, no config.
- **Storage = portable (Option B):** `roche.db` lives next to the `.exe` so the
  whole thing carries on a USB stick. Accepted risk: if the `.exe` is run from a
  write-blocked folder (e.g. `Program Files`), writes fail — we'll switch to
  `%APPDATA%` then. Not solving that in MVP.

**Open (being grilled):**
- CSV import: **members only** (Name + Phone), fresh payment tracking from launch
  month. No payment-history back-import.
- Receipts in MVP or v1.1? (Currently v1.1.)
