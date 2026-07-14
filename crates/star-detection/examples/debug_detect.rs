use star_detection::detect_stars;

fn main() {
    let mut img = vec![0u8; 80 * 80];
    for y in 35..=45 {
        for x in 35..=45 {
            let dx = x as i32 - 40;
            let dy = y as i32 - 40;
            let d2 = dx * dx + dy * dy;
            let v = if d2 <= 4 {
                255u8
            } else if d2 <= 16 {
                150
            } else {
                50
            };
            img[y * 80 + x] = v;
        }
    }
    let stars = detect_stars(&img, 80, 80, 8.0, 1, false, false);
    println!("stars={}", stars.len());
    for s in &stars {
        println!("{:?}", s);
    }
}
