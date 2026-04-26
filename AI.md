# AI.md

This file provides guidance to AI agents when working with code in this repository.

## Project

Pogoda is a Rust CLI that renders color-coded terminal weather reports. Three modes: standard forecast (`pogoda <city> [days]`), drone pilot profile (`--i-drone-you`), and historical (`--delorean D1 D2` where D1 and D2 are `DD.MM.YYYY` dates — both required, placed immediately after the flag).

## Commands

```bash
cargo run -- Wrocław 7       # city name may be Unicode; this shows the basic CLI shape
cargo test                   # the only thing CI runs (no clippy/fmt in CI)
cargo test <name>            # run a single test by name substring
```

## Architecture

Ratatui is a dependency but this is **not** a TUI application. Everything is one-shot printed to stdout via ANSI escape codes scaled to terminal width (`crossterm::terminal::size`, fallback 120). There is no event loop.

HTTP is synchronous — `reqwest::blocking` only, no async runtime anywhere in the codebase.

## Conventions and gotchas

- **Argument parsing is hand-rolled in `main.rs`** — there is no `clap`/`structopt`. Boolean flags are scanned with `raw_args.iter().any(|a| a == "--foo")`; value-taking flags use `raw_args.iter().position(...)` and read `pos + 1` (or further). After scanning, the strip-filter in `main.rs` removes flags from positional args via either a `matches!` allow-list (boolean flags) or a `skip` counter for flags that consume the following N args. **Adding a new flag requires:**
  1. The scan in `main` (`any` for boolean, `position` + `get(pos+1)` for value-taking).
  2. Strip the flag from positional args: add to the `matches!` list (boolean) **or** add an `if a == "--foo" { skip = N; continue; }` branch (value-taking). `--units` / `--lang` are skip-1; `--delorean` is skip-2.
  3. The `print_usage()` help text at the top of `main.rs` — and the corresponding `usage.flag_*` entries in **all** YAML catalogs in `locales/`.
  4. The flag listing in `render::historical::write_hist_footer` and the two equivalent footer blocks in `main.rs` (forecast and drone). All three are literal duplicates and must stay in sync.
- **Colored output must go through `render::emit_span` or `render::write_colored`** — both honor the `mono` flag (`--i-cant-afford-cga`). Direct `\x1b[...m` writes will break monochrome mode.
- **Palettes are OKLCH hue sweeps** (`colors.rs`): each `Theme` is a start/end hue at fixed L=0.62, C=0.14. Metric-specific colors (`temp_color`, `wind_color`, `pressure_color`) map a value into `[0,1]` over a hardcoded range, then call `palette(t, theme)`. Update the range constants at the top of `colors.rs` if scales need tuning.
- **Tests use real Open-Meteo JSON fixtures** in `tests/fixtures/` loaded via `include_str!` from inside `#[cfg(test)] mod tests` blocks in `weather.rs`. There are no integration tests — everything lives next to the code in source files.
- **`--delorean` parsing is position-based** in `main.rs`: it locates `--delorean` and reads `pos + 1` and `pos + 2` as `DD.MM.YYYY` dates. If either fails to parse (or is missing), the program exits with `errors.delorean_invalid_dates`; the date args are stripped from positional args by the `skip = 2` rule in the filter, not by re-parsing. Don't restore the old "strip anything that parses as a date" heuristic — it silently let bad dates leak into the city name.
- **Historical mode auto-selects rendering** by date span: ≤31 days → hourly, ≤365 → daily, otherwise monthly (aggregated client-side from daily via `weather::aggregate_monthly`). Hourly historical synthesizes `precip_prob` from the `precip` value (100 if >0, else 0) since the archive API does not provide it.

## Internationalization

Catalogs live in `locales/<lang>.yml`, loaded at compile time via `rust_i18n::i18n!("locales", fallback = "en")` at the top of `src/main.rs`. Locale + units are auto-detected from the OS via `src/locale.rs` (overridable with `--lang` / `--units`).

