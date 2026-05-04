# garmin-fit tools

A collection of fast, composable command-line tools for reading and analysing Garmin FIT activity files, written in Rust.

> For a full description of the software architecture, module layout, and design decisions see **[DESIGN.md](DESIGN.md)**.  
> For the full requirements catalogue see **[REQUIREMENTS.md](REQUIREMENTS.md)**.

---

## Tools

| Binary       | Status      | Purpose                                                        |
|--------------|-------------|----------------------------------------------------------------|
| `fit2json`   | ✅ Complete  | Query, filter, and extract data from a single FIT file         |
| `fitdir`     | 🔧 Partial  | Batch-process all FIT files in a directory                     |
| `fithistory` | 📋 Planned  | Unpack and ingest a Garmin Connect bulk-export ZIP             |

---

## Building

Requires Rust 1.85 or later (edition 2024).

```sh
git clone <repo-url>
cd fit2json
cargo build --release
```

The release binaries are written to `target/release/`.  To install them to your Cargo bin directory:

```sh
cargo install --path fit2json    # installs fit2json
cargo install --path fitdir      # installs fitdir
```

---

## fit2json — quick start

```
fit2json <subcommand> [options] <file.fit>
```

### See what is in a file

```sh
# Report the FIT file type (activity, workout, course, …)
fit2json filetype file.fit

# … with device and creation-time metadata
fit2json filetype file.fit --format json

# Count of every message type present
fit2json types activity.fit

# Human-readable activity summary (sessions, duration, sport)
fit2json info activity.fit

# Validate structural integrity
fit2json validate activity.fit
```

### Extract data

```sh
# Dump all records to JSON (stdout, compact)
fit2json dump activity.fit

# Dump to a pretty-printed file
fit2json dump activity.fit -o activity.json

# Split into one JSON file per message type
fit2json dump activity.fit --split --output-dir ./exported/

# Select all record messages
fit2json select activity.fit --type record

# Select records within a time window
fit2json select activity.fit --type record --from 2026-04-23T09:58:43 --duration 300

# Select only the first 10 event messages
fit2json select activity.fit --type event --limit 10

# Count matching records without printing them
fit2json select activity.fit --type record --count
```

### GPS track

```sh
# Export full GPS track as GeoJSON (stdout)
fit2json gps activity.fit

# Export to a file with altitude and heart rate as point properties
fit2json gps activity.fit --format geojson --properties heart_rate,altitude -o track.geojson

# Export as GPX
fit2json gps activity.fit --format gpx -o track.gpx

# Print only the bounding box
fit2json gps activity.fit --bbox
```

### Statistics and laps

```sh
# Aggregated statistics for the whole activity
fit2json stats activity.fit

# Per-lap statistics (min/max/mean for all numeric fields)
fit2json stats activity.fit --by lap

# Per-lap stats for specific fields
fit2json stats activity.fit --by lap --fields heart_rate,power,speed

# Lap summary table
fit2json laps activity.fit

# Session overview (especially useful for triathlon/multi-sport files)
fit2json sessions activity.fit
```

### Events

```sh
# Full event log
fit2json events activity.fit

# Only timer events (start, stop, pause, resume)
fit2json events activity.fit --type timer

# Lap triggers and workout-step transitions
fit2json events activity.fit --type lap,workout_step
```

### Multi-sport activities (triathlon, duathlon)

```sh
# List all sessions (e.g. Swim, T1, Bike, T2, Run)
fit2json sessions triathlon.fit

# Lap table for the bike leg only (session 3)
fit2json laps triathlon.fit --session 3

# GPS track for the run leg
fit2json gps triathlon.fit --session 5 --format geojson -o run.geojson
```

### Compare two files

```sh
fit2json compare monday.fit tuesday.fit
fit2json compare monday.fit tuesday.fit --fields heart_rate,power,speed
```

---

## fitdir — quick start

```
fitdir <subcommand> [options]
```

`fitdir` operates on a whole directory of FIT files at once, using parallel processing (rayon) to keep batch jobs fast.

### survey — directory overview

Scan a folder and report per-type statistics: file count, size distribution, record count distribution, and recording date range.

