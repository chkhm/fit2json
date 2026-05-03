/// Centralised field-name resolution for known Garmin `enhanced_*` aliases.
///
/// Garmin's primary versioning pattern is adding a higher-resolution
/// `enhanced_*` variant of an existing field on newer firmware.  This module
/// is the **single place to update** when a new alias is discovered — call
/// sites use the logical name and the resolver picks the best physical field
/// available in the record.
///
/// # Adding a new alias
///
/// Add one arm to the `match` inside [`resolve_field`]:
///
/// ```text
/// "cadence" => Some("enhanced_cadence"),
/// ```
///
/// Nothing else needs to change.
use fitparser::{FitDataRecord, Value};

// ---------------------------------------------------------------------------
// Core resolver
// ---------------------------------------------------------------------------

/// Try the `enhanced_*` alias for `logical` first, then the literal name.
///
/// Returns a reference to the raw [`Value`] so callers can perform
/// type-specific extraction (see [`field_f64`], [`field_u32`],
/// [`field_altitude`]).
///
/// # Known aliases
///
/// | Logical name | Tries first        | Then        |
/// |--------------|--------------------|-------------|
/// | `altitude`   | `enhanced_altitude`  | `altitude`  |
/// | `avg_speed`  | `enhanced_avg_speed` | `avg_speed` |
/// | `max_speed`  | `enhanced_max_speed` | `max_speed` |
pub fn resolve_field<'r>(record: &'r FitDataRecord, logical: &str) -> Option<&'r Value> {
    let enhanced = match logical {
        "altitude"  => Some("enhanced_altitude"),
        "avg_speed" => Some("enhanced_avg_speed"),
        "max_speed" => Some("enhanced_max_speed"),
        _           => None,
    };

    if let Some(enh) = enhanced
        && let Some(f) = record.fields().iter().find(|f| f.name() == enh)
    {
        return Some(f.value());
    }

    record.fields().iter().find(|f| f.name() == logical).map(|f| f.value())
}

// ---------------------------------------------------------------------------
// Typed extraction helpers
// ---------------------------------------------------------------------------

/// Extract a field as `f64` using [`resolve_field`] for name resolution.
///
/// Accepts `Float64`, `Float32`, `UInt32`, and `UInt16` values; returns
/// `None` for all other variants (including timestamps and enums).
pub fn field_f64(record: &FitDataRecord, logical: &str) -> Option<f64> {
    match resolve_field(record, logical)? {
        Value::Float64(v) => Some(*v),
        Value::Float32(v) => Some(f64::from(*v)),
        Value::UInt32(v)  => Some(*v as f64),
        Value::UInt16(v)  => Some(*v as f64),
        _                 => None,
    }
}

/// Extract a field as `u32` using [`resolve_field`] for name resolution.
///
/// Accepts `UInt32`, `UInt32z`, `UInt16`, and `UInt8` values.
pub fn field_u32(record: &FitDataRecord, logical: &str) -> Option<u32> {
    match resolve_field(record, logical)? {
        Value::UInt32(v)  => Some(*v),
        Value::UInt32z(v) => Some(*v),
        Value::UInt16(v)  => Some(*v as u32),
        Value::UInt8(v)   => Some(*v as u32),
        _                 => None,
    }
}

/// Extract altitude in metres, preferring `enhanced_altitude` over `altitude`.
///
/// Handles three encoding cases:
/// - `Float64` / `Float32` — already in metres (fitparser has applied
///   the FIT profile scale/offset).
/// - `UInt16` / `UInt32` — raw unscaled integer; applies the FIT profile
///   rule: `metres = raw / 5 − 500`.
pub fn field_altitude(record: &FitDataRecord) -> Option<f64> {
    match resolve_field(record, "altitude")? {
        Value::Float64(v) => Some(*v),
        Value::Float32(v) => Some(f64::from(*v)),
        Value::UInt16(v)  => Some(*v as f64 / 5.0 - 500.0),
        Value::UInt32(v)  => Some(*v as f64 / 5.0 - 500.0),
        _                 => None,
    }
}
