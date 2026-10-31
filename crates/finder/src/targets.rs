//! The offline OpenNGC target catalog and its exact-id and common-name search.
//!
//! The pinned extract and its CC BY-SA 4.0 provenance are described in `data/README.md`.
//! Objects and duplicate aliases are loaded once from data compiled into the binary.
//! This catalog is separate from [`crate::catalog::messier`], which supplies the chart.

use std::collections::HashMap;
use std::sync::OnceLock;

use math_core::UnitVector;
use serde::Serialize;

use crate::catalog::{kind_of, Family};
use crate::pushto::{self, Class};
use crate::session::Site;

/// One search result, classed at the console's time for the observing session.
#[derive(Debug, Serialize)]
pub struct SearchResult {
    /// The canonical catalog object.
    #[serde(flatten)]
    pub object: &'static Target,
    /// Pointability now, with the same precedence as the session guidance.
    pub class: Class,
}

impl SearchResult {
    /// Class one result using the site, minimum altitude and time of a single read.
    pub fn at(object: &'static Target, site: Option<Site>, min_alt_deg: f64, time: f64) -> Self {
        Self {
            object,
            class: pushto::class(object, site, min_alt_deg, time),
        }
    }
}

/// One canonical OpenNGC object, shared by its designation, Messier id and aliases.
#[derive(Debug, Clone, Serialize)]
pub struct Target {
    /// Designation with a space and no leading zeros, or a Messier-only id (`M40`).
    pub id: String,
    /// Messier number, as `M31`, when present.
    pub messier: Option<String>,
    /// Every common name given by OpenNGC.
    pub common_names: Vec<String>,
    /// OpenNGC type code, as `G` or `Cl+N`.
    pub code: String,
    /// The type in plain words.
    pub kind: &'static str,
    /// Object family, shared with the Messier chart.
    pub family: Family,
    /// J2000 right ascension in degrees.
    pub ra_deg: f64,
    /// J2000 declination in degrees.
    pub dec_deg: f64,
    /// OpenNGC's constellation abbreviation (including `Se1` and `Se2` for Serpens).
    pub constellation: String,
    /// Major axis in arcminutes, when known.
    pub major_arcmin: Option<f64>,
    /// Minor axis in arcminutes, when known.
    pub minor_arcmin: Option<f64>,
    /// Major-axis position angle in degrees, north through east, when known.
    pub pa_deg: Option<f64>,
    /// Visual magnitude, falling back to blue magnitude, when known.
    pub mag: Option<f64>,
}

impl Target {
    /// Its J2000 direction on the sky.
    pub fn vector(&self) -> UnitVector {
        UnitVector::from_radec(self.ra_deg.to_radians(), self.dec_deg.to_radians())
    }
}

struct Catalog {
    objects: Vec<Target>,
    ids: HashMap<String, usize>,
}

/// A key keeps designation suffixes, removing only the space and initial numeric zeros.
fn normalize(id: &str) -> String {
    let id = id.trim();
    match id.find(|c: char| c.is_ascii_digit()) {
        Some(digits) => format!(
            "{}{}",
            id[..digits].trim_end(),
            id[digits..].trim_start_matches('0')
        )
        .to_ascii_uppercase(),
        None => id.to_ascii_uppercase(),
    }
}

// Same display form as catalog::designation; Messier-only names retain their M id.
fn canonical_id(name: &str) -> String {
    let digits = name
        .find(|c: char| c.is_ascii_digit())
        .expect("catalog number");
    let (catalog, number) = name.split_at(digits);
    let space = if catalog == "M" { "" } else { " " };
    format!("{catalog}{space}{}", number.trim_start_matches('0'))
}