```sh
# Survey the current directory (non-recursive, table output)
fitdir survey

# Survey a specific folder
fitdir survey --dir ~/Garmin/Activities/

# Include all subdirectories
fitdir survey --dir ~/Garmin/ --recursive

# JSON output (raw bytes, suitable for jq / further processing)
fitdir survey --dir ~/Garmin/Activities/ --format json

# Save JSON output to a file (pretty-printed by default)
fitdir survey --dir ~/Garmin/ --recursive --format json -o survey.json

# Limit parallelism (default: all logical CPUs)
fitdir survey --dir ~/Garmin/ --jobs 4
```

**Example table output** (Garmin Connect bulk export, 9 182 files):

```
File Type      Files  Size  min / avg / median / max     Records  min / avg / median / max     Date range
─────────────────────────────────────────────────────────────────────────────────────────────────────────────
44              2712   0K / 0K / 0K / 1K                 4 / 10 / 6 / 23               2025-02-01 – 2026-04-23
monitoring_b    2650   1K / 10K / 2K / 93K         10 / 1103 / 184 / 8446              2024-09-21 – 2026-04-23
58              2318   0K / 4K / 1K / 21K               5 / 289 / 62 / 1445            2024-09-21 – 2026-04-23
segment_list     392   8K / 13K / 13K / 16K              37 / 58 / 59 / 72             2025-02-04 – 2026-04-23
activity         353   7K / 157K / 136K / 654K      72 / 8905 / 7354 / 61124           2024-11-08 – 2026-04-23
…
```

