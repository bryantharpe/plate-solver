//! Which constellation a position lies in: the IAU boundaries (Delporte 1930) as tabulated
//! by Roman (1987), searched in B1875 coordinates. See `data/README.md` for the sources.

use std::sync::OnceLock;

use serde::Serialize;

use crate::sky::precess_from_j2000;

/// Julian date of B1875.0, the equinox the boundaries are drawn in.
const B1875: f64 = 2_405_889.258_550_475;

/// A constellation: its IAU abbreviation and name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Constellation {
    /// IAU abbreviation, as `"Ori"`.
    pub abbr: &'static str,
    /// IAU name, as `"Orion"`.
    pub name: &'static str,
}

/// One row of Roman's table: from `dec_lo` northward, RA in `[ra_lo, ra_hi)` hours.
struct Band {
    ra_lo: f64,
    ra_hi: f64,
    dec_lo: f64,
    abbr: &'static str,
}

fn data_lines(text: &'static str) -> impl Iterator<Item = &'static str> {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
}

/// Roman's table, ordered as published: southward by lower declination edge.
fn bands() -> &'static [Band] {
    static BANDS: OnceLock<Vec<Band>> = OnceLock::new();
    BANDS.get_or_init(|| {
        data_lines(include_str!("../data/constellation_boundaries.dat"))
            .map(|line| {
                let f: Vec<&str> = line.split_whitespace().collect();
                let num = |i: usize| f[i].parse::<f64>().expect("numeric boundary field");
                Band {
                    ra_lo: num(0),
                    ra_hi: num(1),
                    dec_lo: num(2),
                    abbr: f[3],
                }
            })
            .collect()
    })
}

fn names() -> &'static [(&'static str, &'static str)] {
    static NAMES: OnceLock<Vec<(&str, &str)>> = OnceLock::new();
    NAMES.get_or_init(|| {
        data_lines(include_str!("../data/constellation_names.dat"))
            .map(|line| line.split_once(' ').expect("abbreviation and name"))
            .collect()
    })
}

/// The constellation with this IAU abbreviation, as `"Ori"`; case-sensitive.
pub fn by_abbr(abbr: &str) -> Option<Constellation> {
    names()
        .iter()
        .find(|(a, _)| *a == abbr)
        .map(|&(abbr, name)| Constellation { abbr, name })
}

/// The constellation containing a J2000 position, in degrees.
pub fn containing(ra_deg: f64, dec_deg: f64) -> Constellation {
    let (ra, dec) = precess_from_j2000(ra_deg, dec_deg, B1875);
    let ra_h = ra / 15.0;
    let band = bands()
        .iter()
        .find(|b| dec >= b.dec_lo && ra_h >= b.ra_lo && ra_h < b.ra_hi)
        .expect("the table's last band covers the sky down to the south pole");
    by_abbr(band.abbr).expect("every boundary names an IAU constellation")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_band_names_a_constellation_and_all_88_appear() {
        let mut seen: Vec<&str> = bands().iter().map(|b| b.abbr).collect();
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(seen.len(), 88);
        assert_eq!(names().len(), 88);
        for abbr in seen {
            assert!(by_abbr(abbr).is_some(), "{abbr} has no name");
        }
    }

    #[test]
    fn bright_stars_land_in_their_constellations() {
        for (ra, dec, abbr) in [
            (88.793, 7.407, "Ori"),    // Betelgeuse
            (85.191, -4.246, "Ori"),   // the orion2 sample's pointing
            (37.955, 89.264, "UMi"),   // Polaris
            (279.235, 38.784, "Lyr"),  // Vega
            (101.287, -16.716, "CMa"), // Sirius
            (10.685, 41.269, "And"),   // M31
            (219.902, -60.834, "Cen"), // Rigil Kentaurus
            (186.650, -63.099, "Cru"), // Acrux
            (0.0, 0.0, "Psc"),         // the vernal equinox
            (0.0, -89.9, "Oct"),       // the south pole
            (274.701, -13.807, "Ser"), // M16, in Serpens Cauda
        ] {
            assert_eq!(containing(ra, dec).abbr, abbr, "({ra}, {dec})");
        }
        assert_eq!(by_abbr("Ori").map(|c| c.name), Some("Orion"));
        assert_eq!(by_abbr("ori"), None);
    }

    #[test]
    fn names_are_spelled_as_the_iau_spells_them() {
        // The upstream file misspelled these three; data/README.md records the fix.
        for (abbr, name) in [
            ("Cha", "Chamaeleon"),
            ("Oph", "Ophiuchus"),
            ("PsA", "Piscis Austrinus"),
            ("Boo", "Boötes"),
        ] {
            assert_eq!(by_abbr(abbr).map(|c| c.name), Some(name));
        }
    }
}