impl Catalog {
    fn parse(extract: &str) -> Self {
        let mut catalog = Self {
            objects: Vec::new(),
            ids: HashMap::new(),
        };
        let mut aliases = Vec::new();
        for line in extract
            .lines()
            .skip(1)
            .filter(|line| !line.trim().is_empty())
        {
            let f: Vec<_> = line.split(';').collect();
            assert_eq!(f.len(), 12, "OpenNGC extract columns");
            match f[1] {
                "NonEx" => continue,
                "Dup" => {
                    aliases.push((f[0], f[11]));
                    continue;
                }
                _ => {}
            }
            let opt =
                |s: &str| (!s.is_empty()).then(|| s.parse::<f64>().expect("numeric OpenNGC field"));
            let index = catalog.objects.len();
            catalog.ids.insert(normalize(f[0]), index);
            if !f[9].is_empty() {
                catalog.ids.insert(normalize(f[9]), index);
            }
            catalog.objects.push(Target {
                id: canonical_id(f[0]),
                messier: (!f[9].is_empty()).then(|| f[9].to_string()),
                common_names: f[10]
                    .split(',')
                    .filter(|name| !name.is_empty())
                    .map(str::to_string)
                    .collect(),
                code: f[1].to_string(),
                kind: kind_of(f[1]),
                family: Family::of(f[1]),
                ra_deg: f[2].parse().expect("OpenNGC right ascension"),
                dec_deg: f[3].parse().expect("OpenNGC declination"),
                constellation: f[4].to_string(),
                major_arcmin: opt(f[5]),
                minor_arcmin: opt(f[6]),
                pa_deg: opt(f[7]),
                mag: opt(f[8]),
            });
        }
        // The extract resolves aliases directly to originals, which can occur later.
        for (alias, original) in aliases {
            let index = *catalog
                .ids
                .get(&normalize(original))
                .expect("OpenNGC duplicate's original");
            catalog.ids.insert(normalize(alias), index);
        }
        catalog
    }

    fn search(&self, query: &str, limit: usize) -> Vec<&Target> {
        let query = query.trim().to_lowercase();
        if query.is_empty() || limit == 0 {
            return Vec::new();
        }
        let exact = self
            .ids
            .get(&normalize(&query))
            .map(|&index| &self.objects[index]);
        let mut matches: Vec<_> = self
            .objects
            .iter()
            .filter(|object| {
                exact.is_some_and(|exact| std::ptr::eq(*object, exact))
                    || object.common_names.iter().any(|name| {
                        name.to_lowercase()
                            .split_whitespace()
                            .any(|word| word.starts_with(&query))
                    })
            })
            .collect();
        matches.sort_by(|a, b| {
            let is_exact = |object: &Target| exact.is_some_and(|exact| std::ptr::eq(object, exact));
            is_exact(b)
                .cmp(&is_exact(a))
                .then_with(|| {
                    a.mag
                        .unwrap_or(f64::INFINITY)
                        .total_cmp(&b.mag.unwrap_or(f64::INFINITY))
                })
                .then_with(|| a.id.cmp(&b.id))
        });
        matches.truncate(limit);
        matches
    }
}

fn catalog() -> &'static Catalog {
    static CATALOG: OnceLock<Catalog> = OnceLock::new();
    CATALOG.get_or_init(|| Catalog::parse(include_str!("../data/openngc.csv")))
}

/// Every canonical object, excluding duplicate and nonexistent entries.
pub fn all() -> &'static [Target] {
    &catalog().objects
}

/// Find a Messier, NGC, IC or addendum id, including duplicate aliases.
///
/// Case, the space before the number and its leading zeros do not matter.
pub fn find(id: &str) -> Option<&'static Target> {
    let catalog = catalog();
    catalog
        .ids
        .get(&normalize(id))
        .map(|&index| &catalog.objects[index])
}