- **Adding a user-visible string** requires editing **all YAML catalogs** in `locales/` (25 languages: `en`, `de`, `es-es`, `es-419`, `fr-fr`, `fr-ca`, `it`, `pt-br`, `pt-pt`, `nl`, `pl`, `cs`, `sk`, `hu`, `ro`, `hr`, `sv`, `da`, `nb`, `fi`, `tr`, `el`, `ru`, `uk`, `ca`). Only `en` is the fallback; missing keys in other locales fall through to English at runtime, which is usually not what you want. Treat the key set as a strict superset that all locales must cover.
- **Adding a new language** requires four edits: a new `locales/<code>.yml`, a branch in `locale::lang_for_locale`, a branch in `locale::chrono_locale` (only locales chrono's `unstable-locales` actually ships are valid — verify), and the `--lang` validator match arm in `main.rs`. Update unit tests in `locale.rs` too.
- **Unit suffixes (`°C`, `°F`, `mph`, `km/h`, `mm`, `in`, `hPa`, `inHg`) are NOT translated** — they come from `Units::*_label()` in `types.rs`. They're symbols, not words. Do not move them into YAML.
- **Day-summary column is locale-sized at runtime.** `summary_label_w()` (in `render/table.rs`) and `drone_summary_label_w()` (in `render/drone.rs`) measure the longest localized label across `SUMMARY_KEYS` / `DRONE_SUMMARY_KEYS`; `print_table` / `print_drone_table` then derive `day_w = max(label_w + value_w + 1, header_w, 18)` with `value_w = 7` and a floor of 18 to preserve the original English layout. Don't hardcode `day_w = 18` in new code; if you add a new summary row, append its key to `SUMMARY_KEYS` / `DRONE_SUMMARY_KEYS` so the width math sees it.
- **Date formatting** uses a per-locale chrono format string in YAML at `location.date_full` (e.g. `"%-d %B %Y"` for most languages, `"%B %-d, %Y"` for `en`, `"%-d. %B %Y"` for `de`). The format is passed to `forecast_date.format_localized(fmt, chrono_loc)`. Weekday and month names are produced by chrono via `%A` / `%a` / `%B` / `%b` and `format_localized` — they are **not** in the YAMLs. **Regional UI** where chrono matters: `pt-br`/`pt-pt`, `es-es`/`es-419` (chrono `es_ES` vs `es_MX`), `fr-fr`/`fr-ca` (`fr_FR` vs `fr_CA`).
- **Footer "Modifiers:" prefix** uses `t!("usage.modifiers_title")` and the continuation lines are indented dynamically by `" ".repeat(label.chars().count() + 1)`. This pattern is duplicated in three sites: `main.rs` forecast footer, `main.rs` drone footer, and `render::historical::write_hist_footer`. Keep them in sync.
- **Char-count == display-width assumption** holds for ASCII + Latin diacritics + Cyrillic only. Adding CJK (`zh-*`, `ja`, `ko`), Arabic, or Indic/Thai would break the column-width math (`chars().count()` undercounts double-width CJK and overcounts combining marks). The migration would be to switch every `chars().count()` in render code to `unicode_width::UnicodeWidthStr::width`.

## Releasing (version bump procedure)

The version is stored in **two different formats**, and both must be updated together for a release. Pushing a `v<version>` git tag triggers `.github/workflows/release.yml`, which builds cross-platform artifacts, publishes the GitHub release, and updates the Homebrew tap.

**Format 1 — full SemVer `X.Y.Z` (required by Cargo):**
- `Cargo.toml` — `version = "X.Y.Z"` (line 3)
- `Cargo.lock` — updates automatically on the next build

**Format 2 — short `X.Y` (used in user-facing text):**
- `src/types.rs` — `pub const VERSION: &str = "X.Y";` (line 3)
- `src/types.rs` — `assert_eq!(VERSION, "X.Y");` in the `version_is_current` test (line 11)
- `README.md` — the `**Terminal Weather Forecast** — vX.Y` subtitle and the two `e.g. vX.Y …` lines in the install instructions
- `README.crates.md` — the `**Terminal Weather Forecast** — vX.Y` subtitle

After bumping, run `cargo test` to confirm `version_is_current` passes and `Cargo.lock` is refreshed. The git tag itself should be `vX.Y` or `vX.Y.Z` — the release workflow strips the leading `v` when writing the Homebrew formula.
