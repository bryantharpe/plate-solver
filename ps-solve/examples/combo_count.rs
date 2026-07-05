use ps_db::{importer, loader};
use ps_solve::{solve_from_image, SolveParams};
use std::path::PathBuf;
use std::time::Instant;

const ASTRONOMICAL_IMAGES: &[&str] = &[
    "2019-07-29T204726_Alt40_Azi-135_Try1.jpg",
    "2019-07-29T204726_Alt40_Azi135_Try1.jpg",
    "2019-07-29T204726_Alt40_Azi-45_Try1.jpg",
    "2019-07-29T204726_Alt40_Azi45_Try1.jpg",
    "2019-07-29T204726_Alt60_Azi-135_Try1.jpg",
    "2019-07-29T204726_Alt60_Azi135_Try1.jpg",
    "2019-07-29T204726_Alt60_Azi-45_Try1.jpg",
    "2019-07-29T204726_Alt60_Azi45_Try1.jpg",
    "hale_bopp.jpg",
];

fn main() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    let npz_path = manifest.join("../reference-solutions/cedar-solve/tetra3/data/default_database.npz");
    let db_imported = importer::import_npz(&npz_path)
        .unwrap_or_else(|e| panic!("import_npz failed: {}", e));

    let tmp = tempfile::NamedTempFile::new().expect("tempfile");
    loader::save_native(&db_imported, tmp.path()).expect("save_native");
    let mut db = loader::load_native(tmp.path()).expect("load_native");
    db.build_kd_tree();

    let image_dir = manifest.join("../reference-solutions/cedar-detect/test_data");

    println!("{:<45} {:<12} {:>16} {:>10}", "image", "status", "combos_examined", "t_solve_s");

    for name in ASTRONOMICAL_IMAGES {
        let img_path = image_dir.join(name);
        let img = image::open(&img_path)
            .unwrap_or_else(|e| panic!("Cannot open {}: {}", img_path.display(), e))
            .into_luma8();

        let params = SolveParams {
            solve_timeout: Some(120_000),
            ..Default::default()
        };

        let t0 = Instant::now();
        let sol = solve_from_image(&db, &img, &params);
        let wall = t0.elapsed().as_secs_f64();

        println!(
            "{:<45} {:<12} {:>16} {:>10.3}",
            name,
            format!("{:?}", sol.status),
            sol.combos_examined,
            sol.t_solve,
        );
        let _ = wall;
    }
}