/// Exact ids first, then common-name word prefixes in brightness order.
///
/// Matching ignores case. Objects without a magnitude come last; canonical id breaks
/// equal-magnitude ties. At most `limit` objects are returned, with no duplicates.
pub fn search(query: &str, limit: usize) -> Vec<&'static Target> {
    catalog().search(query, limit)
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use math_core::angular_distance;

    use super::*;
    use crate::catalog;

    const EXTRACT: &str = include_str!("../data/openngc.csv");
    const README: &str = include_str!("../data/README.md");

    #[test]
    fn object_and_alias_counts_match_the_pinned_extract_readme() {
        // Oracle: the maintainer's pinned extraction recipe; counts must agree exactly.
        let counts = README.split("**Object count: ").nth(1).unwrap();
        let words: Vec<_> = counts.split_whitespace().collect();
        let objects: usize = words[0].parse().unwrap();
        let aliases: usize = words[3].parse().unwrap();
        assert_eq!(all().len(), objects);
        assert_eq!(
            EXTRACT
                .lines()
                .filter(|row| row.split(';').nth(1) == Some("Dup"))
                .count(),
            aliases
        );
        let ids: HashSet<_> = all().iter().map(|object| &object.id).collect();
        assert_eq!(ids.len(), objects, "one canonical id per object");
        assert!(all()
            .iter()
            .all(|object| object.code != "Dup" && object.code != "NonEx"));
    }

    #[test]
    fn every_duplicate_resolves_to_its_original_without_becoming_an_object() {
        // Oracle: every alias and its destination in the pinned OpenNGC extract.
        let objects: HashSet<_> = all().iter().map(|object| object.id.as_str()).collect();
        let mut aliases = 0;
        for fields in EXTRACT
            .lines()
            .skip(1)
            .map(|row| row.split(';').collect::<Vec<_>>())
        {
            if fields[1] != "Dup" {
                continue;
            }
            aliases += 1;
            let duplicate = find(fields[0]).unwrap_or_else(|| panic!("missing {}", fields[0]));
            let original = find(fields[11]).unwrap_or_else(|| panic!("missing {}", fields[11]));
            assert!(
                std::ptr::eq(duplicate, original),
                "{} -> {}",
                fields[0],
                fields[11]
            );
            assert!(objects.contains(original.id.as_str()));
            let digits = fields[0].find(|c: char| c.is_ascii_digit()).unwrap();
            let alias_id = format!(
                "{} {}",
                &fields[0][..digits],
                fields[0][digits..].trim_start_matches('0')
            );
            assert!(
                !objects.contains(alias_id.as_str()),
                "alias {alias_id} listed as an object"
            );
        }
        assert_eq!(aliases, 651);
    }

    #[test]
    fn every_messier_position_agrees_with_the_existing_chart_catalog_within_one_arcsecond() {
        // Independent oracle: the console's existing 110-object Messier extract.
        assert_eq!(catalog::messier().len(), 110);
        for reference in catalog::messier() {
            let object = find(&reference.id).unwrap_or_else(|| panic!("missing {}", reference.id));
            let arcseconds =
                angular_distance(object.vector(), reference.vector()).to_degrees() * 3600.0;
            assert!(
                arcseconds < 1.0,
                "{} differs by {arcseconds} arcseconds",
                reference.id
            );
            assert_eq!(object.messier.as_deref(), Some(reference.id.as_str()));
            let expected_id = reference.designation.as_ref().unwrap_or(&reference.id);
            assert_eq!(&object.id, expected_id);
        }
    }

    #[test]
    fn disputed_and_addendum_messier_ids_follow_the_extract_notes() {
        for (query, id) in [
            ("M102", "NGC 5866"),
            ("M76", "NGC 650"),
            ("M45", "Mel 22"),
            ("M40", "M40"),
        ] {
            assert_eq!(find(query).unwrap().id, id);
        }
        assert!(std::ptr::eq(find("NGC651").unwrap(), find("M76").unwrap()));
        assert!(find("M111").is_none());
    }

    #[test]
    fn case_spacing_and_leading_zeros_find_the_same_canonical_object() {
        let andromeda = find("NGC 224").unwrap();
        for query in ["m31", "M 31", "ngc224", "NGC 0224", "ngc0224", " m0031 "] {
            assert!(std::ptr::eq(find(query).unwrap(), andromeda), "{query}");
        }
        assert_eq!(andromeda.id, "NGC 224");
        for (query, id) in [
            ("ic0434", "IC 434"),
            ("mel00022", "Mel 22"),
            ("pgc000143", "PGC 143"),
            ("eso351-030", "ESO 351-030"),
            ("ngc1530a", "NGC 1530A"),
        ] {
            assert_eq!(find(query).unwrap().id, id);
        }
    }

    #[test]
    fn unknown_ids_are_absent() {
        for query in ["NGC 99999", "M111", "", "qqq", "NGC"] {
            assert!(find(query).is_none(), "{query}");
        }
    }

    #[test]
    fn andromeda_preserves_the_openngc_fields() {
        // Oracle: NGC0224's row, with numeric fields compared exactly to its decimals.
        let object = find("M31").unwrap();
        assert_eq!(object.common_names, ["Andromeda Galaxy"]);
        assert_eq!(object.code, "G");
        assert_eq!((object.ra_deg, object.dec_deg), (10.68479, 41.26906));
        assert_eq!(object.constellation, "And");
        assert_eq!(
            (
                object.major_arcmin,
                object.minor_arcmin,
                object.pa_deg,
                object.mag
            ),
            (Some(177.83), Some(69.66), Some(35.0), Some(3.44))
        );
        assert_eq!(
            find("M76").unwrap().common_names,
            ["Barbell Nebula", "Cork Nebula", "Little Dumbbell Nebula"]
        );
        assert!(find("M22").unwrap().mag.is_none());
    }

    #[test]
    fn search_uses_name_word_prefixes_and_orders_by_magnitude_with_unknowns_last() {
        // Oracle: the extract's 60 names with a word beginning "nebula", and magnitudes.
        let matches = search("NEBULA", usize::MAX);
        assert_eq!(matches.len(), 60);
        assert_eq!(matches[0].id, "NGC 3372");
        assert!(matches.iter().any(|object| object.id == "NGC 1976"));
        assert_eq!(
            matches
                .iter()
                .filter(|object| object.id == "NGC 1976")
                .count(),
            1
        );
        let mut unknown = false;
        let mut previous = f64::NEG_INFINITY;
        for object in matches {
            if let Some(mag) = object.mag {
                assert!(!unknown, "known magnitude after an unknown one");
                assert!(mag >= previous, "objects must be brightest first");
                previous = mag;
            } else {
                unknown = true;
            }
        }
        assert!(unknown);
        assert_eq!(
            search("cork", 20)[0].id,
            "NGC 650",
            "all common names are searched"
        );
        assert_eq!(search("orion", 20)[0].id, "NGC 1976", "match any word");
        assert!(
            search("romeda", 20).is_empty(),
            "no substring matching inside a word"
        );
        assert!(search("qqq", 20).is_empty());
        assert!(search("  ", 20).is_empty());
        assert!(search("nebula", 0).is_empty());
        assert_eq!(search("M31", 1)[0].id, "NGC 224");
        assert_eq!(search("ngc0651", 20)[0].id, "NGC 650");
    }

    #[test]
    fn exact_ids_precede_brighter_name_matches_and_nonexistent_rows_are_excluded() {
        // Independent fixture: the exact M31 is faint, also matches by name, and has
        // a duplicate before its original. The other three matches have known ordering.
        let extract = [
            EXTRACT.lines().next().unwrap(),
            "IC0001;Dup;;;;;;;;;;NGC0001",
            "NGC0001;G;10;20;And;;;;15;M31;M31 Faint Galaxy;",
            "NGC0002;G;10;20;And;;;;1;;M31 Bright Galaxy;",
            "NGC0003;G;10;20;And;;;;;;M31 Unknown Galaxy;",
            "NGC0004;G;10;20;And;;;;3;;M31 Medium Galaxy;",
            "NGC0005;NonEx;;;;;;;;;;",
            "",
        ]
        .join("\n");
        let catalog = Catalog::parse(&extract);
        assert_eq!(catalog.objects.len(), 4);
        let ids: Vec<_> = catalog
            .search("m31", 20)
            .into_iter()
            .map(|object| object.id.as_str())
            .collect();
        assert_eq!(ids, ["NGC 1", "NGC 2", "NGC 4", "NGC 3"]);
        assert_eq!(catalog.search("ic1", 1)[0].id, "NGC 1");
        assert!(catalog.search("ngc5", 20).is_empty());
    }

    #[test]
    fn every_source_object_is_reachable_by_its_raw_and_canonical_id() {
        for fields in EXTRACT
            .lines()
            .skip(1)
            .map(|line| line.split(';').collect::<Vec<_>>())
        {
            if fields[1] == "Dup" {
                continue;
            }
            let object = find(fields[0]).unwrap_or_else(|| panic!("missing {}", fields[0]));
            assert!(std::ptr::eq(object, find(&object.id).unwrap()));
            assert_eq!(object.code, fields[1]);
            assert_eq!(object.constellation, fields[4]);
        }
    }
}
