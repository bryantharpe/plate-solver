//! The deep-sky catalog: the 110 Messier objects (from OpenNGC; see `data/README.md`), and
//! "something like this one" for an observer.

use std::sync::OnceLock;

use math_core::{angular_distance, UnitVector};
use serde::Serialize;

use crate::constellation::{self, Constellation};
use crate::sky::{alt_az, Observer};

/// Altitude, in degrees, above which an object counts as well placed.
pub const MIN_ALTITUDE_DEG: f64 = 15.0;

/// Objects closer than this to the one asked about, in degrees, are part of the same view
/// (M43 beside M42) rather than something else to look at.
pub const SAME_VIEW_DEG: f64 = 1.0;

/// What kind of object, grouped the way an observer means "something like this".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Family {
    /// Emission, reflection and other diffuse nebulae, with or without a cluster.
    Nebula,
    /// Planetary nebulae.
    PlanetaryNebula,
    /// Supernova remnants.
    SupernovaRemnant,
    /// Open clusters and star associations.
    OpenCluster,
    /// Globular clusters.
    GlobularCluster,
    /// Galaxies, alone or in pairs and groups.
    Galaxy,
    /// Stars, double stars and asterisms.
    Star,
}

impl Family {
    /// The family of an OpenNGC type code.
    pub fn of(code: &str) -> Self {
        match code {
            "Neb" | "HII" | "EmN" | "RfN" | "DrkN" | "Cl+N" => Self::Nebula,
            "PN" => Self::PlanetaryNebula,
            "SNR" => Self::SupernovaRemnant,
            "OCl" | "*Ass" => Self::OpenCluster,
            "GCl" => Self::GlobularCluster,
            "G" | "GPair" | "GTrpl" | "GGroup" => Self::Galaxy,
            _ => Self::Star,
        }
    }

    /// The families whose members count as similar to this one's. The Crab is the only
    /// supernova remnant in the list, so it borrows the nebulae.
    pub fn similar(self) -> &'static [Family] {
        match self {
            Self::SupernovaRemnant => &[Self::PlanetaryNebula, Self::Nebula],
            Self::Nebula => &[Self::Nebula],
            Self::PlanetaryNebula => &[Self::PlanetaryNebula],
            Self::OpenCluster => &[Self::OpenCluster],
            Self::GlobularCluster => &[Self::GlobularCluster],
            Self::Galaxy => &[Self::Galaxy],
            Self::Star => &[Self::Star],
        }
    }

    /// The family in plain words, plural, as "diffuse nebulae".
    pub fn plural(self) -> &'static str {
        match self {
            Self::Nebula => "diffuse nebulae",
            Self::PlanetaryNebula => "planetary nebulae",
            Self::SupernovaRemnant => "supernova remnants",
            Self::OpenCluster => "open clusters",
            Self::GlobularCluster => "globular clusters",
            Self::Galaxy => "galaxies",
            Self::Star => "stars and asterisms",
        }
    }
}

/// An OpenNGC type code in plain words.
pub fn kind_of(code: &str) -> &'static str {
    match code {
        "*" => "Star",
        "**" => "Double star",
        "*Ass" => "Star association",
        "OCl" => "Open cluster",
        "GCl" => "Globular cluster",
        "Cl+N" => "Nebula with cluster",
        "G" => "Galaxy",
        "GPair" => "Galaxy pair",
        "GTrpl" => "Galaxy triplet",
        "GGroup" => "Galaxy group",
        "PN" => "Planetary nebula",
        "HII" | "EmN" => "Emission nebula",
        "RfN" => "Reflection nebula",
        "DrkN" => "Dark nebula",
        "Neb" => "Nebula",
        "SNR" => "Supernova remnant",
        "Nova" => "Nova",
        // In the Messier list only M73, a little knot of four stars, is "Other".
        _ => "Asterism",
    }
}

/// A deep-sky object.
#[derive(Debug, Clone, Serialize)]
pub struct DeepSkyObject {
    /// Messier number, as `"M42"`.
    pub id: String,
    /// NGC, IC or other designation, as `"NGC 1976"`, when it has one.
    pub designation: Option<String>,
    /// Common name, as `"Great Orion Nebula"`, when it has one.
    pub name: Option<String>,
    /// OpenNGC type code, as `"Cl+N"`.
    pub code: String,
    /// What it is, as `"Nebula with cluster"`.
    pub kind: &'static str,
    /// Its family, for "something like this".
    pub family: Family,
    /// J2000 right ascension in degrees.
    pub ra_deg: f64,
    /// J2000 declination in degrees.
    pub dec_deg: f64,
    /// The constellation it lies in.
    pub constellation: Constellation,
    /// Major axis in arcminutes, when known.
    pub major_arcmin: Option<f64>,
    /// Minor axis in arcminutes, when known.
    pub minor_arcmin: Option<f64>,
    /// Position angle of the major axis in degrees, north through east, when known.
    pub pa_deg: Option<f64>,
    /// Visual magnitude (blue when no visual one is known), when known.
    pub mag: Option<f64>,
}

