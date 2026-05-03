# Measuring execution time, memory, and CPU utilisation

## Time (built in)

The `time` builtin gives wall-clock, user CPU, and system CPU:

```sh
time cargo run -p fitdir --release -- survey --dir test-data/UploadedFiles_0-_Part6/
```

For repeated runs without recompilation:

```sh
time ./target/release/fitdir survey --dir test-data/UploadedFiles_0-_Part6/
```

Example output: `21.32s user 1.49s system 82% cpu 27.658 total`

---

## Memory + CPU together — `/usr/bin/time -l`

macOS ships a more capable `/usr/bin/time` (distinct from the shell builtin) that reports peak RSS:

```sh
/usr/bin/time -l ./target/release/fitdir survey --dir test-data/UploadedFiles_0-_Part6/ 2>&1 | grep -E "real|maximum resident"
```

Key output lines:
```
       27.42 real        21.18 user         1.47 sys
  123456789  maximum resident set size    ← peak RAM in bytes (÷ 1048576 = MB)
```

---

## Continuous CPU + memory — `top` or `htop`

Run the survey in one terminal, watch it in another:

```sh
# Terminal 1 — run the binary (takes ~28 s, plenty of time to observe)
./target/release/fitdir survey --dir test-data/UploadedFiles_0-_Part6/

# Terminal 2 — watch it
top -pid $(pgrep fitdir)
# or, if you have htop installed:
htop -p $(pgrep fitdir)
```

---

## Detailed profiling — Instruments (macOS native, free)

Instruments is the most accurate tool for seeing per-thread CPU, allocation timeline, and flame graphs:

```sh
# Record a CPU profile (opens Instruments.app with results when done)
xcrun xctrace record --template "Time Profiler" \
  --launch -- ./target/release/fitdir survey --dir test-data/UploadedFiles_0-_Part6/
```

Or launch Instruments.app from Xcode → Open Developer Tool → Instruments, pick **Time Profiler** or **Allocations**, then drag your binary in.

---

## Sampling flame graph — `samply` (lightweight, Rust-friendly)

```sh
cargo install samply
samply record ./target/release/fitdir survey --dir test-data/UploadedFiles_0-_Part6/
# Opens a Firefox Profiler flame graph in your browser automatically
```

This is the fastest way to see *where* the time is actually going (parsing vs. I/O vs. summarisation).

---

## Quick reference

| Goal | Command |
|------|---------|
| Wall / user / sys time | `time ./target/release/fitdir survey …` |
| Peak RAM | `/usr/bin/time -l ./target/release/fitdir survey …` |
| Live CPU + RSS | `top -pid $(pgrep fitdir)` |
| Flame graph | `samply record ./target/release/fitdir survey …` |
| Full profiling | Instruments → Time Profiler |

For a typical first pass, `/usr/bin/time -l` is the most convenient — one command gives wall time, CPU breakdown, and peak memory with no extra tools required.
