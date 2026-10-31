//! The shared observing session and the console's injectable clock.

use std::fmt;
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Deserializer, Serialize};

use crate::sky::{self, Frame, Observer};
use crate::targets::{self, Target};

/// The console's time source, in Unix seconds. Each application owns its clock.
pub trait Clock: Send + Sync {
    /// Read the console's time.
    fn now(&self) -> f64;
}

/// The real system clock, used by the binary and by default in the library.
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> f64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or_else(|e| -e.duration().as_secs_f64(), |d| d.as_secs_f64())
    }
}

/// A fixed Unix time for tests and simulations. No binary option constructs one.
pub struct FixedClock(pub f64);

impl Clock for FixedClock {
    fn now(&self) -> f64 {
        self.0
    }
}

/// Where the telescope stands, in degrees (north and east positive).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Site {
    /// Latitude within ±90°.
    pub lat_deg: f64,
    /// Longitude within ±180°.
    pub lon_deg: f64,
}

/// A checked partial edit. An omitted field is retained; null clears site or target.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionEdit {
    /// A replacement site, or explicit null to clear it.
    #[serde(default, deserialize_with = "present_nullable")]
    pub site: Option<Option<Site>>,
    /// A replacement minimum altitude within 0–80°; null is invalid.
    #[serde(default, deserialize_with = "present_number")]
    pub min_alt_deg: Option<f64>,
    /// A catalog id, or explicit null to clear the target.
    #[serde(default, deserialize_with = "present_nullable")]
    pub target: Option<Option<String>>,
}

fn present_nullable<'de, D, T>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer).map(Some)
}

fn present_number<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<f64>, D::Error> {
    f64::deserialize(deserializer).map(Some)
}

/// The most recent solved frame's pointing, held at its frame time.
#[derive(Debug, Clone, Serialize)]
pub struct Pointing {
    /// Monotonically increasing within this observing session, starting at one.
    pub seq: u64,
    /// The solved projection, retained for placing targets on the frame.
    pub frame: Frame,
    /// J2000 right ascension at the aim point (the frame centre until one is set).
    pub ra_deg: f64,
    /// J2000 declination at the aim point, in degrees.
    pub dec_deg: f64,
    /// Geometric altitude at frame time when a site is set, in degrees.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alt_deg: Option<f64>,
    /// Azimuth at frame time when a site is set, north through east, in degrees.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub az_deg: Option<f64>,
    /// Frame time on the console's clock, in Unix seconds.
    pub time: f64,
}

impl Pointing {
    fn with_site(&mut self, site: Option<Site>) {
        let horizontal = site.map(|site| {
            sky::alt_az(
                self.ra_deg,
                self.dec_deg,
                &Observer {
                    lat: site.lat_deg,
                    lon: site.lon_deg,
                    time: self.time,
                },
            )
        });
        self.alt_deg = horizontal.map(|(alt, _)| alt);
        self.az_deg = horizontal.map(|(_, az)| az);
    }
}

/// One consistent read of the observing session and its console time.
#[derive(Debug, Serialize)]
pub struct SessionView {
    /// The telescope's site, if set.
    pub site: Option<Site>,
    /// The minimum altitude in degrees.
    pub min_alt_deg: f64,
    /// The resolved canonical catalog object, if selected.
    pub target: Option<&'static Target>,
    /// The latest pointing, if a frame has solved in this session.
    pub pointing: Option<Pointing>,
    /// Unix seconds from the application's clock.
    pub time: f64,
}

#[derive(Clone)]
struct Values {
    site: Option<Site>,
    min_alt_deg: f64,
    target: Option<&'static Target>,
    pointing: Option<Pointing>,
}

impl Default for Values {
    fn default() -> Self {
        Self {
            site: None,
            min_alt_deg: 15.0,
            target: None,
            pointing: None,
        }
    }
}

impl Values {
    fn view(&self, time: f64) -> SessionView {
        SessionView {
            site: self.site,
            min_alt_deg: self.min_alt_deg,
            target: self.target,
            pointing: self.pointing.clone(),
            time,
        }
    }
}

/// One shared observing session. By default it lives entirely in memory.
#[derive(Default)]
pub struct Session {
    values: Mutex<Values>,
    state_file: Option<PathBuf>,
}

/// An edit failed without changing the observing session.
#[derive(Debug)]
pub enum SessionError {
    /// An invalid site or minimum altitude.
    Invalid(String),
    /// The target id does not exist in the catalog.
    UnknownTarget(String),
    /// The state file could not be written.
    Storage(io::Error),
}

impl fmt::Display for SessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(message) => f.write_str(message),
            Self::UnknownTarget(id) => write!(f, "unknown target {id:?}"),
            Self::Storage(error) => write!(f, "could not write the session state file: {error}"),
        }
    }
}

impl std::error::Error for SessionError {}

impl Session {
    /// Read all session fields together, using time from the application's clock.
    pub fn snapshot(&self, time: f64) -> SessionView {
        self.values.lock().expect("observing session").view(time)
    }

    /// Record a successful solve and return its sequence. Pointings are never persisted.
    /// The position is the solve report's J2000 centre, in degrees.
    pub fn record_pointing(&self, frame: Frame, ra_deg: f64, dec_deg: f64, time: f64) -> u64 {
        let mut values = self.values.lock().expect("observing session");
        let seq = values.pointing.as_ref().map_or(1, |p| p.seq + 1);
        let mut pointing = Pointing {
            seq,
            frame,
            ra_deg,
            dec_deg,
            time,
            alt_deg: None,
            az_deg: None,
        };
        pointing.with_site(values.site);
        values.pointing = Some(pointing);
        seq
    }