impl DeepSkyObject {
    /// Its direction on the sky.
    pub fn vector(&self) -> UnitVector {
        UnitVector::from_radec(self.ra_deg.to_radians(), self.dec_deg.to_radians())
    }
}

/// `NGC1976` → `NGC 1976`, `Mel022` → `Mel 22`; Messier-only names (`M040`) → `None`.
fn designation(name: &str) -> Option<String> {
    let digits = name.find(|c: char| c.is_ascii_digit())?;
    let (catalog, number) = name.split_at(digits);
    if catalog == "M" {
        return None;
    }
    let number = number.trim_start_matches('0');
    Some(format!("{catalog} {number}"))
}

/// The 110 Messier objects, in Messier order.
pub fn messier() -> &'static [DeepSkyObject] {
    static OBJECTS: OnceLock<Vec<DeepSkyObject>> = OnceLock::new();
    OBJECTS.get_or_init(|| {
        include_str!("../data/messier.csv")
            .lines()
            .skip(1)
            .filter(|l| !l.trim().is_empty())
            .map(|line| {
                let f: Vec<&str> = line.split(';').collect();
                let opt = |s: &str| s.parse::<f64>().ok();
                let num = |s: &str| s.parse::<f64>().expect("numeric catalog field");
                let (ra_deg, dec_deg) = (num(f[3]), num(f[4]));
                DeepSkyObject {
                    id: f[0].to_string(),
                    designation: designation(f[1]),
                    name: (!f[10].is_empty()).then(|| f[10].to_string()),
                    code: f[2].to_string(),
                    kind: kind_of(f[2]),
                    family: Family::of(f[2]),
                    ra_deg,
                    dec_deg,
                    constellation: constellation::containing(ra_deg, dec_deg),
                    major_arcmin: opt(f[6]),
                    minor_arcmin: opt(f[7]),
                    pa_deg: opt(f[8]),
                    mag: opt(f[9]),
                }
            })
            .collect()
    })
}

/// The object with this Messier number, as `"M42"` or `"m42"`.
pub fn find(id: &str) -> Option<&'static DeepSkyObject> {
    messier()
        .iter()
        .find(|o| o.id.eq_ignore_ascii_case(id.trim()))
}

/// An object suggested as "something like this", with where it is.
#[derive(Debug, Clone, Serialize)]
pub struct Suggestion {
    /// The object.
    #[serde(flatten)]
    pub object: DeepSkyObject,
    /// Angular distance from the object asked about, in degrees: how far to slew.
    pub separation_deg: f64,
    /// Altitude in degrees, when an observer was given.
    pub alt_deg: Option<f64>,
    /// Azimuth in degrees from north through east, when an observer was given.
    pub az_deg: Option<f64>,
    /// Whether it is at least the requested minimum altitude up, when an observer was given.
    pub up: Option<bool>,
}

/// The answer to "something like this one".
#[derive(Debug, Clone, Serialize)]
pub struct SimilarReport {
    /// The object asked about.
    pub reference: DeepSkyObject,
    /// What the suggestions have in common, as "diffuse nebulae".
    pub family: &'static str,
    /// Who the altitudes are for, when given.
    pub observer: Option<Observer>,
    /// The suggestions, best first.
    pub suggestions: Vec<Suggestion>,
}

/// Objects like `reference`, best first: its family (or a related one), not the object
/// itself nor anything within [`SAME_VIEW_DEG`] of it, nearest first. With an observer,
/// the ones well placed now come before the ones that are not.
pub fn similar(
    reference: &DeepSkyObject,
    observer: Option<&Observer>,
    limit: usize,
) -> SimilarReport {
    similar_above(reference, observer, limit, 15.0)
}

