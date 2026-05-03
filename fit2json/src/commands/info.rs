use std::path::PathBuf;

use anyhow::Result;
use clap::Args;
use fitparser::profile::field_types::MesgNum;
use serde::Serialize;

use crate::cli::GlobalArgs;
use crate::commands::{require_activity_file, resolve_input, to_json, write_output};

#[derive(Args)]
pub struct InfoArgs {
    pub input: Option<PathBuf>,
    #[arg(long, default_value = "json")]
    pub format: super::types::OutputFormat,
}

// ---------------------------------------------------------------------------
// Summary output type
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
struct ActivitySummary {
    sport: Option<String>,
    sub_sport: Option<String>,
    start_time: Option<String>,
    total_elapsed_s: Option<f64>,
    total_timer_s: Option<f64>,
    total_distance_m: Option<f64>,
    total_ascent_m: Option<u32>,
    total_descent_m: Option<u32>,
    total_calories: Option<u32>,
    avg_heart_rate: Option<u32>,
    max_heart_rate: Option<u32>,
    avg_power_w: Option<u32>,
    max_power_w: Option<u32>,
    normalized_power_w: Option<u32>,
    avg_cadence_rpm: Option<u32>,
    max_cadence_rpm: Option<u32>,
    avg_speed_ms: Option<f64>,
    max_speed_ms: Option<f64>,
    num_laps: Option<u32>,
    num_sessions: usize,
    manufacturer: Option<String>,
    serial_number: Option<u32>,
}

// ---------------------------------------------------------------------------
// Run
// ---------------------------------------------------------------------------

pub fn run(global: &GlobalArgs, args: InfoArgs) -> Result<()> {
    let path = resolve_input(global, &args.input)?;
    let data = fitlib::parse::load_file(&path)?;
    require_activity_file(&data, "info")?;
    let activity = fitlib::hierarchy::build_activity(&data)?;

    // Pull the first session record directly for its pre-computed summary fields.
    let session_rec = data
        .iter()
        .find(|r| r.kind() == MesgNum::Session);

    let summary = ActivitySummary {
        sport:              activity.sessions.first().and_then(|s| s.sport.clone()),
        sub_sport:          activity.sessions.first().and_then(|s| s.sub_sport.clone()),
        start_time:         activity.sessions.first()
                                .and_then(|s| s.start_time)
                                .map(|t| t.to_rfc3339()),
        total_elapsed_s:    session_rec.and_then(|r| fitlib::fields::field_f64(r, "total_elapsed_time")),
        total_timer_s:      session_rec.and_then(|r| fitlib::fields::field_f64(r, "total_timer_time")),
        total_distance_m:   session_rec.and_then(|r| fitlib::fields::field_f64(r, "total_distance")),
        total_ascent_m:     session_rec.and_then(|r| fitlib::fields::field_u32(r, "total_ascent")),
        total_descent_m:    session_rec.and_then(|r| fitlib::fields::field_u32(r, "total_descent")),
        total_calories:     session_rec.and_then(|r| fitlib::fields::field_u32(r, "total_calories")),
        avg_heart_rate:     session_rec.and_then(|r| fitlib::fields::field_u32(r, "avg_heart_rate")),
        max_heart_rate:     session_rec.and_then(|r| fitlib::fields::field_u32(r, "max_heart_rate")),
        avg_power_w:        session_rec.and_then(|r| fitlib::fields::field_u32(r, "avg_power")),
        max_power_w:        session_rec.and_then(|r| fitlib::fields::field_u32(r, "max_power")),
        normalized_power_w: session_rec.and_then(|r| fitlib::fields::field_u32(r, "normalized_power")),
        avg_cadence_rpm:    session_rec.and_then(|r| fitlib::fields::field_u32(r, "avg_cadence")),
        max_cadence_rpm:    session_rec.and_then(|r| fitlib::fields::field_u32(r, "max_cadence")),
        avg_speed_ms:       session_rec.and_then(|r| fitlib::fields::field_f64(r, "avg_speed")),
        max_speed_ms:       session_rec.and_then(|r| fitlib::fields::field_f64(r, "max_speed")),
        num_laps:           session_rec.and_then(|r| fitlib::fields::field_u32(r, "num_laps")),
        num_sessions:       activity.num_sessions,
        manufacturer:       activity.file_id.manufacturer.clone(),
        serial_number:      activity.file_id.serial_number,
    };

    let json = to_json(global, &summary)?;
    write_output(global, &json)
}

