//! The sample frames the console offers: the image files in one directory.

use std::path::{Path, PathBuf};

use serde::Serialize;

/// Image file extensions the solver reads.
const IMAGE_EXTENSIONS: [&str; 7] = ["jpg", "jpeg", "png", "bmp", "pgm", "ppm", "pnm"];

/// The sample frames: image files directly inside one directory, by file name.
#[derive(Debug, Clone, Default)]
pub struct Samples {
    dir: PathBuf,
    names: Vec<String>,
}

/// One sample, as the page lists it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SampleInfo {
    /// File name, as `"orion_belt.jpg"`.
    pub name: String,
    /// Readable label, as `"Orion belt"`.
    pub label: String,
}

impl Samples {
    /// The image files directly inside `dir`, sorted by name. Subdirectories are skipped.
    pub fn scan(dir: &Path) -> std::io::Result<Self> {
        let mut names = Vec::new();
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().into_owned();
            if entry.file_type()?.is_file() && is_image_name(&name) {
                names.push(name);
            }
        }
        names.sort();
        Ok(Self {
            dir: dir.to_path_buf(),
            names,
        })
    }

    /// The samples, as the page lists them.
    pub fn list(&self) -> Vec<SampleInfo> {
        self.names
            .iter()
            .map(|name| SampleInfo {
                label: label_of(name),
                name: name.clone(),
            })
            .collect()
    }

    /// How many samples there are.
    pub fn len(&self) -> usize {
        self.names.len()
    }

    /// Whether there are none.
    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }

    /// The file of a listed sample. Only names from the listing resolve, so a request
    /// cannot reach any other file.
    pub fn path(&self, name: &str) -> Option<PathBuf> {
        self.names
            .iter()
            .any(|n| n == name)
            .then(|| self.dir.join(name))
    }
}

fn extension(name: &str) -> String {
    Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
}

/// Whether a file name has an image extension the solver reads.
pub fn is_image_name(name: &str) -> bool {
    IMAGE_EXTENSIONS.contains(&extension(name).as_str())
}

/// The media type of an image file, by extension.
pub fn content_type(name: &str) -> &'static str {
    match extension(name).as_str() {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "bmp" => "image/bmp",
        _ => "image/x-portable-anymap",
    }
}

/// `orion_belt.jpg` → `Orion belt`; a leading capture timestamp is dropped
/// (`2019-07-29T204726_Alt40.jpg` → `Alt40`).
fn label_of(name: &str) -> String {
    let stem = Path::new(name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(name);
    let b = stem.as_bytes();
    let stamped = b.len() > 18
        && (b[4], b[7], b[10], b[17]) == (b'-', b'-', b'T', b'_')
        && b[..17]
            .iter()
            .enumerate()
            .all(|(i, c)| [4, 7, 10].contains(&i) || c.is_ascii_digit());
    let stem = if stamped { &stem[18..] } else { stem }.replace('_', " ");
    let mut chars = stem.chars();
    chars
        .next()
        .map(|c| c.to_uppercase().chain(chars).collect())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_labels_and_types() {
        assert!(is_image_name("a.JPG") && is_image_name("b.pgm") && !is_image_name("c.txt"));
        assert!(!is_image_name("README"));
        assert_eq!(content_type("x.jpeg"), "image/jpeg");
        assert_eq!(content_type("x.PNG"), "image/png");
        assert_eq!(content_type("x.bmp"), "image/bmp");
        assert_eq!(content_type("x.pgm"), "image/x-portable-anymap");
        assert_eq!(label_of("orion_belt.jpg"), "Orion belt");
        assert_eq!(
            label_of("2019-07-29T204726_Alt40_Azi-135.jpg"),
            "Alt40 Azi-135"
        );
        assert_eq!(label_of("2019-07-29_notes.jpg"), "2019-07-29 notes");
    }

    #[test]
    fn scan_lists_images_only_and_paths_resolve_only_for_listed_names() {
        let dir = std::env::temp_dir().join(format!("finder-samples-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("output")).unwrap();
        for name in ["b.jpg", "a.png", "notes.txt"] {
            std::fs::write(dir.join(name), b"x").unwrap();
        }
        let samples = Samples::scan(&dir).unwrap();
        let names: Vec<String> = samples.list().into_iter().map(|s| s.name).collect();
        assert_eq!(names, ["a.png", "b.jpg"]);
        assert_eq!((samples.len(), samples.is_empty()), (2, false));
        assert_eq!(samples.path("a.png"), Some(dir.join("a.png")));
        assert_eq!(samples.path("notes.txt"), None);
        assert_eq!(samples.path("../a.png"), None);
        assert!(Samples::default().is_empty());
        assert!(Samples::scan(&dir.join("missing")).is_err());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