/// Objects like `reference`, ranking those at or above `min_alt_deg` first.
/// Family, exclusion radius and distance ordering are the same as [`similar`].
pub fn similar_above(
    reference: &DeepSkyObject,
    observer: Option<&Observer>,
    limit: usize,
    min_alt_deg: f64,
) -> SimilarReport {
    let families = reference.family.similar();
    let mut suggestions: Vec<Suggestion> = messier()
        .iter()
        .filter(|o| o.id != reference.id && families.contains(&o.family))
        .map(|o| {
            let separation_deg = angular_distance(reference.vector(), o.vector()).to_degrees();
            let place = observer.map(|obs| alt_az(o.ra_deg, o.dec_deg, obs));
            Suggestion {
                object: o.clone(),
                separation_deg,
                alt_deg: place.map(|p| p.0),
                az_deg: place.map(|p| p.1),
                up: place.map(|p| p.0 >= min_alt_deg),
            }
        })
        .filter(|s| s.separation_deg >= SAME_VIEW_DEG)
        .collect();
    suggestions.sort_by(|a, b| {
        let down = |s: &Suggestion| s.up == Some(false);
        down(a)
            .cmp(&down(b))
            .then(a.separation_deg.total_cmp(&b.separation_deg))
    });
    suggestions.truncate(limit);
    SimilarReport {
        reference: reference.clone(),
        family: reference.family.plural(),
        observer: observer.copied(),
        suggestions,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_110_parse_in_order_with_known_fields() {
        let all = messier();
        assert_eq!(all.len(), 110);
        for (i, o) in all.iter().enumerate() {
            assert_eq!(o.id, format!("M{}", i + 1));
        }
        let m42 = find("m42").expect("M42");
        assert_eq!(m42.designation.as_deref(), Some("NGC 1976"));
        assert_eq!(m42.name.as_deref(), Some("Great Orion Nebula"));
        assert_eq!(
            (m42.kind, m42.family, m42.constellation.abbr),
            ("Nebula with cluster", Family::Nebula, "Ori")
        );
        assert_eq!(m42.mag, Some(4.0));
        assert_eq!(find("M40").and_then(|o| o.designation.clone()), None);
        assert_eq!(
            find("M45").and_then(|o| o.designation.clone()).as_deref(),
            Some("Mel 22")
        );
        assert_eq!(find("M22").and_then(|o| o.mag), None);
        assert!(find("M111").is_none() && find("").is_none());
    }

    #[test]
    fn computed_constellations_agree_with_openngc() {
        // The CSV keeps OpenNGC's own constellation (Serpens split into Se1/Se2); the
        // boundary lookup must land every object in the same one.
        for line in include_str!("../data/messier.csv").lines().skip(1) {
            let f: Vec<&str> = line.split(';').collect();
            let expected = match f[5] {
                "Se1" | "Se2" => "Ser",
                other => other,
            };
            let object = find(f[0]).unwrap();
            assert_eq!(object.constellation.abbr, expected, "{}", object.id);
        }
    }

    #[test]
    fn families_and_kinds_cover_every_code_in_the_list() {
        for o in messier() {
            assert_eq!(o.family, Family::of(&o.code));
            assert!(!o.kind.is_empty());
        }
        assert_eq!(kind_of("Other"), "Asterism");
        assert_eq!(Family::of("Other"), Family::Star);
        assert_eq!(Family::of("GPair"), Family::Galaxy);
        assert_eq!(Family::SupernovaRemnant.plural(), "supernova remnants");
    }

    #[test]
    fn like_m42_without_an_observer_is_the_nearest_other_nebula() {
        let report = similar(find("M42").unwrap(), None, 5);
        let ids: Vec<&str> = report
            .suggestions
            .iter()
            .map(|s| s.object.id.as_str())
            .collect();
        assert_eq!(ids.first(), Some(&"M78"), "{ids:?}");
        assert!(!ids.contains(&"M43"), "M43 shares M42's view");
        assert!(report.suggestions.iter().all(|s| s.up.is_none()));
        assert_eq!(report.family, "diffuse nebulae");
    }

    #[test]
    fn like_m42_on_a_january_evening_puts_summer_nebulae_last() {
        // 40° N, 75° W, 2026-01-16 02:00 UTC (9 pm local): Orion high, Sagittarius down.
        let observer = Observer {
            lat: 40.0,
            lon: -75.0,
            time: 1_768_528_800.0,
        };
        let report = similar(find("M42").unwrap(), Some(&observer), 10);
        let first = &report.suggestions[0];
        assert_eq!((first.object.id.as_str(), first.up), ("M78", Some(true)));
        let lagoon = report
            .suggestions
            .iter()
            .find(|s| s.object.id == "M8")
            .unwrap();
        assert_eq!(lagoon.up, Some(false));
        assert!(lagoon.alt_deg.unwrap() < 0.0);
        let ups: Vec<bool> = report.suggestions.iter().map(|s| s.up.unwrap()).collect();
        assert!(
            ups.windows(2).all(|w| w[0] >= w[1]),
            "up ones first: {ups:?}"
        );
    }

    #[test]
    fn the_crab_borrows_planetaries_and_nebulae() {
        let report = similar(find("M1").unwrap(), None, 200);
        assert!(!report.suggestions.is_empty());
        assert!(report
            .suggestions
            .iter()
            .all(|s| matches!(s.object.family, Family::PlanetaryNebula | Family::Nebula)));
    }
}
