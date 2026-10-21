//! The RA/Dec grid over a solved frame, drawn the way a star atlas draws its chart lines:
//! parallels and hour circles at round values, labelled where they meet the frame's edge.

use math_core::{angular_distance, UnitVector};
use serde::Serialize;

use crate::sky::Frame;

/// A grid line's kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GridKind {
    /// A line of constant right ascension (an hour circle).
    Ra,
    /// A line of constant declination (a parallel).
    Dec,
}

/// An edge of the frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Edge {
    /// `y = 0`.
    Top,
    /// `x = width`.
    Right,
    /// `y = height`.
    Bottom,
    /// `x = 0`.
    Left,
}

/// Where a grid line meets the frame's edge: where its label goes.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct EdgeTick {
    /// Image position on the edge.
    pub x: f64,
    /// See `x`.
    pub y: f64,
    /// Which edge.
    pub edge: Edge,
}

/// One line of the grid.
#[derive(Debug, Clone, Serialize)]
pub struct GridLine {
    /// Parallel or hour circle.
    pub kind: GridKind,
    /// Its coordinate, in degrees.
    pub value_deg: f64,
    /// As the chart labels it: `5h 40m`, `−4°`.
    pub label: String,
    /// The line in image coordinates, carried a little past the frame; the page clips it.
    pub paths: Vec<Vec<[f64; 2]>>,
    /// Where it crosses the frame's edge.
    pub ticks: Vec<EdgeTick>,
}

/// Declination spacings to choose from, degrees.
const DEC_STEPS: [f64; 10] = [0.25, 0.5, 1.0, 2.0, 5.0, 10.0, 15.0, 20.0, 30.0, 45.0];
/// Right ascension spacings to choose from, minutes of time.
const RA_STEPS_MIN: [f64; 14] = [
    1.0, 2.0, 4.0, 5.0, 10.0, 15.0, 20.0, 30.0, 60.0, 90.0, 120.0, 180.0, 240.0, 360.0,
];
/// About this many parallels across the frame's width.
const LINES_ACROSS: f64 = 4.0;
/// Points per line.
const SAMPLES: usize = 120;
/// How far past the frame lines are carried, as a fraction of its size.
const OVERSHOOT: f64 = 0.15;

fn nearest(steps: &[f64], want: f64) -> f64 {
    steps
        .iter()
        .copied()
        .min_by(|a, b| (a - want).abs().total_cmp(&(b - want).abs()))
        .expect("non-empty step list")
}

/// The grid for a solved frame, parallels first.
pub fn graticule(frame: &Frame) -> Vec<GridLine> {
    let (ra0, dec0) = frame.center.to_radec();
    let (ra0, dec0) = (ra0.to_degrees(), dec0.to_degrees());
    let (width, height) = frame.size();
    let reach = frame.half_diagonal().to_degrees() * (1.0 + 2.0 * OVERSHOOT);
    let fov = 2.0 * (frame.half_diagonal().to_degrees()) * width / width.hypot(height);
    let dec_step = nearest(&DEC_STEPS, fov / LINES_ACROSS);
    // Keep cells roughly square on the sky: hour circles converge toward the poles.
    let cos_dec = dec0.to_radians().cos().max(1e-3);
    let ra_step = nearest(&RA_STEPS_MIN, dec_step / cos_dec * 4.0) / 4.0;

    let dec_lo = (dec0 - reach).max(-90.0);
    let dec_hi = (dec0 + reach).min(90.0);
    let polar = dec0.abs() + reach >= 90.0;
    let ra_half = if polar {
        180.0
    } else {
        (reach / (dec0.abs() + reach).to_radians().cos()).min(180.0)
    };

    let mut lines = Vec::new();
    let mut d = (dec_lo / dec_step).ceil() * dec_step;
    while d <= dec_hi + 1e-9 {
        if d.abs() < 90.0 - 1e-9 {
            let points = (0..=SAMPLES).map(|i| {
                let t = i as f64 / SAMPLES as f64;
                (ra0 - ra_half + 2.0 * ra_half * t, d)
            });
            push_line(
                &mut lines,
                frame,
                reach,
                GridKind::Dec,
                d,
                dec_label(d, dec_step),
                points,
            );
        }
        d += dec_step;
    }
    let (first, last) = if polar {
        (0.0, 360.0 - ra_step / 2.0)
    } else {
        (((ra0 - ra_half) / ra_step).ceil() * ra_step, ra0 + ra_half)
    };
    let mut a = first;
    while a <= last + 1e-9 {
        let value = a.rem_euclid(360.0);
        let points = (0..=SAMPLES).map(|i| {
            let t = i as f64 / SAMPLES as f64;
            (value, dec_lo + (dec_hi - dec_lo) * t)
        });
        push_line(
            &mut lines,
            frame,
            reach,
            GridKind::Ra,
            value,
            ra_label(value),
            points,
        );
        a += ra_step;
    }
    lines
}

