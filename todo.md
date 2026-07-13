# stats command — follow-up cleanup

Remaining items after the partial refactor (month-header collision fix, Heatmap extraction, module split). Smaller polish and correctness follow-ups, sequenced by priority.

## Medium

- [ ] Reconcile `>9` display saturation with the gradient domain max. Display caps at `>9` but the gradient's last anchor is `10.0`, so a 10-KB cell shows `>9` with the top-of-gradient color while a 9-KB cell shows `9` one step below — breakpoint and visual ramp disagree by one. Pick one source of truth (either display at `>=10` and keep domain max `10`, or display `>9` and set domain max `9`). In `src/commands/stats/heatmap.rs::Heatmap::cell` + `DISPLAY_FLOOR`.
- [ ] Pin test data timezone. `generate_multi_year_metadata` (and similar helpers) anchor `NaiveDateTime` via `Local`, so the same entry lands in different ISO weeks depending on the runner's tz. The snapshot is only reproducible in one tz. Switch to `chrono::FixedOffset` or `Utc` in `src/commands/stats/mod.rs` test helpers so ISO-week assignment is deterministic.
- [ ] Move `console::set_colors_enabled(true)` / `set_true_colors_enabled(true)` out of `Command::execute` into a one-time startup call (e.g. in `src/main.rs` before subcommand dispatch). Currently `execute` mutates process-global terminal state every run and `test_stats_snapshot_multi_year` sets both to `true` to match (so its snapshot stays deterministic regardless of test-thread scheduling). Once it's startup-only, drop the defensive toggling in tests.

## Low

- [ ] Drop the `Year(u32)` newtype or make it `Year(i32)`. Every call site does `year.0 as i32` or `Year(year as u32)` because chrono speaks `i32` everywhere; the newtype currently buys nothing but casts. If keeping it for documentation value, switch the inner type to `i32`. In `src/commands/stats/index.rs`.
- [ ] Add a parametric test table for `weekday_range` over several years × all 7 weekdays. Only 3 cases are covered today; assert `min_kw ∈ {1, 2}` and that a Monday in week `min_kw` actually exists. In `src/commands/stats/mod.rs` tests.

## Done (for reference)

- [x] Distinguish 0-KB cells from 1-KB cells in the heatmap (render empty weeks as plain spaces, no background).
- [x] Remove the stale `#[allow(dead_code)]` on `EntryMetadata::size` in `src/entry/repository.rs:9` — `size` is now consumed by the stats index, so the allow is no longer needed.
- [x] Fix month-header collision on partial years (clamp `month_week_ranges` to `n_weeks`).
- [x] Extract `Heatmap` struct (gradient + contrast + palette) built once, reused per row.
- [x] Split `stats.rs` into `stats/{mod,index,heatmap,render}.rs`.
- [x] `last_used_week` returns `Option<usize>` (no `0` conflation with "skip this year").
- [x] `MockUserOutput::expect_print` asserts the printed string is non-empty.