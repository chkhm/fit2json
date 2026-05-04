# Design document — garmin-fit tools

This document describes the software architecture, module layout, key design decisions, and future roadmap for the garmin-fit toolchain.

For user-facing usage examples see [README.md](README.md).  
For the full CLI flag reference for each subcommand see [CLI_CONCEPT.md](CLI_CONCEPT.md).  
For the requirements catalogue see [REQUIREMENTS.md](REQUIREMENTS.md).

---

## Table of contents

1. [Goals](#goals)
2. [FIT file format overview](#fit-file-format-overview)
3. [Repository layout](#repository-layout)
4. [Library crate — `fitlib`](#library-crate--fitlib)
5. [Binary crate — `fit2json`](#binary-crate--fit2json)
6. [Binary crate — `fitdir`](#binary-crate--fitdir)
7. [Design decisions](#design-decisions)
8. [Dependency rationale](#dependency-rationale)
9. [Roadmap](#roadmap)
10. [FIT format stability and versioning strategy](#fit-format-stability-and-versioning-strategy)

---

## Goals

- **Fast**: FIT files are small (3 000–8 000 records for a typical activity); the tool should feel instantaneous. Parallelism is reserved for the one place it earns its keep: processing many files concurrently.
- **Safe**: Rust is chosen specifically to eliminate memory-safety bugs and provide expressive error handling without exceptions.
- **Composable**: Every subcommand writes to stdout by default so output can be piped to `jq`, `csvkit`, `psql`, or any other tool.
- **Reusable**: All domain logic lives in a library crate (`fitlib`) that can be used by other binaries in this workspace and eventually published to crates.io.

---

## FIT file format overview

A Garmin FIT (Flexible and Interoperable Data Transfer) file is a compact binary format. It consists of a fixed file header, a chronological stream of definition and data messages, and a 2-byte CRC footer.

Although the stream is flat, it encodes a strict logical containment hierarchy:

```
[File ID]
└── [Activity]  ← carries num_sessions
    ├── [Session 0]  (e.g. Bike leg of a triathlon)
    │   ├── [Lap 0]
    │   │   ├── Record  (timestamp, GPS, HR, power, cadence, …)
    │   │   ├── Record
    │   │   └── Event   (timer paused, lap triggered, …)
    │   └── [Lap 1]
    │       └── Record …
    └── [Session 1]  (e.g. Run leg)
        └── [Lap 0]
            └── Record …
```

**Key message types:**

| Message | Role |
|---|---|
| `file_id` | Root descriptor: device, manufacturer, serial number, `time_created`. |
| `activity` | Top-level container; carries `num_sessions`. |
| `session` | One sport/leg. Multi-sport files (triathlon) can have five. |
| `lap` | Segment within a session; carries `start_time` and `timestamp` (end time). |
| `record` | Real-time sensor data recorded every second (or at smart-recording intervals). |
| `event` | State changes: timer start/pause/resume, lap trigger, power-off, workout step. |
| `device_info` | Hardware details, firmware version, battery status. |
| `developer_data_id` | Identifies custom Connect IQ data fields. |

**Hierarchy reconstruction:** `lap` and `session` summary messages appear *after* their child records in the stream (they are summaries). `fitlib::hierarchy::build_activity` reconstructs the tree by matching each `record` timestamp against the `[start_time, timestamp]` windows declared on `lap` messages, then grouping laps into sessions the same way.

---

## Repository layout

The repository is a **Cargo workspace**. All crates share a single `Cargo.lock` and a `[workspace.dependencies]` table so version pins are managed in one place.

```
fit2json/                       ← workspace root
├── Cargo.toml                  ← [workspace] only
├── Cargo.lock
├── README.md
├── DESIGN.md                   ← this file
├── CLI_CONCEPT.md              ← detailed subcommand flag reference
├── Research.md                 ← background notes on the FIT format
├── test-data/                  ← shared sample .fit files
│
├── fitlib/                     ← core library crate
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── error.rs
│       ├── fields.rs
│       ├── parse.rs
│       ├── filter.rs
│       ├── hierarchy.rs
│       ├── timestamp.rs
│       ├── stats.rs
│       ├── gps.rs
│       ├── validate.rs
│       └── survey.rs
│
├── fit2json/                   ← primary CLI binary
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       ├── cli.rs
│       └── commands/
│           ├── mod.rs
│           ├── dump.rs
│           ├── info.rs
│           ├── types.rs
│           ├── select.rs
│           ├── stats.rs
│           ├── gps.rs
│           ├── events.rs
│           ├── sessions.rs
│           ├── laps.rs
│           ├── validate.rs
│           ├── compare.rs
│           └── filetype.rs
│
├── fitdir/                     ← batch-directory binary
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       ├── cli.rs
│       └── commands/
│           ├── mod.rs
│           └── survey.rs
│
└── fithistory/                 ← Garmin Connect ZIP binary (stub)
    ├── Cargo.toml
    └── src/main.rs
```

**Rule:** No business logic lives in binary crates. Binary crates contain only argument parsing and dispatch. All domain logic lives in `fitlib`.

---

## Library crate — `fitlib`

### `error.rs` — unified error type

```rust
pub enum FitError {
    Io(std::io::Error),             // file open / read failure
    Parse(fitparser::Error),        // binary decode failure
    MissingMessage(String),         // required message absent
    TimestampMissing { kind },      // field absent or wrong type
    IntegrityFailure(String),       // CRC or structural check
    NoGpsData,                      // GPS subcommand on non-GPS file
    SessionOutOfRange(usize),       // --session N exceeds file
    LapOutOfRange(usize, usize),    // --lap N exceeds session
}
```

Binary crates wrap `FitError` with `anyhow` for ergonomic `?`-propagation and human-readable error messages. Library callers can match on variants for structured handling.

### `fields.rs` — centralised field-name resolver

```rust
pub fn resolve_field<'r>(record: &'r FitDataRecord, logical: &str) -> Option<&'r Value>
pub fn field_f64(record: &FitDataRecord, logical: &str) -> Option<f64>
pub fn field_u32(record: &FitDataRecord, logical: &str) -> Option<u32>
pub fn field_altitude(record: &FitDataRecord) -> Option<f64>
```

This module is the **single place to update** when Garmin introduces a new `enhanced_*` field variant on newer firmware — see [FIT format stability and versioning strategy](#fit-format-stability-and-versioning-strategy) for the full rationale.

`resolve_field` checks whether a known `enhanced_*` alias exists for the requested logical name, tries that first, then falls back to the literal name.  Adding a new alias is one line in its `match` arm.  All typed helpers (`field_f64`, `field_u32`, `field_altitude`) call `resolve_field` internally, so every call site throughout the codebase benefits automatically.

`field_altitude` has a special case beyond name resolution: when the raw `UInt16`/`UInt32` value is returned (fitparser without full profile decoding), it applies the FIT spec's scale/offset rule: `metres = raw / 5 − 500`.

### `parse.rs` — I/O entry points

```rust
pub fn load_file(path: &Path) -> Result<Vec<FitDataRecord>, FitError>
pub fn load_reader<R: Read>(reader: &mut R) -> Result<Vec<FitDataRecord>, FitError>
```

These are the only places in the codebase that call `fitparser`. `load_reader` is used by `fithistory` to parse FIT data extracted from a ZIP without writing to disk.

### `filter.rs` — record selection

```rust
pub fn count_kinds(data: &[FitDataRecord]) -> HashMap<String, usize>
pub fn select_kind(data: &[FitDataRecord], kind: MesgNum) -> Vec<&FitDataRecord>
pub fn select_kind_with_ts(data, kind, from, until) -> Vec<&FitDataRecord>
pub fn record_timestamp(record: &FitDataRecord) -> Option<DateTime<Local>>
```

All functions return **references** into the original slice; callers `.cloned()` only if they need owned values for serialisation. Sequential iteration is deliberate — rayon's thread-pool overhead (~50–200 µs) far exceeds the actual work for 3 000–8 000 records.

### `hierarchy.rs` — tree reconstruction

The central domain module. `build_activity` reconstructs the containment hierarchy from the flat record stream:

1. Collect all `lap` summary records (each carries `start_time` and `timestamp`).
2. For each lap, find all `record` messages whose timestamp falls in `[start_time, timestamp]`; store their indices into the original flat slice.
3. Collect all `session` records; assign laps by the same timestamp-containment rule.
4. Populate `FileIdInfo` from the first `file_id` record.

```rust
pub struct FitActivity { pub file_id: FileIdInfo, pub num_sessions: usize, pub sessions: Vec<FitSession> }
pub struct FitSession  { pub index: usize, pub sport, pub sub_sport, pub start_time, pub end_time, pub laps: Vec<FitLap> }
pub struct FitLap      { pub index: usize, pub start_time, pub end_time, pub record_indices: Vec<usize> }
```

Laps store `record_indices` rather than cloned records to keep memory usage proportional to the number of laps, not the number of records.

### `timestamp.rs` — display and parsing

```rust
pub enum TzMode { Local, Utc, Named(String) }
pub fn kind_and_ts_to_str(record: &FitDataRecord) -> String
pub fn format_ts(ts: DateTime<Local>, mode: &TzMode) -> String
pub fn parse_timestamp(s: &str, activity_start: Option<DateTime<Local>>) -> Result<DateTime<Local>, FitError>
```

`parse_timestamp` accepts ISO 8601 with or without a timezone offset, and also `HH:MM:SS` relative to the activity start time — matching the `--from` / `--until` flags on `select` and `gps`.

### `stats.rs` — numeric aggregation

```rust
pub fn aggregate(records: &[&FitDataRecord], field_filter: &[&str]) -> RecordSetStats
pub fn per_lap(data, session: &FitSession, field_filter) -> Vec<RecordSetStats>
pub fn per_session(data, activity: &FitActivity, field_filter) -> Vec<RecordSetStats>
```

`RecordSetStats` holds a `Vec<FieldStats>` where each entry contains `{ field, min, max, mean, sum, count }`. The `record_indices` stored on `FitLap` are used to project the flat slice without copying records.

### `gps.rs` — GPS extraction and export

```rust
pub fn extract_track(data: &[FitDataRecord]) -> Vec<GpsPoint>
pub fn bounding_box(track: &[GpsPoint]) -> Option<BoundingBox>
pub fn to_geojson(data, properties_filter) -> Result<serde_json::Value, FitError>
pub fn to_gpx(data) -> Result<String, FitError>
```

Garmin devices store latitude and longitude as **semicircles** (signed 32-bit integers). Conversion: `degrees = value × (180 / 2³¹)`. GPX output is produced with `quick-xml` rather than a purpose-built GPX crate, which is unmaintained; this also gives a clean foundation for future TCX export.

GeoJSON is constructed as `serde_json::Value` directly — no additional crate is needed for a standard LineString Feature.

### `survey.rs` — directory-level statistics

The module supports two usage patterns: aggregate statistics (used by `fitdir survey`) and per-file listing (used by `fitdir list`).

**Aggregate path:**

```rust
pub struct FileSurveySample { pub file_type: String, pub size_bytes: u64,
                               pub record_count: usize, pub time_created: Option<DateTime<Local>> }
pub struct TypeStats { pub file_type: String, pub file_count: usize,
                       pub size_min_bytes: u64, pub size_max_bytes: u64,
                       pub size_mean_bytes: f64, pub size_median_bytes: f64,
                       pub records_min: usize, pub records_max: usize,
                       pub records_mean: f64, pub records_median: f64,
                       pub oldest_date: Option<String>, pub newest_date: Option<String> }

pub fn collect_sample(size_bytes: u64, data: &[FitDataRecord]) -> FileSurveySample
pub fn summarize(samples: &[FileSurveySample]) -> Vec<TypeStats>
```

`collect_sample` is the per-file primitive: it calls `crate::file_type(data)` for the type string, reads `data.len()` for the record count, and scans the `file_id` record once for `time_created`.  It is designed to be called from a `rayon::par_iter` — it takes only a `u64` and a slice reference, not a path, so filesystem access (the `metadata` call for `size_bytes`) stays in the caller.

`summarize` groups samples by file type into a `HashMap`, sorts the per-group size and record-count vectors (for exact median), and builds one `TypeStats` per distinct type.  Output is sorted by `file_count` descending so the most common type appears first.  The entire computation is single-threaded and allocation-light; the expensive part is the parallel parse phase in `fitdir`, not the aggregation.

**Per-file listing path:**

```rust
pub struct FileEntry {
    pub path: PathBuf,
    pub file_type: String,
    pub size_bytes: u64,
    pub record_count: usize,
    pub time_created: Option<DateTime<Local>>,
    pub sports: Vec<String>,      // one entry per session; empty for non-activity files
    pub sub_sports: Vec<String>,  // parallel to sports
}

pub fn to_file_entry(path: PathBuf, size_bytes: u64, data: &[FitDataRecord]) -> FileEntry
```

`FileEntry` derives `Serialize` so it is emitted directly as JSON by `fitdir list --format json`.  `sports` / `sub_sports` are populated by scanning `session` records in the flat message slice — no call to `build_activity()` is needed.  Multiple entries in `sports` indicate a multi-sport file (triathlon, duathlon); `fitdir list` displays them joined by `+` in the `Sport` table column as a visual multi-sport marker.

### `validate.rs` — integrity checks

`validate(data)` runs all checks and returns a `ValidationReport`:

- **Required messages**: `file_id` must be present; `activity` message required only for activity-type files.
- **Session count**: `activity.num_sessions` must match the number of `session` records found (activity files only).
- **Timestamp ordering**: `record` timestamps must be monotonically non-decreasing.
- **Developer fields**: noted as `Info` if `developer_data_id` messages are present (signals Connect IQ custom data).
- **File type note**: for non-activity files an `Info`-level issue is emitted identifying the type, and the activity-specific checks are skipped rather than reported as errors.

CRC verification is not yet implemented at the `fitparser` API level; it will be added when the upstream crate exposes the raw byte stream.

---

## Binary crate — `fit2json`

### `main.rs`

Three lines: declare the two modules, parse the CLI with `clap`, call `commands::dispatch`.

```rust
mod cli;
mod commands;

fn main() -> anyhow::Result<()> {
    commands::dispatch(cli::Cli::parse())
}
```

### `cli.rs`

Defines `Cli` (the top-level clap `Parser`), `GlobalArgs` (flags shared across all subcommands via `#[command(flatten)]`), and the `Command` enum (one variant per subcommand, each holding its own `*Args` struct).

`GlobalArgs` fields are declared `global = true` so clap accepts them in any position on the command line.

### `commands/mod.rs`

Declares the subcommand modules and exposes three shared helpers used by every subcommand:

| Helper | Purpose |
|---|---|
| `resolve_input` | Picks the input path from the subcommand positional arg or the global `--input` flag. |
| `write_output` | Writes a string to the `--output` file or to stdout. |
| `to_json` | Serialises a value as pretty or compact JSON based on the global flags. |
| `require_activity_file` | Returns an error with a descriptive message if the parsed data is not from an activity file; passes through when the type is `"activity"` or cannot be determined. Called at the top of every subcommand that requires the session/lap hierarchy. |

### Each subcommand module

Every file under `commands/` follows the same pattern:

```rust
#[derive(clap::Args)]
pub struct XxxArgs { /* subcommand-specific flags */ }

pub fn run(global: &GlobalArgs, args: XxxArgs) -> anyhow::Result<()> {
    let data = fitlib::parse::load_file(&resolve_input(global, &args.input)?)?;
    // call fitlib functions
    // write_output(global, &to_json(global, &result)?)
}
```

No domain logic is written in these files; they are pure glue between the CLI surface and `fitlib`.

---

## Binary crate — `fitdir`

`fitdir` processes all `*.fit` files in a directory.  The outer loop uses `walkdir` for cross-platform recursive traversal and `rayon::par_iter` for **file-level** parallelism — the one context where rayon pays off.  Per-file logic is entirely delegated to `fitlib`.

### `cli.rs`

Defines `Cli` (global output flags: `--output`, `--pretty`, `--compact`) and a `Command` enum with one variant per subcommand, following the same pattern as `fit2json`.  Global output flags are separated into an `OutputOpts` struct before dispatch so the `Command` enum variant can be moved without a partial-move error.

### `commands/mod.rs`

Dispatches to subcommand `run` functions and exposes two shared helpers used by all subcommands:

| Helper | Purpose |
|---|---|
| `OutputOpts` | Holds `--output`, `--pretty`, `--compact`; `write(content)` writes to file or stdout; `use_pretty()` resolves the pretty/compact flag. |
| `collect_fit_paths` | Walks a directory with `walkdir` and returns all `*.fit` paths; respects `--recursive`. |

No per-file input helpers are needed because `fitdir` subcommands operate on directories rather than individual files.

### `commands/survey.rs` — implemented

Scans a directory with `collect_fit_paths`, parses files in parallel with `rayon::par_iter`, calls `fitlib::survey::collect_sample` per file, then calls `fitlib::survey::summarize` and renders the result.

```sh
fitdir survey --dir ~/activities/                  # table output
fitdir survey --dir ~/Garmin/ --recursive          # recursive
fitdir survey --dir ~/Garmin/ --format json        # machine-readable
fitdir survey --dir ~/Garmin/ --jobs 4             # limit parallelism
```

Files that fail to parse are logged to stderr and skipped (REQ-DIR-006); the scan continues.

### `commands/list.rs` — implemented

Lists individual files, one per row, with optional type filtering, configurable sort order, and a row limit.

```sh
fitdir list --dir ~/activities/                              # all files, sorted by date
fitdir list --dir ~/Garmin/ --type activity                  # only activity files
fitdir list --dir ~/Garmin/ --type activity --sort date --desc --limit 10
fitdir list --dir ~/Garmin/ --type monitoring_b --format json
```

**Sort fields**: `date` (default; `None` sorts last regardless of direction), `size`, `records`, `name` (case-insensitive filename).

**Tiebreaker**: all sort fields break ties on the full path, except `name` which breaks ties on `time_created`.

**`--desc`** reverses the primary sort key; the `None`-last invariant for `date` is preserved by reversing only the `Some`/`Some` comparison, not the `Some`/`None` sentinel.

**Sport filter**: `--sport cycling` (repeatable, short `-s`) retains only files where at least one session record has a matching sport value (case-insensitive). Multi-value: `--sport cycling --sport swimming` includes files matching either sport.

Output formats: `table` (aligned columns: `#`, `Date`, `Type`, `Sport`, `Size`, `Records`, `File`) and `json` (array of `FileEntry` objects with raw byte values).  The `Sport` column shows `—` for non-activity files, the sport name for single-sport files, and `sport1+sport2` for multi-sport files.  Sub-sport is not shown in the table but is present in the JSON `sub_sports` array.

### Future `fitdir` subcommands

The subcommand architecture makes it straightforward to add further batch operations:

```sh
fitdir info    --dir ~/activities/           # one summary row per activity file
fitdir dump    --dir ~/activities/ --output-dir ./json/
fitdir validate --dir ~/activities/          # structural check on every file
```

---

## Future binary crate — `fithistory`

Reads a Garmin Connect bulk-export ZIP (the archive downloaded from the Connect website) without extracting it to disk.  The `zip` crate provides in-memory iteration over entries; each `*.fit` entry is piped directly to `fitlib::parse::load_reader`.

```sh
fithistory --zip garmin_export.zip --output-dir ./json/ --since 2024-01-01
```

---

## Design decisions

### Sequential iteration over FIT records

The original proof-of-concept used `rayon` parallel iterators inside the per-file scan functions. This was benchmarked and found to be counterproductive:

- A 1-hour activity produces ~3 000–8 000 total records.
- Per-item work is ~20 ns (enum comparison + Vec push).
- Rayon's thread-pool setup and work-stealing costs ~50–200 µs per call.
- Break-even is approximately 100 000 items with this per-item cost.

Sequential `iter().filter().collect()` is 2–5× faster for typical FIT files and has no surprising behaviour around ordering.

Rayon is kept as a workspace dependency because `fitdir` uses it at the **file level**, where the work per task (parsing a complete FIT file) is large enough to amortise the overhead.

### References over clones in filter functions

`select_kind` and `select_kind_with_ts` return `Vec<&FitDataRecord>` rather than `Vec<FitDataRecord>`. Cloning a `FitDataRecord` requires allocating a new `Vec<DataField>` with `String` copies for each field. For a 12 000-record file this is significant. Callers that need owned values for JSON serialisation call `.cloned()` on the slice — paid once at the output boundary, not on every intermediate filter step.

### `record_indices` in `FitLap`

`FitLap` stores indices into the original flat slice rather than references or cloned records. This allows `FitActivity`, `FitSession`, and `FitLap` to all be `'static` (no lifetime parameters), which makes them straightforward to return from functions, store in data structures, and serialise with `serde`. The cost is an extra level of indirection when accessing lap records, which is negligible compared to I/O and parse time.

### One error type for the library

`FitError` centralises all errors behind a `thiserror`-derived enum. This keeps the library's public API typed and matchable by downstream callers (e.g. `fitdir` can distinguish `FitError::Parse` from `FitError::Io` to decide whether to skip a file or abort). Binary crates wrap it with `anyhow` for ergonomic reporting — the two libraries complement each other rather than competing.

### `quick-xml` for GPX

The most commonly referenced Rust GPX crate (`gpx 0.10`) has not been maintained since 2022. `quick-xml` is actively maintained, produces clean XML, and will also serve future TCX export without adding another dependency.

---

## Dependency rationale

| Crate | Version | Used in | Why |
|---|---|---|---|
| `fitparser` | 0.10 | `fitlib` | The only maintained Rust FIT binary decoder; provides `FitDataRecord`, `MesgNum`, and `Value`. |
| `chrono` | 0.4 | `fitlib`, binaries | Timestamp types and arithmetic; `fitparser` uses `chrono::DateTime<Local>` internally. |
| `serde` + `serde_json` | 1.0 | all | JSON serialisation of all output types. |
| `thiserror` | 2.0 | `fitlib` | Derive-macro error types for the library; keeps errors typed without boilerplate. |
| `anyhow` | 1.0 | binaries | Ergonomic `?`-propagation across heterogeneous errors in CLI dispatch code. |
| `clap` (derive) | 4 | binaries | Structured argument parsing; generates `--help`, shell completions, and usage errors automatically. |
| `quick-xml` | 0.37 | `fitlib` | GPX output; future TCX export. Actively maintained. |
| `rayon` | 1.12 | `fitdir` | File-level parallelism when processing directories. |
| `walkdir` | 2 | `fitdir` | Cross-platform recursive directory traversal. |
| `zip` | 2 | `fithistory` | Read Garmin Connect export ZIPs without extracting to disk. |

---

## Roadmap

Items are roughly ordered by implementation dependency.

### Near-term

- [ ] `info` subcommand: richer text/table output (currently outputs raw JSON of the hierarchy struct)
- [ ] `laps` subcommand: formatted table output (currently outputs raw JSON)
- [ ] `select --field` predicate parsing: implement `name=value`, `name>value`, etc.
- [ ] `select --lap` range parsing: implement `--lap 2-5` syntax
- [ ] `validate`: CRC verification once `fitparser` exposes the raw byte stream
- [ ] Named timezone support in `timestamp.rs`: add `chrono-tz` dependency

### Medium-term

- [x] `fitdir survey`: directory walk + rayon parallel dispatch + per-type statistics ✅
- [ ] `fitdir` further subcommands: `info`, `dump`, `validate` batch variants
- [ ] `fithistory`: implement ZIP extraction + per-file processing
- [ ] `zones` subcommand: time-in-zone analysis for HR and power
- [ ] `devices` subcommand: list sensors and devices from `device_info` records
- [ ] `user` subcommand: extract `user_profile` and `zones_target` messages
- [ ] `workouts` subcommand: decode structured workout steps
- [ ] Configuration file (`~/.config/fit2json/config.toml`): default timezone, FTP, HR zones

### Long-term

- [ ] Publish `fitlib` to crates.io as a standalone library
- [ ] TCX export alongside GPX in `gps` subcommand
- [ ] Ramer-Douglas-Peucker GPS simplification for `gps --simplify`
- [ ] Shell completions (`fit2json completions bash/zsh/fish`)
- [ ] `--field` projection in `select` (output only named fields per record)

---

## FIT format stability and versioning strategy

### Threat model

The FIT specification has three distinct layers with very different rates of change:

| Layer | What it covers | Change rate |
|---|---|---|
| **Protocol** | Binary framing, CRC, record headers | Essentially frozen; Garmin has been on protocol 2.x for over a decade. `fitparser` handles this transparently. |
| **Profile** | Message definitions, field names, units, enum values | Additive changes roughly quarterly. New fields appear; existing semantics are stable. |
| **Device/undocumented** | Numeric type codes (44, 58, 68, …) and `unknown_field_N` values | Opaque; can change with any firmware release. |

### Garmin's primary versioning pattern

Profile changes are almost always **additive**: a new higher-resolution variant of an existing field is introduced alongside the original rather than replacing it. The clearest examples observed in real files:

| Legacy field | Enhanced variant | Introduced |
|---|---|---|
| `altitude` | `enhanced_altitude` | Edge/Fenix firmware ≥ 2.x |
| `speed` | `enhanced_speed` | Fenix/Forerunner |
| `avg_speed` | `enhanced_avg_speed` | Session-level summary fields |
| `max_speed` | `enhanced_max_speed` | Session-level summary fields |

Semantically breaking changes — renaming a field, changing its unit, reversing an enum — are extremely rare because Garmin's own Connect platform and thousands of third-party integrations would break simultaneously.

### Why `fields.rs` was created

Before `fields.rs` existed, the `enhanced_*` fallback logic was duplicated in two places using inconsistent patterns: `gps.rs` used an explicit loop over a hardcoded name slice; `info.rs` used inline `.or_else()` chains. When Garmin introduces the next alias, every affected call site would have needed a separate edit, and the risk of missing one was real.

`fields.rs` centralises all alias resolution behind `resolve_field(record, logical_name)`. Adding a new alias is a single `match` arm in one function. Every typed helper (`field_f64`, `field_u32`, `field_altitude`) calls `resolve_field` internally, so all call sites benefit without any further changes.

### What is deliberately not built

A general-purpose version-dispatch system (detect profile version, switch behaviour) was considered and rejected. The complexity would be high and the benefit low because:

1. The core activity structure (session/lap/record hierarchy) has been unchanged for 10+ years.
2. New fields on existing messages appear as `unknown_field_N` — harmlessly ignorable with `--no-unknown`.
3. New message types appear as numeric `MesgNum` values — also harmlessly ignorable.
4. The one pattern that does break code (`enhanced_*` aliases) is handled by `fields.rs` without version detection.

### Handling `fitparser` profile lag

`fitparser` bundles a specific FIT profile snapshot and is updated independently of Garmin firmware releases. A new Garmin device may produce fields that appear as `unknown_field_N` for a few weeks until a `fitparser` update arrives. The appropriate response is:

1. Check whether a newer `fitparser` is available and update `Cargo.toml`.
2. If the field is structurally new (a new `enhanced_*` alias), add it to `fields.rs`.
3. Document the finding in the README quirks section.

`fitparser` is pinned to an exact version (`= 0.10.0`) rather than a semver range so upgrades are deliberate and tested, not silent.

### Non-activity file types

A Garmin Connect bulk export contains far more than workout recordings. Analysis of a 9 182-file sample showed only 3.8% were activity files; the remainder were monitoring, configuration, segment, and HRV files with numeric type codes not in the public profile (see `test-results/file-type-survey-UploadedFiles_0-_Part6.md`).

`fitlib::file_type(data)` performs a fast O(n) scan of the `file_id` record to return the type string. Subcommands that require the activity hierarchy call `require_activity_file` at entry and return a descriptive error for other types. `fitdir` uses `file_type()` as a pre-filter to skip ~96% of files before calling the expensive `build_activity()`.
