# Advice for the next dozen Rust GUIs (summary)

> Source: https://raphlinus.github.io/rust/gui/2022/07/15/next-dozen-guis.html
> Author: Raph Levien · Published: 2022-07-15
> Retrieved: 2026-06-25
> Type: Personal blog post — summarized, not reproduced verbatim

## Premise

People constantly ask "what is the best Rust UI toolkit?" and there is still no
clear answer. The usual top contenders are **egui, Iced, and Druid**, with
**Slint** promising and web-based approaches like **Tauri** gaining momentum —
plus the perennial temptation to build yet another one. The post is a sequel to
Levien's "Rust 2020: GUI and community" and draws lessons from building Druid.

Motivation remains strong: Electron keeps growing, but there's broad desire for
a less resource-hungry alternative — yet no consensus on what it should look
like, and there's fragmentation even at the infrastructure layer.

## Main sections / arguments

- **A large tradeoff space** — GUI design involves many competing choices; no
  single point dominates.
- **A small rant about "native"** — pushes back on treating "native" widgets as
  an unqualified good.
- **On winit** — discussion of the cross-platform windowing layer most Rust GUIs
  build on, and its limitations.
- **Tradeoff: use of system compositor** — whether to lean on the OS compositor.
- **Tradeoff: platform text rendering** — using OS text stacks vs. rolling your
  own (relevant to Levien's font/text work).
- **On architecture** — reactive/data-flow models and the difficulty of an
  ergonomic, performant architecture in Rust.
- **The crochet experiment** — an experimental reactive approach explored in
  Druid.
- **Accessibility** — treated as a first-class, must-solve concern (led toward
  what became AccessKit).
- **What of Druid?** — reflections on Druid's limits and the direction beyond it
  (foreshadowing Xilem).

## Takeaway

A "clear-eyed survey" arguing the ecosystem needs to converge on shared
infrastructure (windowing, text, accessibility, architecture) rather than each
toolkit reinventing everything, with concrete advice aimed at authors of future
Rust GUI libraries.