Only ~3.8% of a typical Garmin Connect export are activity files — `survey` lets you understand what the rest actually is before processing. See [Non-activity file types](#non-activity-file-types-found-in-garmin-connect-bulk-exports) for a breakdown of the known types.

### list — per-file listing

Enumerate individual files with optional type filtering, configurable sort order, and a row limit.

```sh
# List all FIT files in the current directory, sorted by recording date
fitdir list

# List only activity files
fitdir list --dir ~/Garmin/ --type activity

# 10 most recent activity files
fitdir list --dir ~/Garmin/ --type activity --sort date --desc --limit 10

# 10 largest activity files
fitdir list --dir ~/Garmin/ --type activity --sort size --desc --limit 10

# List multiple types
fitdir list --dir ~/Garmin/ --type activity --type monitoring_b

# JSON output for downstream processing
fitdir list --dir ~/Garmin/ --type activity --format json | jq '.[].path'

# Write JSON to a file (pretty-printed automatically)
fitdir list --dir ~/Garmin/ --recursive --format json -o files.json
```

**Example table output:**

```
    #  Date        Type          Size     Records  File
────────────────────────────────────────────────────────────────────────────────────
    1  2024-11-08  activity      136K        7354  /Garmin/Activities/2024-11-08.fit
    2  2024-11-15  activity      148K        8201  /Garmin/Activities/2024-11-15.fit
    3  2024-11-22  activity       92K        5103  /Garmin/Activities/2024-11-22.fit
    4  —           unknown         2K          14  /Garmin/Activities/orphan.fit
```

Date column shows `YYYY-MM-DD`; `—` when `time_created` is absent.  Files without a date always sort last, regardless of `--desc`.

### fitdir global options

| Flag | Short | Description |
|------|-------|-------------|
| `--output <file>` | `-o` | Write output to a file instead of stdout. |
| `--pretty` | | Pretty-print JSON (default when `--output` is set). |
| `--compact` | | Compact single-line JSON (default when writing to stdout). |
| `--help` | `-h` | Print help for the current subcommand. |
| `--version` | `-V` | Print the tool version. |

### survey options

| Flag | Short | Default | Description |
|------|-------|---------|-------------|
| `--dir <path>` | `-d` | `.` | Directory to scan. |
| `--recursive` | `-r` | off | Recurse into subdirectories. |
| `--jobs <n>` | `-j` | all CPUs | Number of parallel worker threads. |
| `--format <fmt>` | | `table` | Output format: `table` or `json`. |

### list options

| Flag | Short | Default | Description |
|------|-------|---------|-------------|
| `--dir <path>` | `-d` | `.` | Directory to scan. |
| `--recursive` | `-r` | off | Recurse into subdirectories. |
| `--jobs <n>` | `-j` | all CPUs | Number of parallel worker threads. |
| `--type <type>` | `-t` | (all) | Keep only files of this type. Repeatable. |
| `--sort <field>` | | `date` | Sort key: `date`, `size`, `records`, or `name`. |
| `--desc` | | off | Reverse sort order. |
| `--limit <n>` | `-n` | (none) | Return at most N results. |
| `--format <fmt>` | | `table` | Output format: `table` or `json`. |

---

## Global options (`fit2json`)

These flags work with every subcommand.

| Flag | Short | Description |
|---|---|---|
| `--input <file>` | `-i` | Input FIT file (alternative to the positional argument). |
| `--output <file>` | `-o` | Write output to a file instead of stdout. |
| `--pretty` | | Pretty-print JSON (default when `--output` is set). |
| `--compact` | | Compact single-line JSON (default when writing to stdout). |
| `--utc` | | Display all timestamps in UTC. |
| `--timezone <tz>` | | Display timestamps in an IANA timezone (e.g. `Europe/Berlin`). |
| `--no-unknown` | | Suppress unknown and Connect IQ developer-defined fields. |
| `--help` | `-h` | Print help for the current subcommand. |
| `--version` | `-V` | Print the tool version and the FIT profile version in use. |

---

## Subcommand reference

Commands marked **activity only** return an error when run against a non-activity FIT file
(e.g. workout, course, monitoring).  Use `filetype` to check first.

| Subcommand | Works on | Description |
|---|---|---|
| `filetype` | all files | Report the FIT file type from the `file_id` record (`activity`, `workout`, `course`, …). |
| `dump` | all files | Full JSON extraction of all records. Supports `--split` for one file per message type. |
| `types` | all files | Table of all message types and their record counts. |
| `select` | all files | General-purpose record filter: by type, time range, session, lap, and field predicates. |
| `gps` | all files | GPS track extraction as GeoJSON, GPX, or JSON bounding box. |
| `events` | all files | Event log with optional filtering by event type. |
| `stats` | all files¹ | Numeric aggregation (min/max/mean/sum) at activity, session, or lap granularity. |
| `validate` | all files | Structural integrity checks; activity-specific rules skipped for non-activity files. |
| `info` | **activity only** | High-level summary: device, sport, duration, distance, HR, power, GPS bbox. |
| `sessions` | **activity only** | Per-session summary table (sport, start time, duration, distance). |
| `laps` | **activity only** | Per-lap summary table, optionally restricted to one session. |
| `compare` | all files | Side-by-side statistics comparison of two FIT files. |

¹ `stats --by session` and `stats --by lap` require an activity file; `--by activity` works on any file.

For the full flag reference for each subcommand, run:

```sh
fit2json <subcommand> --help
```

---

## Example workflow: feeding data into a database

```sh
# Build one JSON summary row per activity
fit2json info activity.fit --format json \
  | jq '{sport, start, duration_s, distance_m, avg_hr, avg_power}'
```

```sh
# Export per-lap data for all laps as newline-delimited JSON
fit2json stats activity.fit --by lap --format json \
  | jq -c '.[]'
```

---

## FIT file quirks by sport and device

This section collects device- and sport-specific quirks discovered while testing real Garmin FIT files.  It will grow as more file types are added.

### General

- **`enhanced_speed` vs `speed`**: Newer Garmin firmware (Edge, Fenix, Forerunner) stores speed as `enhanced_speed` / `enhanced_avg_speed` / `enhanced_max_speed` rather than `speed`.  Use these field names with `--fields` on `stats` and `select`.
- **`enhanced_altitude` vs `altitude`**: Similarly, altitude on current firmware appears as `enhanced_altitude` (already converted to metres by fitparser).  The legacy `altitude` field may be absent on newer files.
- **Unknown fields**: Developer-defined or undocumented fields appear as `unknown_field_N`.  Use `--no-unknown` to suppress them in JSON output.
- **`position_lat` / `position_long`**: These are raw **semicircle** integers in the flat record stream; the GPS subcommand converts them automatically.  Do not use them with `--fields` on `stats` unless you want semicircle arithmetic.

### Swimming (`sport=swimming`, `sub_sport=lap_swimming`)

- **`timestamp` = `start_time`**: The `timestamp` field on every lap and session record equals the session `start_time`.  End times must be derived from `start_time + total_elapsed_time`.  The tool handles this automatically; be aware of it if you inspect raw FIT records directly.
- **No GPS data**: Pool swim files contain no position records.  `gps --bbox` and `gps --format geojson/gpx` will report "no GPS data" — this is expected.
- **No altitude**: Swimming files have no `altitude` or `enhanced_altitude` fields; the `min_alt`/`max_alt` bbox fields will be `null`.
- **Lap structure**: A pool swim session typically has many short laps (one per length or per interval).  The 65-lap file tested here encodes 141 individual lengths distributed across 65 interval laps.
- **Cadence unit**: For swimming, `avg_cadence` represents strokes per minute (SPM), not pedal RPM.

### Indoor cycling (`sport=cycling`, `sub_sport=indoor_cycling`)

- **`timestamp` = `start_time`**: Same as swimming — the `timestamp` on session and lap records equals the activity start time; end times require `start_time + total_elapsed_time`.
- **No GPS data**: Indoor trainer files contain no position records; GPS subcommands will report "no GPS data" — expected.
- **No altitude, ascent, or descent**: `total_ascent`, `total_descent`, and altitude fields are absent.  The corresponding `info` fields will be `null`.
- **Virtual distance**: `total_distance` is present but reflects trainer-computed distance, not GPS ground distance.

### Outdoor cycling (`sport=cycling`, `sub_sport=road` or `generic`)

- **`timestamp` ≠ `start_time`**: The session `timestamp` field holds the session end time (distinct from `start_time`), so time-window filtering and lap assignment work via the standard FIT mechanism.
- **Altitude scale**: `enhanced_altitude` values from `fitparser` are already in metres with scale and offset applied.  Raw `altitude` fields, if present, may appear as `UInt16` integers requiring manual conversion (`raw / 5 − 500`).
- **Power fields**: `avg_power`, `max_power`, and `normalized_power` are present on power-meter files.  `training_stress_score`, `intensity_factor`, and `threshold_power` appear when a power zone is configured on the device.

### Non-activity file types found in Garmin Connect bulk exports

A Garmin Connect bulk-export ZIP contains far more than just activity recordings.  In the
`UploadedFiles_0-_Part6` sample of 9 182 files only **353 (~3.8%) were activity files**; the
remainder are device-management and health-monitoring files.  `fitparser` returns numeric strings
for type codes that are not in the public FIT profile.

| `filetype` output | FIT type code | Approx. count | Key message types | Notes |
|-------------------|---------------|---------------|-------------------|-------||
| `activity` | 4 | 353 | `session`, `lap`, `record`, … | Standard workout recordings; the only type supported by `info`, `sessions`, `laps`. |
| `monitoring_b` | 32 | 2 650 | `monitoring`, `monitoring_hr_data`, `monitoring_info`, `stress_level`, `respiration_rate` | Continuous health monitoring (HR, stress, respiration).  Stored in circular buffers; one file covers roughly one day. |
| `segment_list` | 35 | 392 | `segment_file` (×57) | Directory of on-device segments (Strava segments etc.).  Contains no time-series data. |
| `44` | undocumented | 2 712 | `device_info`, `file_creator`, undoc 241, 410 | Tiny device-sync / configuration record (~180–1 069 B).  No useful user data. |
| `58` | undocumented | 2 318 | `software`, `timestamp_correlation`, `device_info`, undoc 318 | Device software / firmware metadata.  Contains `software_version` fields. |
| `68` | undocumented | 168 | **`hrv_value`** (×79 per file), `timestamp_correlation` | **HRV (Heart Rate Variability) recordings.**  Each file holds ~79 `hrv_value` records from an overnight or on-demand HRV measurement.  A likely candidate for future extraction. |
| `57` | undocumented | 18 | **`met_zone`** (×22), **`speed_zone`** (×11), undoc 14, 16, 71 | **Sport profile / training-zone configuration.**  Stores MET zones and speed/pace zones as configured on the device. |
| `49` | undocumented | 274 | `device_info`, `file_creator`, undoc 412 | Tiny device-config record (~360–980 B).  No useful user data. |
| `41` | undocumented | 183 | `file_creator`, undoc 217 (×13) | Tiny file (~180–1 850 B).  Possibly goals or app configuration. |
| `79` | undocumented | 114 | `file_creator`, undoc 470, 471 | Tiny uniform file (exactly 372 B each).  Purpose unknown. |

**Practical implications for `fitdir`**: call `fitlib::file_type()` on each file first (O(n) scan of
the `file_id` record) and skip any file whose type is not `"activity"` before calling the expensive
`build_activity()`.  This avoids processing ~96% of the files in a typical export.

---

## License

MIT — see [LICENSE](LICENSE).
