# A 2025 Survey of Rust GUI Libraries (summary)

> Source: https://www.boringcactus.com/2025/04/13/2025-survey-of-rust-gui-libraries.html
> Author: boringcactus · Published: 2025-04-13
> Retrieved: 2026-06-25
> Type: Personal blog post — summarized, not reproduced verbatim

## Method

A follow-up to the author's 2020 and 2021 surveys. They work through the
**Are We GUI Yet?** listings and attempt one tiny, consistent task in each
library: a text label plus an input field that updates the label. The task is
deliberately trivial so many libraries can be evaluated, but it disadvantages
frameworks that optimize for large-app scaling over quick initial setup. The
author cautions readers not to assume the conclusions transfer to substantially
more complex projects.

## Libraries evaluated (~43)

Azul, cacao, core-foundation, Crux, Cushy, CXX-Qt, Dioxus, Dominator, egui,
Floem, fltk, flutter_rust_bridge, Freya, fui, GemGui, GPUI, GTK 3, GTK 4, Iced,
imgui, KAS, kittest, Leptos, lvgl, Makepad, masonry, Maycoon, Pax, qmetaobject,
relm, Relm4, Ribir, Rinf, rui, Slint, Tauri, tinyfiledialogs, Tk, Vizia,
WebRender, windows, WinSafe, Xilem.

## Conclusions (paraphrased)

- 43 options is a *lot* — healthy for the ecosystem, but it makes triage
  (separating not-ready-yet from usable) increasingly important.
- Suggested "winners" by preference:
  - **Dioxus** — if you'd rather deal with CSS layout quirks; "Diet Electron,"
    better than regular Electron (though the author finds it not-better-enough).
  - **Slint** — if you like DSL-driven UIs with serious developer tooling.
  - **egui** — if you want to avoid DSLs/macros and write only plain Rust.
  - **Freya** and **Xilem** — basically usable now if you'll tolerate the
    bleeding edge and some jank; good to invest in early.
  - **Floem** and **iced** — have open accessibility issues worth watching
    (the iced one has been open ~4.5 years).
- Overall verdict: no single "obviously correct" slam-dunk choice, but many
  reasonable options — a clear improvement over 2021.
