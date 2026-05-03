/// GPS track extraction and output formatting.
///
/// Garmin devices store latitude and longitude as **semicircles** (i32 values).
/// Conversion to degrees: `degrees = value * (180.0 / 2^31)`
use fitparser::profile::field_types::MesgNum;
use fitparser::{FitDataRecord, Value};
use serde::Serialize;

use crate::FitError;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, Serialize)]
pub struct LatLon {
    pub lat: f64,
    pub lon: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct GpsPoint {
    pub position: LatLon,
    pub altitude_m: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BoundingBox {
    pub min_lat: f64,
    pub max_lat: f64,
    pub min_lon: f64,
    pub max_lon: f64,
    pub min_alt: Option<f64>,
    pub max_alt: Option<f64>,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Extract an ordered GPS track from `record` messages in `data`.
///
/// Records without valid position data are silently skipped.
pub fn extract_track(data: &[FitDataRecord]) -> Vec<GpsPoint> {
    data.iter()
        .filter(|r| r.kind() == MesgNum::Record)
        .filter_map(|r| {
            let lat = field_semicircles(r, "position_lat")?;
            let lon = field_semicircles(r, "position_long")?;
            let altitude_m = field_altitude(r);
            Some(GpsPoint {
                position: LatLon { lat, lon },
                altitude_m,
            })
        })
        .collect()
}

/// Compute the bounding box of a GPS track.
///
/// Returns `None` if the track is empty.
pub fn bounding_box(track: &[GpsPoint]) -> Option<BoundingBox> {
    if track.is_empty() {
        return None;
    }
    let mut min_lat = f64::MAX;
    let mut max_lat = f64::MIN;
    let mut min_lon = f64::MAX;
    let mut max_lon = f64::MIN;
    let mut min_alt: Option<f64> = None;
    let mut max_alt: Option<f64> = None;

    for pt in track {
        min_lat = min_lat.min(pt.position.lat);
        max_lat = max_lat.max(pt.position.lat);
        min_lon = min_lon.min(pt.position.lon);
        max_lon = max_lon.max(pt.position.lon);
        if let Some(a) = pt.altitude_m {
            min_alt = Some(min_alt.map_or(a, |m: f64| m.min(a)));
            max_alt = Some(max_alt.map_or(a, |m: f64| m.max(a)));
        }
    }

    Some(BoundingBox { min_lat, max_lat, min_lon, max_lon, min_alt, max_alt })
}

/// Serialize the GPS track from `data` as a GeoJSON `FeatureCollection`
/// containing a single `LineString` feature.
///
/// `properties_filter`: additional FIT field names to attach as properties
/// on each coordinate (e.g. `["heart_rate", "power", "altitude"]`).
/// Pass an empty slice for a plain coordinate-only LineString.
pub fn to_geojson(
    data: &[FitDataRecord],
    _properties_filter: &[&str],
) -> Result<serde_json::Value, FitError> {
    let track = extract_track(data);
    if track.is_empty() {
        return Err(FitError::NoGpsData);
    }

    // Build coordinate array: [lon, lat] or [lon, lat, alt] per GeoJSON spec.
    let coords: Vec<serde_json::Value> = track
        .iter()
        .map(|pt| match pt.altitude_m {
            Some(alt) => serde_json::json!([pt.position.lon, pt.position.lat, alt]),
            None      => serde_json::json!([pt.position.lon, pt.position.lat]),
        })
        .collect();

    let feature = serde_json::json!({
        "type": "FeatureCollection",
        "features": [{
            "type": "Feature",
            "geometry": {
                "type": "LineString",
                "coordinates": coords
            },
            "properties": {}
        }]
    });

    Ok(feature)
}

/// Serialize the GPS track from `data` as a GPX 1.1 XML string.
pub fn to_gpx(data: &[FitDataRecord]) -> Result<String, FitError> {
    use quick_xml::Writer;
    use std::io::Cursor;

    let track = extract_track(data);
    if track.is_empty() {
        return Err(FitError::NoGpsData);
    }

    let mut writer = Writer::new_with_indent(Cursor::new(Vec::new()), b' ', 2);

    use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};

    writer.write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))?;

    let mut gpx_start = BytesStart::new("gpx");
    gpx_start.push_attribute(("version", "1.1"));
    gpx_start.push_attribute(("creator", "fit2json"));
    gpx_start.push_attribute(("xmlns", "http://www.topografix.com/GPX/1/1"));
    writer.write_event(Event::Start(gpx_start))?;

    writer.write_event(Event::Start(BytesStart::new("trk")))?;
    writer.write_event(Event::Start(BytesStart::new("trkseg")))?;

    for pt in &track {
        let mut trkpt = BytesStart::new("trkpt");
        trkpt.push_attribute(("lat", pt.position.lat.to_string().as_str()));
        trkpt.push_attribute(("lon", pt.position.lon.to_string().as_str()));
        writer.write_event(Event::Start(trkpt))?;
        if let Some(alt) = pt.altitude_m {
            writer.write_event(Event::Start(BytesStart::new("ele")))?;
            writer.write_event(Event::Text(BytesText::new(&alt.to_string())))?;
            writer.write_event(Event::End(BytesEnd::new("ele")))?;
        }
        writer.write_event(Event::End(BytesEnd::new("trkpt")))?;
    }

    writer.write_event(Event::End(BytesEnd::new("trkseg")))?;
    writer.write_event(Event::End(BytesEnd::new("trk")))?;
    writer.write_event(Event::End(BytesEnd::new("gpx")))?;

    let bytes = writer.into_inner().into_inner();
    Ok(String::from_utf8(bytes).unwrap_or_default())
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

const SEMICIRCLES_TO_DEGREES: f64 = 180.0 / (1u64 << 31) as f64;

fn field_semicircles(record: &FitDataRecord, name: &str) -> Option<f64> {
    record.fields().iter().find(|f| f.name() == name).and_then(|f| {
        match f.value() {
            Value::SInt32(v) => Some(*v as f64 * SEMICIRCLES_TO_DEGREES),
            _ => None,
        }
    })
}

fn field_altitude(record: &FitDataRecord) -> Option<f64> {
    // fitparser decodes the scale/offset for well-known fields and produces a
    // Float32 or Float64.  When parsing without full profile decoding (or for
    // unknown sub-protocols) the raw UInt16 may be returned instead; in that
    // case apply the FIT profile rule manually: metres = raw/5 − 500.
    //
    // Prefer `enhanced_altitude` (higher resolution, present on Edge/Fenix
    // firmware ≥ 2.x) then fall back to the legacy `altitude` field.
    for name in &["enhanced_altitude", "altitude"] {
        if let Some(field) = record.fields().iter().find(|f| f.name() == *name) {
            let v = match field.value() {
                Value::Float64(v) => Some(*v),
                Value::Float32(v) => Some(f64::from(*v)),
                // Raw unscaled integer: apply FIT profile scale=5 offset=500.
                Value::UInt16(v)  => Some(*v as f64 / 5.0 - 500.0),
                Value::UInt32(v)  => Some(*v as f64 / 5.0 - 500.0),
                _ => None,
            };
            if v.is_some() {
                return v;
            }
        }
    }
    None
}

impl From<quick_xml::Error> for FitError {
    fn from(e: quick_xml::Error) -> Self {
        FitError::IntegrityFailure(e.to_string())
    }
}
