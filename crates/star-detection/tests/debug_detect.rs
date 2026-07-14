use star_detection::detect_stars;

fn make_image(width: usize, height: usize, fill: u8) -> Vec<u8> {
    vec![fill; width * height]
}

#[test]
fn debug_detect_single_star() {
    let width = 200;
    let height = 200;
    let mut img = make_image(width, height, 20);
    let cx = 100.0;
    let cy = 100.0;
    for y in 80..120 {
        for x in 80..120 {
            let dx = x as f64 - cx;
            let dy = y as f64 - cy;
            let r2 = dx * dx + dy * dy;
            let v = 220.0 * (-r2 / 2.0).exp();
            let v = (v as u8).max(20);
            img[y * width + x] = v;
        }
    }

    // Replicate scan_rows threshold behavior for every row.
    let sigma_noise_2 = 13i64;
    let sigma_noise_3 = 20i64;
    let mut candidates = Vec::new();
    for y in 3..height - 3 {
        let row_offset = y * width;
        let mut row_min = u8::MAX;
        for x in (3..width - 3).step_by(64) {
            row_min = row_min.min(img[row_offset + x]);
        }
        if row_min == u8::MAX {
            row_min = img[row_offset + 3];
        }
        let threshold = row_min as i64 + sigma_noise_2 / 2;
        for x in 3..width - 3 {
            let c = img[row_offset + x] as i64;
            if c < threshold { continue; }
            let lb = img[row_offset + x - 3] as i64;
            let l = img[row_offset + x - 2] as i64;
            let lm = img[row_offset + x - 1] as i64;
            let rm = img[row_offset + x + 1] as i64;
            let r = img[row_offset + x + 2] as i64;
            let rb = img[row_offset + x + 3] as i64;
            let sig = 2 * c - (lb + rb);
            if sig < sigma_noise_2 { continue; }
            if l > c || r > c { continue; }
            if lm >= c || rm >= c { continue; }
            if (l == c && lm == c) || (r == c && rm == c) { continue; }
            if (lb - rb).abs() > sigma_noise_3 { continue; }
            candidates.push((x, y));
        }
    }

    // Form blobs manually.
    let mut parent: Vec<usize> = (0..candidates.len()).collect();
    fn find(parent: &mut [usize], i: usize) -> usize {
        let mut root = i;
        while parent[root] != root {
            root = parent[root];
        }
        let mut j = i;
        while parent[j] != root {
            let next = parent[j];
            parent[j] = root;
            j = next;
        }
        root
    }
    fn union(parent: &mut [usize], a: usize, b: usize) {
        let ra = find(parent, a);
        let rb = find(parent, b);
        if ra != rb {
            parent[rb] = ra;
        }
    }
    for i in 0..candidates.len() {
        for j in 0..candidates.len() {
            if i == j { continue; }
            let (x1, y1) = candidates[i];
            let (x2, y2) = candidates[j];
            if y1.abs_diff(y2) <= 1 && x1.abs_diff(x2) <= 3 {
                union(&mut parent, i, j);
            }
        }
    }
    let mut blobs: std::collections::HashMap<usize, Vec<(usize, usize)>> = std::collections::HashMap::new();
    for (i, &c) in candidates.iter().enumerate() {
        let root = find(&mut parent, i);
        blobs.entry(root).or_default().push(c);
    }

    // Process blob: expand to 3x3.
    for (_, blob) in &blobs {
        let xs: Vec<_> = blob.iter().map(|(x, _)| *x).collect();
        let ys: Vec<_> = blob.iter().map(|(_, y)| *y).collect();
        let left = *xs.iter().min().unwrap();
        let right = *xs.iter().max().unwrap();
        let top = *ys.iter().min().unwrap();
        let bottom = *ys.iter().max().unwrap();
        let mut core_left = left;
        let mut core_top = top;
        let mut core_width = right - left + 1;
        let mut core_height = bottom - top + 1;

        if core_width < 3 {
            let pad = (3 - core_width) / 2;
            core_left = core_left.saturating_sub(pad);
            core_width = 3;
            if core_left + core_width > width {
                core_left = width.saturating_sub(core_width);
            }
        }
        if core_height < 3 {
            let pad = (3 - core_height) / 2;
            core_top = core_top.saturating_sub(pad);
            core_height = 3;
            if core_top + core_height > height {
                core_top = height.saturating_sub(core_height);
            }
        }

        println!("expanded core_left={} core_top={} core_width={} core_height={}", core_left, core_top, core_width, core_height);

        let nb_left = core_left.saturating_sub(1);
        let nb_top = core_top.saturating_sub(1);
        let nb_width = (core_width + 2).min(width - nb_left);
        let nb_height = (core_height + 2).min(height - nb_top);

        let mg_left = core_left.saturating_sub(2);
        let mg_top = core_top.saturating_sub(2);
        let mg_width = (core_width + 4).min(width - mg_left);
        let mg_height = (core_height + 4).min(height - mg_top);

        let pr_left = core_left.saturating_sub(3);
        let pr_top = core_top.saturating_sub(3);
        let pr_width = (core_width + 6).min(width - pr_left);
        let pr_height = (core_height + 6).min(height - pr_top);

        println!("pr_left={} pr_top={} pr_width={} pr_height={}", pr_left, pr_top, pr_width, pr_height);
        println!("pr bounds check: {} <= {} and {} <= {}", pr_left + pr_width, width, pr_top + pr_height, height);

        fn box_mean(image: &[u8], width: usize, left: usize, top: usize, box_width: usize, box_height: usize) -> f64 {
            let mut sum = 0u64;
            for y in top..top + box_height {
                let row_offset = y * width;
                for x in left..left + box_width {
                    sum += u64::from(image[row_offset + x]);
                }
            }
            sum as f64 / (box_width * box_height) as f64
        }

        fn box_mean_excluding_corners(image: &[u8], width: usize, left: usize, top: usize, box_width: usize, box_height: usize) -> f64 {
            let mut sum = 0u64;
            let mut count = 0usize;
            for y in top..top + box_height {
                let row_offset = y * width;
                for x in left..left + box_width {
                    let is_corner = (y == top || y == top + box_height - 1) && (x == left || x == left + box_width - 1);
                    if is_corner { continue; }
                    sum += u64::from(image[row_offset + x]);
                    count += 1;
                }
            }
            if count == 0 { return 0.0; }
            sum as f64 / count as f64
        }

        fn box_stats_perimeter(image: &[u8], width: usize, left: usize, top: usize, box_width: usize, box_height: usize) -> (f64, f64, f64, f64) {
            let mut sum = 0u64;
            let mut count = 0usize;
            let mut min = f64::INFINITY;
            let mut max = f64::NEG_INFINITY;
            for y in top..top + box_height {
                let row_offset = y * width;
                for x in left..left + box_width {
                    let on_perimeter = y == top || y == top + box_height - 1 || x == left || x == left + box_width - 1;
                    if !on_perimeter { continue; }
                    let v = f64::from(image[row_offset + x]);
                    sum += v as u64;
                    count += 1;
                    if v < min { min = v; }
                    if v > max { max = v; }
                }
            }
            if count == 0 { return (0.0, 0.0, 0.0, 0.0); }
            let mean = sum as f64 / count as f64;
            let mut acc = 0.0;
            for y in top..top + box_height {
                let row_offset = y * width;
                for x in left..left + box_width {
                    let on_perimeter = y == top || y == top + box_height - 1 || x == left || x == left + box_width - 1;
                    if !on_perimeter { continue; }
                    let d = f64::from(image[row_offset + x]) - mean;
                    acc += d * d;
                }
            }
            (mean, (acc / count as f64).sqrt(), min, max)
        }

        let core_mean = box_mean(&img, width, core_left, core_top, core_width, core_height,
        );
        let neighbor_mean = box_mean_excluding_corners(
            &img, width, nb_left, nb_top, nb_width, nb_height,
        );
        let margin_mean = box_mean(
            &img, width, mg_left, mg_top, mg_width, mg_height,
        );
        let (perimeter_mean, perimeter_stddev, perimeter_min, perimeter_max) = box_stats_perimeter(
            &img, width, pr_left, pr_top, pr_width, pr_height,
        );

        println!("core_mean={} neighbor_mean={} margin_mean={} perimeter_mean={} stddev={} min={} max={}",
            core_mean, neighbor_mean, margin_mean, perimeter_mean, perimeter_stddev, perimeter_min, perimeter_max);
        println!("core >= neighbor: {}", core_mean >= neighbor_mean);
        println!("core > margin: {}", core_mean > margin_mean);
        println!("uniform perimeter: {} <= {}", perimeter_max - perimeter_min, 3.0 * 8.0 * 0.2);
        println!("significance: {} >= {}", core_mean - perimeter_mean, 8.0 * 0.2f64.max(perimeter_stddev));
    }

    let stars = detect_stars(&img, width, height, 8.0, 1, false, false);
    println!("stars={:?}", stars);
}
