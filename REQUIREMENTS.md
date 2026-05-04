# Requirements — garmin-fit tools

This document is the single source of truth for functional and non-functional requirements of the garmin-fit toolchain. It is intended to be kept up-to-date as the project evolves and to serve as a reference when planning new features.

**Status legend**

| Symbol | Meaning |
|---|---|
| ✅ | Implemented and verified |
| 🔧 | Partially implemented (scaffolding exists, incomplete behaviour) |
| 📋 | Planned, not yet started |

Related documents:
- [README.md](README.md) — user-facing usage guide
- [DESIGN.md](DESIGN.md) — architecture and module design
- [CLI_CONCEPT.md](CLI_CONCEPT.md) — detailed subcommand flag reference

---

## Table of contents

1. [Product vision](#1-product-vision)
2. [Non-functional requirements](#2-non-functional-requirements)
3. [Architecture requirements](#3-architecture-requirements)
4. [Global CLI requirements](#4-global-cli-requirements)
5. [fit2json — subcommand requirements](#5-fit2json--subcommand-requirements)
6. [fitdir — batch directory tool](#6-fitdir--batch-directory-tool)
7. [fithistory — Garmin Connect export tool](#7-fithistory--garmin-connect-export-tool)
8. [Future subcommands and features](#8-future-subcommands-and-features)
9. [Configuration file](#9-configuration-file)
10. [Change log](#10-change-log)

---

## 1. Product vision

### REQ-VIS-001 ✅
The toolchain shall read and decode Garmin FIT binary activity files into structured, queryable data.

### REQ-VIS-002 ✅
The toolchain shall support three usage modes: single-file interactive queries (`fit2json`), batch processing of a directory (`fitdir`), and bulk ingestion of a Garmin Connect export archive (`fithistory`).

### REQ-VIS-003 ✅
All domain logic shall reside in a reusable library crate (`fitlib`) that is independent of any specific CLI tool and can eventually be published to crates.io.

### REQ-VIS-004 ✅
The toolchain shall be implemented in Rust for memory safety, predictable performance, and strong type guarantees.

---

## 2. Non-functional requirements

### Performance

#### REQ-PERF-001 ✅
Processing a typical single-activity FIT file (up to 10 000 records) shall complete in under 200 ms on modern hardware, including JSON serialisation to stdout.

#### REQ-PERF-002 ✅
Per-file record scanning shall use sequential iteration. Rayon parallelism shall not be used within a single file's processing pipeline; the overhead of thread-pool coordination exceeds the work for file sizes of 3 000–8 000 records.

#### REQ-PERF-003 ✅
When processing a directory of FIT files (`fitdir`), parallelism shall be applied at the file level using `rayon::par_iter`, where the work per task (parsing an entire file) is large enough to amortise thread-pool overhead.

#### REQ-PERF-004 ✅
Filter functions (`select_kind`, `select_kind_with_ts`) shall return references into the original record slice rather than clones. Cloning shall occur only once at the output boundary (JSON serialisation), not at intermediate filter steps.

### Safety and correctness

#### REQ-SAFE-001 ✅
No production code path shall call `unwrap()` or `expect()` on a `Result` or `Option` that can realistically be `None`/`Err`. All such paths shall use `?` propagation or explicit `match` with a meaningful error.

#### REQ-SAFE-002 ✅
The library shall define a single typed error enum (`FitError`) using `thiserror`, allowing downstream callers to match on specific error variants. Binary crates may wrap library errors with `anyhow` for ergonomic reporting.

#### REQ-SAFE-003 ✅
No information shall be silently lost during JSON serialisation. Fields that cannot be serialised shall produce an error, not be dropped.

#### REQ-SAFE-004 ✅
The tool shall never modify its input FIT file.

### Code quality

#### REQ-QUAL-001 ✅
All crates in the workspace shall compile with zero Clippy warnings when run with `cargo clippy --workspace -- -D warnings`.

#### REQ-QUAL-002 ✅
All crates shall use the `edition = "2024"` Rust edition setting.

#### REQ-QUAL-003 ✅
All public library functions shall carry doc comments explaining their purpose, parameters, and return value.

### Portability

#### REQ-PORT-001 📋
The toolchain shall compile and run correctly on Linux, macOS, and Windows. No platform-specific APIs shall be used directly; cross-platform abstractions (`std::path::Path`, `walkdir`) shall be used instead.

---

## 3. Architecture requirements

### REQ-ARCH-001 ✅
The repository shall be structured as a Cargo workspace with members `fitlib`, `fit2json`, `fitdir`, and `fithistory`, sharing a single `Cargo.lock` and `[workspace.dependencies]` table.

### REQ-ARCH-002 ✅
Binary crates (`fit2json`, `fitdir`, `fithistory`) shall contain only argument parsing and dispatch logic. No business logic (parsing, filtering, aggregation, formatting) shall be written in binary crates.

### REQ-ARCH-003 ✅
`fitlib` shall not depend on `clap`, `anyhow`, or any other binary-oriented crate. Its dependency tree shall remain minimal and suitable for embedding in other Rust projects.

### REQ-ARCH-004 ✅
`fitparser` shall be called exclusively from `fitlib::parse`. No binary crate shall call `fitparser` directly except to use its types in function signatures.

### REQ-ARCH-005 ✅
The FIT containment hierarchy (Activity → Session → Lap → Record) shall be reconstructed by `fitlib::hierarchy::build_activity` using timestamp-range containment. `FitLap` shall store indices into the original flat slice (`record_indices: Vec<usize>`) rather than cloned records, keeping the structs `'static`.

---

## 4. Global CLI requirements

These requirements apply to all subcommands of all binary tools.

### REQ-CLI-001 ✅
Every subcommand shall accept its primary input FIT file either as a positional argument or via the global `--input` / `-i` flag.

### REQ-CLI-002 ✅
Output shall go to stdout by default. The `--output` / `-o` flag redirects output to a file.

### REQ-CLI-003 ✅
JSON output shall default to compact form when writing to stdout, and to pretty-printed form when writing to a file. The `--pretty` and `--compact` flags override these defaults. The two flags are mutually exclusive.

### REQ-CLI-004 ✅
All timestamps in output shall default to the system local timezone. `--utc` forces UTC; `--timezone <IANA>` selects any named timezone. `--utc` and `--timezone` are mutually exclusive.

### REQ-CLI-005 📋
`--timezone <IANA>` shall accept standard IANA timezone identifiers (e.g. `Europe/Berlin`, `America/New_York`). This requires the `chrono-tz` crate.

### REQ-CLI-006 ✅
`--no-unknown` shall suppress fields whose names are not defined in the FIT profile and fields originating from Connect IQ `developer_data_id` entries.

### REQ-CLI-007 ✅
`--version` / `-V` shall print the tool version and the `fitparser` FIT profile version in use, then exit.

### REQ-CLI-008 ✅
`--help` / `-h` shall print usage and flag descriptions for the current subcommand, then exit. This is provided automatically by `clap`.

### REQ-CLI-009 ✅
Timestamp arguments (`--from`, `--until`) shall accept:
- ISO 8601 with timezone offset: `2026-04-23T09:58:43+02:00`
- ISO 8601 without timezone (assumed local): `2026-04-23T09:58:43`
- `HH:MM:SS` relative offset from activity start

---

## 5. fit2json — subcommand requirements

### 5.1 `dump` — full data extraction

#### REQ-DUMP-001 ✅
`dump` shall serialise all records in the FIT file to JSON and write to the output destination.

#### REQ-DUMP-002 ✅
`--split` shall write one JSON file per message type into the directory specified by `--output-dir` (default: current directory). Each file shall be named `<kind>.json`.

#### REQ-DUMP-003 📋
`--include-raw` shall include the raw integer value alongside the decoded string value for every enumerated field in the output.

---

### 5.2 `info` — activity summary

#### REQ-INFO-001 🔧
`info` shall output a summary of the FIT file. At minimum this shall include:
- File metadata: creator device, `time_created`, FIT profile version
- Number of sessions (`activity.num_sessions`)
- Per-session: sport type, sub-sport, start time, end time, duration
- GPS bounding box (min/max latitude, longitude, altitude)
- Message-type counts
- High-level performance metrics per session: total distance, total ascent/descent, average and max heart rate, average and max speed, average and max power, total calories
- Normalised power (if power data is present)
- Connected sensors and devices from `device_info` records
- Whether Connect IQ developer fields are present

#### REQ-INFO-002 🔧
`--format json|text|table` shall select the output format. Default: `text`.

#### REQ-INFO-003 📋
`--counts-only` shall print only the message-type count table and exit.

---

### 5.3 `types` — message type listing

#### REQ-TYPES-001 ✅
`types` shall list every FIT message kind present in the file with its record count.

#### REQ-TYPES-002 ✅
`--sort count` (default) shall sort by count descending, then alphabetically by name as a tiebreaker.

#### REQ-TYPES-003 ✅
`--sort name` shall sort alphabetically by message type name.

#### REQ-TYPES-004 ✅
`--format json|text|table` shall select the output format. Default: `table`.

---

### 5.4 `select` — record query

#### REQ-SEL-001 ✅
`select --type <name>` (required) shall filter records by message type. Type names shall be case-insensitive.

#### REQ-SEL-002 ✅
`--from` and `--until` shall restrict results to the timestamp range `[from, until)`.

#### REQ-SEL-003 ✅
`--duration <seconds>` shall set `until = from + duration` and is mutually exclusive with `--until`.

#### REQ-SEL-004 📋
`--session <n>` shall restrict results to records belonging to session `n` (1-based). The flag shall be repeatable to select multiple sessions. Requires the hierarchy to be reconstructed.

#### REQ-SEL-005 📋
`--lap <n>` shall restrict results to records belonging to lap `n` within the selected session(s) (1-based). The flag shall be repeatable and shall accept ranges in the form `n-m` (e.g. `--lap 2-5`).

#### REQ-SEL-006 📋
`--field <name><op><value>` shall filter records by field value where `op` is one of `=`, `!=`, `>`, `<`, `>=`, `<=`. The flag shall be repeatable; multiple predicates are combined with AND logic.

#### REQ-SEL-007 📋
`--fields <f1,f2,...>` shall project the output to only the named fields per record.

#### REQ-SEL-008 ✅
`--limit <n>` shall return at most `n` matching records.

#### REQ-SEL-009 ✅
`--count` shall print only the integer count of matching records, not the records themselves.

---

### 5.5 `stats` — aggregated statistics

#### REQ-STATS-001 ✅
`stats` shall compute min, max, mean, and sum for every numeric field across the selected records.

#### REQ-STATS-002 ✅
`--by activity` (default) shall aggregate all records in the file into a single result.

#### REQ-STATS-003 ✅
`--by session` shall produce one result per session.

#### REQ-STATS-004 ✅
`--by lap` shall produce one result per lap; lap statistics shall always be scoped within their parent session.

#### REQ-STATS-005 ✅
`--session <n>` shall restrict `--by lap` aggregation to session `n`.

#### REQ-STATS-006 ✅
`--fields <f1,f2,...>` shall restrict aggregation to the named fields.

#### REQ-STATS-007 🔧
`--format json|text|table` shall select the output format. Default: `table`.

---

### 5.6 `gps` — GPS track extraction

#### REQ-GPS-001 ✅
`gps` shall extract the ordered GPS track from `record` messages.

#### REQ-GPS-002 ✅
Latitude and longitude stored as Garmin semicircles (signed 32-bit integers) shall be converted to decimal degrees using the formula `degrees = value × (180 / 2³¹)`.

#### REQ-GPS-003 ✅
`--format geojson` (default) shall output a GeoJSON `FeatureCollection` containing a single `LineString` feature.

#### REQ-GPS-004 ✅
`--format gpx` shall output a valid GPX 1.1 XML document.

#### REQ-GPS-005 ✅
`--format json` shall output the raw GPS point array as JSON.

#### REQ-GPS-006 ✅
`--bbox` shall print only the bounding box (min/max lat, lon, altitude) and exit.

#### REQ-GPS-007 📋
`--properties <f1,f2,...>` shall attach the named FIT fields as properties on each GeoJSON point.

#### REQ-GPS-008 📋
`--lap <n>` shall restrict the output to the GPS points recorded during lap `n`.

#### REQ-GPS-009 📋
`--from` / `--until` / `--duration` shall restrict the output to GPS points within the given time window.

#### REQ-GPS-010 📋
`--session <n>` shall restrict the output to GPS points belonging to session `n`.

#### REQ-GPS-011 📋
`--simplify <tolerance>` shall apply Ramer-Douglas-Peucker simplification to the track using `tolerance` as the epsilon value in degrees.

---

### 5.7 `events` — event log

#### REQ-EVT-001 ✅
`events` shall extract all `event` records from the file with their timestamp, event type, and decoded event data.

#### REQ-EVT-002 ✅
`--type <event_type>` shall filter by event type name (e.g. `timer`, `lap`, `workout_step`). The flag shall accept a comma-separated list of types.

#### REQ-EVT-003 🔧
`--format json|text|table` shall select the output format. Default: `table`.

---

### 5.8 `sessions` — session summary

#### REQ-SESS-001 ✅
`sessions` shall list each session in the file with its 1-based index, sport type, sub-sport, start time, end time, and number of laps.

#### REQ-SESS-002 🔧
`--format json|text|table` shall select the output format. Default: `table`.

---

### 5.9 `laps` — lap summary

#### REQ-LAPS-001 ✅
`laps` shall list each lap with its 1-based index within its session, start time, end time, and record count.

#### REQ-LAPS-002 ✅
`--session <n>` shall restrict output to laps belonging to session `n`. Without this flag, laps from all sessions are shown, grouped by session.

#### REQ-LAPS-003 🔧
`--format json|text|table` shall select the output format. Default: `table`.

---

### 5.10 `validate` — integrity check

#### REQ-VAL-001 ✅
`validate` shall check that `file_id` and `activity` messages are present. Absence of either is an `Error`-severity issue.

#### REQ-VAL-002 ✅
`validate` shall compare `activity.num_sessions` with the number of `session` records found. A mismatch is a `Warning`-severity issue.

#### REQ-VAL-003 ✅
`validate` shall check that `record` timestamps are monotonically non-decreasing. Each violation is counted; the total is reported as a `Warning`.

#### REQ-VAL-004 ✅
`validate` shall report the presence of `developer_data_id` records as an `Info`-severity issue so the user knows Connect IQ custom fields may be present.

#### REQ-VAL-005 📋
`validate` shall verify the 2-byte CRC at the end of the file. A mismatch is an `Error`-severity issue. (Blocked on `fitparser` exposing the raw byte stream.)

#### REQ-VAL-006 📋
`validate` shall detect GPS outliers: consecutive position jumps exceeding 1 000 m in less than 1 second shall be flagged as `Warning`-severity issues.

#### REQ-VAL-007 ✅
`validate` shall exit with a non-zero status code if any `Error`-severity issue is found.

#### REQ-VAL-008 🔧
`--format json|text` shall select the output format. Default: `text`.

---

### 5.11 `compare` — file comparison

#### REQ-CMP-001 ✅
`compare <file1> <file2>` shall compute aggregated statistics for both files and present them side by side.

#### REQ-CMP-002 ✅
`--by activity|session|lap` shall select the aggregation granularity. Default: `activity`.

#### REQ-CMP-003 ✅
`--fields <f1,f2,...>` shall restrict the comparison to the named fields.

---

## 6. fitdir — batch directory tool

### REQ-DIR-001 ✅
`fitdir survey --dir <path>` shall discover and process all `*.fit` files found at the given path.

### REQ-DIR-002 ✅
`--recursive` (`-r`) shall extend discovery to all subdirectories.

### REQ-DIR-003 ✅
`--jobs <n>` (`-j`) shall control the number of worker threads used for parallel file processing. Default: number of logical CPUs.

### REQ-DIR-004 ✅
`fitdir` shall use named subcommands (e.g. `survey`, `list`, `info`, `dump`, `validate`) rather than a generic `--subcommand <name>` flag. This follows the same clap subcommand pattern as `fit2json` and allows each subcommand to define its own flags cleanly. (`survey` and `list` are implemented; further subcommands are planned.)

### REQ-DIR-005 📋
`--output-dir <path>` shall write one output file per input file into the specified directory. *(Planned for `info` and `dump` batch subcommands.)*

### REQ-DIR-006 ✅
A file that fails to parse shall produce an error message on stderr and be skipped; processing of other files shall continue.

### REQ-DIR-007 ✅
`fitdir` shall use `rayon::par_iter` for file-level parallelism. No parallelism shall be applied within the processing of a single file.

---

### 6.1 `survey` — directory overview

#### REQ-DIR-SURVEY-001 ✅
`fitdir survey` shall scan a directory of FIT files and produce a per-type statistical overview grouped by the `file_id.type` field.

#### REQ-DIR-SURVEY-002 ✅
For each distinct file type the report shall include:
- File count.
- Byte-size distribution: minimum, mean, median, and maximum.
- Record-count distribution: minimum, mean, median, and maximum.
- Date range: oldest and newest `time_created` from the `file_id` record (ISO 8601 date strings).

#### REQ-DIR-SURVEY-003 ✅
`--format table` (default) shall render a human-readable aligned table sorted by file count descending.

#### REQ-DIR-SURVEY-004 ✅
`--format json` shall emit a JSON array of per-type statistics objects with raw byte values, suitable for downstream processing with `jq` or import into a database.

#### REQ-DIR-SURVEY-005 ✅
The per-file data collection logic (`fitlib::survey::collect_sample`) and the aggregation logic (`fitlib::survey::summarize`) shall reside in `fitlib`, not in `fitdir`. `fitdir` is responsible only for directory traversal, parallelism, and rendering.

---

### 6.2 `list` — per-file listing

#### REQ-DIR-LIST-001 ✅
`fitdir list` shall enumerate individual FIT files in a directory, one per output row, and show the path, file type, file size, record count, and `time_created` date for each file.

#### REQ-DIR-LIST-002 ✅
`--type <type>` (`-t`) shall filter the results to files whose `file_id.type` matches the given string (case-insensitive). The flag shall be repeatable; a file is included if it matches any of the specified types. Omitting `--type` shall list all files regardless of type.

#### REQ-DIR-LIST-003 ✅
The `--sort` flag shall control the primary sort key:

| Value | Primary key | Tiebreaker |
|---|---|---|
| `date` (default) | `time_created` | full path |
| `size` | `size_bytes` | full path |
| `records` | `record_count` | full path |
| `name` | filename (case-insensitive) | `time_created` |

Files without a `time_created` date shall always sort after files that have one, regardless of whether `--desc` is set.

#### REQ-DIR-LIST-004 ✅
`--desc` shall reverse the primary sort order.

#### REQ-DIR-LIST-005 ✅
`--limit <n>` (`-n`) shall truncate the output to at most `n` rows after sorting.

#### REQ-DIR-LIST-006 ✅
`--format table` (default) shall render a human-readable aligned table with columns: row number, date (`YYYY-MM-DD` or `—`), type, size (K/M), record count, file path.

#### REQ-DIR-LIST-007 ✅
`--format json` shall emit a JSON array of objects, one per file, with raw numeric byte values.

#### REQ-DIR-LIST-008 ✅
The `FileEntry` struct (`fitlib::survey::FileEntry`) and `to_file_entry` constructor shall reside in `fitlib`. `fitdir list` is responsible only for directory traversal, parallelism, filtering, sorting, and rendering.

#### REQ-DIR-LIST-009 ✅
`--sport <sport>` (short: `-s`) shall filter results to files where *at least one* session record has a matching `sport` value. The flag shall be repeatable (`--sport cycling --sport running`); a file is included if any session matches any of the requested sports. Matching shall be case-insensitive.

#### REQ-DIR-LIST-010 ✅
The table output shall include a `Sport` column showing the sport(s) of the file's session records. Non-activity files (no session records) shall display `—`. A file with multiple sessions that have *different* sports shall display all unique sports joined by `+` (e.g. `cycling+running`); this concatenated form serves as the multi-sport marker. Sub-sport is not shown in the table but is present in JSON output as `sub_sports`.

---

## 7. fithistory — Garmin Connect export tool

### REQ-HIST-001 📋
`fithistory --zip <path>` shall open a Garmin Connect bulk-export ZIP archive without extracting it to disk.

### REQ-HIST-002 📋
`fithistory` shall iterate all `*.fit` entries in the ZIP and process each one via `fitlib::parse::load_reader`.

### REQ-HIST-003 📋
`--output-dir <path>` shall write one JSON output file per FIT entry into the specified directory.

### REQ-HIST-004 📋
`--since <date>` and `--until <date>` shall filter entries by the activity's start timestamp so that only activities within the given date range are processed. Both accept ISO 8601 date or datetime strings.

### REQ-HIST-005 📋
Entries in the ZIP that are not `*.fit` files (e.g. GPX files, CSV summaries) shall be silently skipped.

### REQ-HIST-006 📋
A ZIP entry that fails to parse as a FIT file shall produce an error message on stderr and be skipped; remaining entries shall continue to be processed.

---

## 8. Future subcommands and features

These requirements capture ideas discussed during design; they have no committed timeline.

### 8.1 `zones` — time-in-zone analysis

#### REQ-ZONES-001 📋
`zones --type hr` shall compute time spent in each heart rate zone. Zone boundaries shall be read from the `hr_zone` FIT messages in the file if present, or from `--zones <b1,b2,...>` on the command line.

#### REQ-ZONES-002 📋
`zones --type power --ftp <n>` shall compute time in each power zone relative to the given FTP.

#### REQ-ZONES-003 📋
`zones --type pace` shall compute time in each pace zone (min/km or min/mile).

### 8.2 `devices` — sensor listing

#### REQ-DEV-001 📋
`devices` shall list all `device_info` records: device name, manufacturer, serial number, battery status, and ANT+/BLE sensor IDs.

### 8.3 `user` — athlete profile

#### REQ-USER-001 📋
`user` shall extract `user_profile` (name, weight, age, max HR) and `zones_target` (FTP, HR zones, pace zones) from the file.

### 8.4 `workouts` — structured workout steps

#### REQ-WKT-001 📋
`workouts` shall extract `workout` and `workout_step` messages, displaying step names, target types (HR, power, pace), target values or ranges, and duration conditions.

### 8.5 Shell completions

#### REQ-COMP-001 📋
`fit2json completions <shell>` shall generate shell completion scripts for `bash`, `zsh`, and `fish` using `clap`'s built-in completion generation.

### 8.6 TCX export

#### REQ-TCX-001 📋
`gps --format tcx` shall export the GPS track and HR data as a Garmin Training Center XML (TCX) file, reusing the `quick-xml` dependency already in `fitlib`.

### 8.7 `fitlib` publication

#### REQ-LIB-001 📋
`fitlib` shall be published to crates.io with complete API documentation, a stable public interface, and semantic versioning.

---

## 9. Configuration file

### REQ-CFG-001 📋
`fit2json` shall look for a configuration file at `~/.config/fit2json/config.toml` (XDG base directory convention).

### REQ-CFG-002 📋
The configuration file shall support the following settings, all of which may be overridden by command-line flags:

```toml
[defaults]
format   = "table"        # default output format
timezone = "Europe/Berlin"
pretty   = true

[athlete]
ftp      = 280            # functional threshold power (watts)
max_hr   = 192            # maximum heart rate (bpm)
hr_zones = [100, 140, 160, 175, 192]  # HR zone boundaries
```

### REQ-CFG-003 📋
If no configuration file is found the tool shall start without error using built-in defaults.

---

## 10. Change log

| Date | Change |
|---|---|
| 2026-05-03 | Initial version; captures all requirements discussed during project inception and first implementation sprint. |
| 2026-05-03 | Marked REQ-PERF-003, REQ-DIR-001/002/003/006/007 ✅. Revised REQ-DIR-004 to reflect subcommand-based CLI design. Added REQ-DIR-SURVEY-001–005 for the implemented `survey` subcommand. |
| 2026-05-03 | Marked REQ-DIR-004 ✅ (both `survey` and `list` now implemented). Added REQ-DIR-LIST-001–008 for the implemented `list` subcommand. |
| 2026-05-03 | Added REQ-DIR-LIST-009–010 for `--sport` filter and multi-sport marker in `fitdir list`. |
