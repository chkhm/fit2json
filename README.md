# garmin-fit tools

A collection of fast, composable command-line tools for reading and analysing Garmin FIT activity files, written in Rust.

> For a full description of the software architecture, module layout, and design decisions see **[DESIGN.md](DESIGN.md)**.  
> For the full requirements catalogue see **[REQUIREMENTS.md](REQUIREMENTS.md)**.

---

## Tools

| Binary | Purpose |
|---|---|
| `fit2json` | Query, filter, and extract data from a single FIT file |
| `fitdir` | Batch-process all FIT files in a directory *(planned)* |
| `fithistory` | Unpack and ingest a Garmin Connect bulk-export ZIP *(planned)* |

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
```

---

## fit2json — quick start

```
fit2json <subcommand> [options] <file.fit>
```

### See what is in a file

```sh
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

## Global options

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

| Subcommand | Description |
|---|---|
| `dump` | Full JSON extraction of all records. Supports `--split` for one file per message type. |
| `info` | High-level summary: device, sport, duration, distance, HR, power, GPS bbox. |
| `types` | Table of all message types and their record counts. |
| `select` | General-purpose record filter: by type, time range, session, lap, and field predicates. |
| `stats` | Numeric aggregation (min/max/mean/sum) at activity, session, or lap granularity. |
| `gps` | GPS track extraction as GeoJSON, GPX, or JSON bounding box. |
| `events` | Event log with optional filtering by event type. |
| `sessions` | Per-session summary table (sport, start time, duration, distance). |
| `laps` | Per-lap summary table, optionally restricted to one session. |
| `validate` | Structural integrity checks: required messages, session counts, timestamp ordering. |
| `compare` | Side-by-side statistics comparison of two FIT files. |

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

## License

MIT — see [LICENSE](LICENSE).
