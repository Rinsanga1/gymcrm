# Rust for Windows, and the windows crate

> Source: https://learn.microsoft.com/en-us/windows/dev-environment/rust/rust-for-windows
> Retrieved: 2026-06-25
> Type: Microsoft Learn documentation (summary/notes)

## What it is

Rust for Windows lets you call **any** Windows API (past, present, and future)
directly and seamlessly from Rust via the **`windows` crate**. It's an open
source *language projection* developed on GitHub, similar in spirit to C++/WinRT.

## Key points

- The `windows` crate covers the full Windows API surface: classic Win32
  functions (e.g. `CreateEventW`, `WaitForSingleObject`), graphics engines like
  Direct3D, traditional windowing (`CreateWindowExW`, `DispatchMessageW`), and
  newer UI frameworks such as Composition.
- It is powered by the **win32metadata** project, which provides strongly-typed
  metadata (signatures, parameters, types) for Win32 APIs. This same metadata
  also drives projections for C# and C++.
- Dependencies are managed the normal Rust way: **Cargo** + **crates.io**. Add
  the `windows` crate and you can immediately start calling Windows APIs.
- API reference for the crate is on **docs.rs**, and there is separate
  "Rust documentation for the Windows API" describing how Windows APIs/types are
  projected into idiomatic Rust.

## Resources mentioned

- **Rust for Windows** GitHub repo — questions, issues, and simple examples.
- Sample app: Robert Mikhayelyan's **Minesweeper**.
- Release log of the Rust for Windows repo for latest updates.
- Follow-on tutorial: **RSS reader** walkthrough for writing a simple app.

## Contributors (per page metadata)

stevewhims, Karl-Bridge-Microsoft, makubacki, zanedp, legowerewolf,
mohitp930, gurry, dacarab.