    /// Validate the entire edit and persist it before changing any field.
    pub fn edit(&self, edit: SessionEdit, time: f64) -> Result<SessionView, SessionError> {
        let mut values = self.values.lock().expect("observing session");
        let next = values.edited(edit)?;
        if let Some(path) = &self.state_file {
            write_state(path, &next).map_err(SessionError::Storage)?;
        }
        *values = next;
        Ok(values.view(time))
    }

    /// Load a durable observing session, creating its state file and parent folder.
    ///
    /// An unreadable or invalid file is reported on stderr and renamed with a `.bad`
    /// suffix before a fresh session is written. Only site, minimum altitude and the
    /// canonical target id are durable; the clock and pointing are never stored.
    pub fn with_state_file(path: impl Into<PathBuf>) -> io::Result<Self> {
        let path = path.into();
        let values = match read_state(&path) {
            Ok(values) => values,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                let values = Values::default();
                write_state(&path, &values)?;
                values
            }
            Err(error) => {
                eprintln!(
                    "could not read session state file {}: {error}; keeping it as .bad",
                    path.display()
                );
                fs::rename(&path, path.with_added_extension("bad"))?;
                let values = Values::default();
                write_state(&path, &values)?;
                values
            }
        };
        Ok(Self {
            values: Mutex::new(values),
            state_file: Some(path),
        })
    }
}

impl Values {
    fn edited(&self, edit: SessionEdit) -> Result<Self, SessionError> {
        if let Some(Some(site)) = edit.site {
            check_range("site.lat_deg", site.lat_deg, -90.0, 90.0)?;
            check_range("site.lon_deg", site.lon_deg, -180.0, 180.0)?;
        }
        if let Some(min) = edit.min_alt_deg {
            check_range("min_alt_deg", min, 0.0, 80.0)?;
        }
        let target = edit
            .target
            .map(|id| {
                id.map(|id| targets::find(&id).ok_or(SessionError::UnknownTarget(id)))
                    .transpose()
            })
            .transpose()?;
        let mut next = self.clone();
        if let Some(site) = edit.site {
            next.site = site;
            if let Some(pointing) = &mut next.pointing {
                pointing.with_site(site);
            }
        }
        if let Some(min) = edit.min_alt_deg {
            next.min_alt_deg = min;
        }
        if let Some(target) = target {
            next.target = target;
        }
        Ok(next)
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredSession {
    site: Option<Site>,
    min_alt_deg: f64,
    target: Option<String>,
}

fn read_state(path: &Path) -> io::Result<Values> {
    let bytes = fs::read(path)?;
    let stored: StoredSession = serde_json::from_slice(&bytes)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    Values::default()
        .edited(SessionEdit {
            site: Some(stored.site),
            min_alt_deg: Some(stored.min_alt_deg),
            target: Some(stored.target),
        })
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
}

fn write_state(path: &Path, values: &Values) -> io::Result<()> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }
    let temporary = path.with_added_extension("tmp");
    let file = File::create(&temporary)?;
    serde_json::to_writer(
        &file,
        &StoredSession {
            site: values.site,
            min_alt_deg: values.min_alt_deg,
            target: values.target.map(|target| target.id.clone()),
        },
    )
    .map_err(io::Error::other)?;
    file.sync_all()?;
    fs::rename(temporary, path)
}

fn check_range(field: &str, value: f64, min: f64, max: f64) -> Result<(), SessionError> {
    if (min..=max).contains(&value) {
        Ok(())
    } else {
        Err(SessionError::Invalid(format!(
            "{field} must be within {min}–{max} degrees"
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Oracle: a failed durable write must leave the previously observed session intact.
    #[test]
    fn failed_state_write_does_not_commit_the_edit() {
        let folder = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../target")
            .join(format!("session-write-failure-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        let path = folder.join("session.json");
        let session = Session::with_state_file(&path).unwrap();
        // A directory at the required temporary filename makes writing fail even as root.
        std::fs::create_dir(path.with_added_extension("tmp")).unwrap();
        let edit = SessionEdit {
            min_alt_deg: Some(30.0),
            ..SessionEdit::default()
        };
        assert!(matches!(
            session.edit(edit, 0.0),
            Err(SessionError::Storage(_))
        ));
        assert_eq!(session.snapshot(0.0).min_alt_deg, 15.0);
        let restored = Session::with_state_file(&path).unwrap();
        assert_eq!(restored.snapshot(0.0).min_alt_deg, 15.0);
        std::fs::remove_dir_all(folder).unwrap();
    }

    // Oracle: FR-009's exact defaults; no numerical tolerance.
    #[test]
    fn fresh_session_has_no_site_target_or_pointing_and_fifteen_degree_minimum() {
        let view = Session::default().snapshot(1768528800.0);
        assert!(view.site.is_none());
        assert!(view.target.is_none());
        assert!(view.pointing.is_none());
        assert_eq!(view.min_alt_deg, 15.0);
    }

    // Oracle: 2026-01-16 02:00 UT = 1768528800 Unix seconds, exactly.
    #[test]
    fn fixed_clock_reads_the_same_unix_time_twice() {
        let clock = FixedClock(1768528800.0);
        assert_eq!(clock.now(), 1768528800.0);
        assert_eq!(clock.now(), 1768528800.0);
    }
}