/// Project a line's points, split it where it leaves the drawable area, and keep it when
/// some of it crosses the frame.
fn push_line(
    lines: &mut Vec<GridLine>,
    frame: &Frame,
    reach: f64,
    kind: GridKind,
    value_deg: f64,
    label: String,
    points: impl Iterator<Item = (f64, f64)>,
) {
    let (width, height) = frame.size();
    let (mx, my) = (width * OVERSHOOT, height * OVERSHOOT);
    let drawable = |(x, y): (f64, f64)| x >= -mx && x <= width + mx && y >= -my && y <= height + my;
    let mut paths: Vec<Vec<[f64; 2]>> = vec![Vec::new()];
    for (ra, dec) in points {
        let v = UnitVector::from_radec(ra.to_radians(), dec.to_radians());
        let point = (angular_distance(frame.center, v).to_degrees() < reach)
            .then(|| frame.project(v))
            .flatten()
            .filter(|&p| drawable(p));
        match point {
            Some((x, y)) => paths.last_mut().expect("one path").push([x, y]),
            None if !paths.last().expect("one path").is_empty() => paths.push(Vec::new()),
            None => {}
        }
    }
    paths.retain(|p| p.len() >= 2);
    let ticks: Vec<EdgeTick> = paths
        .iter()
        .flat_map(|p| edge_ticks(p, width, height))
        .collect();
    let crosses_frame = paths
        .iter()
        .flatten()
        .any(|&[x, y]| frame.contains((x, y), 0.0));
    if crosses_frame {
        lines.push(GridLine {
            kind,
            value_deg,
            label,
            paths,
            ticks,
        });
    }
}

