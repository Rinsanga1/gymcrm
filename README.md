# RocheCRM

A simple gym CRM for Windows. Single portable `.exe`, no installer, no runtime
prerequisites. All data lives in a SQLite database next to the executable.

## Install / run

1. Download `roche_crm.exe`.
2. Put it in a writable folder (e.g. `Documents\RocheCRM\`). Do **not** drop it
   in `Program Files`; that folder is read-only for normal users and the DB
   won't be writable.
3. Double-click `roche_crm.exe`.

That's it. On first launch the app creates `roche.db` next to the executable
and seeds default settings (currency `Rs`, default monthly fee `1500`).

## Where is my data?

In the same folder as the `.exe`:

```
roche_crm.exe
roche.db                 # main database
roche.db-wal             # SQLite write-ahead log
roche.db-shm             # SQLite shared-memory file
backups/
    roche_20260625-184201.db
    roche_20260626-101533.db
    ...
```

The app keeps the last 7 backups in `backups/`. A new backup is written each
time you close the app cleanly, and you can take one on demand from
**Settings → Backup now**.

## Features

- **Members** — virtualized table (smooth at 5,000+ rows), search by name or
  phone, add/edit/delete, active/inactive toggle, prefilled payment dialog
  when adding a member.
- **Merchandise** — products with stock tracking; record sales with multiple
  line items; low-stock warnings; editing a sale restores old stock and
  re-applies new lines.
- **Expenses** — quick add/edit/delete with date, amount, and note.
- **Dashboard** — KPI cards, revenue trend chart, "Due this month" list with
  one-click payment entry. Time chips: All Time / Today / This Week /
  This Month / This Quarter / This Year / Custom.
- **Settings** — gym name, default monthly fee, currency.
- **CSV import** — bulk-add members from a CSV with `Name,Phone` columns.
- **CSV export** — members, payments, sales, expenses.
- **Backups** — automatic on close, manual on demand, restore from any backup.

## Moving to a new machine

Copy the entire folder (`roche_crm.exe`, `roche.db*`, and `backups/`). That's
the whole installation.

## Restoring a backup

1. Open **Settings → Restore from file…** and pick a `.db` from `backups/`
   (or anywhere else).
2. Close the app.
3. Open it again — the restore is applied at startup, and a safety copy of
   the previous DB is written to `roche.db.pre-restore` in the same folder.

## Build from source

Requires the Rust **GNU** toolchain (`x86_64-pc-windows-gnu`) plus a working
MinGW-w64 with `dlltool` available. No Visual Studio needed.

```sh
cargo build --release
```

The single-file binary lands at `target/release/roche_crm.exe`.
