use std::path::PathBuf;

use anyhow::Result;
use clap::{Args, ValueEnum};

use crate::cli::GlobalArgs;
use crate::commands::{resolve_input, to_json, write_output};

#[derive(ValueEnum, Clone, Default)]
pub enum GpsFormat {
    #[default]
    Geojson,
    Gpx,
    Json,
}

#[derive(Args)]
pub struct GpsArgs {
    pub input: Option<PathBuf>,
    /// Output format.
    #[arg(long, default_value = "geojson")]
    pub format: GpsFormat,
    /// Additional FIT fields to attach as GeoJSON properties (comma-separated).
    #[arg(long)]
    pub properties: Option<String>,
    /// Print only the bounding box and exit.
    #[arg(long)]
    pub bbox: bool,
    /// Restrict to lap N (1-based).
    #[arg(long)]
    pub lap: Option<usize>,
    /// Include records at or after this time.
    #[arg(long)]
    pub from: Option<String>,
    /// Include records before this time.
    #[arg(long)]
    pub until: Option<String>,
    /// Include records within this many seconds after --from.
    #[arg(short, long)]
    pub duration: Option<u64>,
    /// Apply Ramer-Douglas-Peucker simplification with this epsilon (degrees).
    #[arg(long)]
    pub simplify: Option<f64>,
}

pub fn run(global: &GlobalArgs, args: GpsArgs) -> Result<()> {
    let path = resolve_input(global, &args.input)?;
    let data = fitlib::parse::load_file(&path)?;

    let props: Vec<&str> = args
        .properties
        .as_deref()
        .map(|s| s.split(',').map(str::trim).collect())
        .unwrap_or_default();

    if args.bbox {
        let track = fitlib::gps::extract_track(&data);
        let bbox = fitlib::gps::bounding_box(&track)
            .ok_or_else(|| anyhow::anyhow!("No GPS data found in file"))?;
        let json = to_json(global, &bbox)?;
        return write_output(global, &json);
    }

    match args.format {
        GpsFormat::Geojson | GpsFormat::Json => {
            let geojson = fitlib::gps::to_geojson(&data, &props)?;
            let json = to_json(global, &geojson)?;
            write_output(global, &json)?;
        }
        GpsFormat::Gpx => {
            let gpx = fitlib::gps::to_gpx(&data)?;
            write_output(global, &gpx)?;
        }
    }
    Ok(())
}