/// Where a polyline crosses the frame's border.
fn edge_ticks(path: &[[f64; 2]], width: f64, height: f64) -> Vec<EdgeTick> {
    let inside = |[x, y]: [f64; 2]| x >= 0.0 && x <= width && y >= 0.0 && y <= height;
    let mut ticks = Vec::new();
    for pair in path.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        if inside(a) == inside(b) {
            continue;
        }
        // Bisect for the crossing, starting from the inside end.
        let (mut lo, mut hi) = if inside(a) { (a, b) } else { (b, a) };
        for _ in 0..40 {
            let mid = [(lo[0] + hi[0]) / 2.0, (lo[1] + hi[1]) / 2.0];
            if inside(mid) {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        let [x, y] = lo;
        let distances = [
            (y, Edge::Top),
            (width - x, Edge::Right),
            (height - y, Edge::Bottom),
            (x, Edge::Left),
        ];
        let edge = distances
            .iter()
            .min_by(|p, q| p.0.total_cmp(&q.0))
            .expect("four edges")
            .1;
        ticks.push(EdgeTick { x, y, edge });
    }
    ticks
}

/// `−4°`, or `−4° 30′` on a sub-degree grid.
fn dec_label(dec: f64, step: f64) -> String {
    let sign = if dec < -1e-9 {
        "\u{2212}"
    } else if dec > 1e-9 {
        "+"
    } else {
        ""
    };
    let minutes = (dec.abs() * 60.0).round() as i64;
    if step >= 1.0 || minutes % 60 == 0 {
        format!("{sign}{}°", minutes / 60)
    } else {
        format!("{sign}{}° {:02}′", minutes / 60, minutes % 60)
    }
}

/// `5h 40m`, or `6h` on the hour.
fn ra_label(ra: f64) -> String {
    let minutes = (ra.rem_euclid(360.0) * 4.0).round() as i64 % 1440;
    if minutes % 60 == 0 {
        format!("{}h", minutes / 60)
    } else {
        format!("{}h {:02}m", minutes / 60, minutes % 60)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sky::rotation_from_radec_roll;
    use math_core::PinholeCamera;
    use plate_solver::Solution;

    fn frame(ra: f64, dec: f64, roll: f64) -> Frame {
        let solution = Solution {
            ra: Some(ra.to_radians()),
            dec: Some(dec.to_radians()),
            roll: Some(roll.to_radians()),
            camera: Some(PinholeCamera::new(800.0, 600.0, 10_f64.to_radians())),
            distortion: Some(0.0),
            ..Solution::default()
        };
        let f = Frame::from_solution(&solution).unwrap();
        // The frame's rotation is the one the solver's convention gives.
        let r = rotation_from_radec_roll(ra.to_radians(), dec.to_radians(), roll.to_radians());
        assert!((r.det() - 1.0).abs() < 1e-12);
        f
    }

    fn line<'a>(lines: &'a [GridLine], label: &str) -> &'a GridLine {
        lines.iter().find(|l| l.label == label).unwrap_or_else(|| {
            panic!(
                "no line {label}: {:?}",
                lines.iter().map(|l| &l.label).collect::<Vec<_>>()
            )
        })
    }

    #[test]
    fn on_the_equator_north_up_the_grid_is_square_to_the_frame() {
        let lines = graticule(&frame(90.0, 0.0, 0.0));
        // The equator through the center is horizontal; the 6h hour circle is vertical.
        let equator = line(&lines, "0°");
        assert_eq!(equator.kind, GridKind::Dec);
        assert!(equator
            .paths
            .iter()
            .flatten()
            .all(|p| (p[1] - 300.0).abs() < 1e-6));
        let edges: Vec<Edge> = equator.ticks.iter().map(|t| t.edge).collect();
        assert_eq!(
            edges,
            [Edge::Right, Edge::Left],
            "east is left, and RA grows eastward"
        );
        let six = line(&lines, "6h");
        assert_eq!(six.kind, GridKind::Ra);
        assert!(six
            .paths
            .iter()
            .flatten()
            .all(|p| (p[0] - 400.0).abs() < 1e-6));
        let mut edges: Vec<Edge> = six.ticks.iter().map(|t| t.edge).collect();
        edges.sort_by_key(|e| *e as u8);
        assert_eq!(edges, [Edge::Top, Edge::Bottom]);
        for t in six.ticks.iter().chain(&equator.ticks) {
            let on_edge = t.x.abs() < 1e-6
                || (t.x - 800.0).abs() < 1e-6
                || t.y.abs() < 1e-6
                || (t.y - 600.0).abs() < 1e-6;
            assert!(on_edge, "{t:?}");
        }
        // A 10° field gets 2° parallels and 10-minute hour circles.
        assert!(
            lines.iter().any(|l| l.label == "+2°") && lines.iter().any(|l| l.label == "\u{2212}2°")
        );
        assert!(lines.iter().any(|l| l.label == "5h 50m"));
    }

    #[test]
    fn near_the_pole_hour_circles_radiate() {
        let lines = graticule(&frame(30.0, 88.0, 25.0));
        let hour_circles = lines.iter().filter(|l| l.kind == GridKind::Ra).count();
        assert!(hour_circles >= 6, "{hour_circles}");
        for l in &lines {
            assert!(l.paths.iter().all(|p| p.len() >= 2));
            for p in l.paths.iter().flatten() {
                assert!(p[0].is_finite() && p[1].is_finite());
            }
        }
    }

    #[test]
    fn labels() {
        assert_eq!(dec_label(-4.0, 2.0), "\u{2212}4°");
        assert_eq!(dec_label(0.0, 2.0), "0°");
        assert_eq!(dec_label(2.5, 0.5), "+2° 30′");
        assert_eq!(dec_label(-3.0, 0.5), "\u{2212}3°");
        assert_eq!(ra_label(85.0), "5h 40m");
        assert_eq!(ra_label(90.0), "6h");
        assert_eq!(ra_label(359.999_9), "0h");
        assert_eq!(nearest(&DEC_STEPS, 2.6), 2.0);
    }
}
