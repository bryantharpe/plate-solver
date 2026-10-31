//! US4's independent oracles are catalog stars, cedar defaults, JPEG decoding,
//! the camera's reported ranges, and externally advanced tokio time.
use math_core::UnitVector;
use pattern_database::PatternDatabase;
use scope_link::{
    proto::{self, scope_node_client::ScopeNodeClient},
    BearerToken,
};
use scope_node::{
    detect::{self, DetectSettings},
    Camera, Settings,
};
use simulator::{Optics, SimCamera, SimNode, Simulator, Site, VirtualClock};
use std::{
    path::Path,
    sync::{Arc, LazyLock},
    time::Duration,
};
use tonic::{service::interceptor::InterceptedService, transport::Channel};

type Client = ScopeNodeClient<InterceptedService<Channel, BearerToken>>;
static DB: LazyLock<Arc<PatternDatabase>> = LazyLock::new(|| {
    Arc::new(
        pattern_database::load_from_path(
            &Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../reference-solutions/cedar-solve/tetra3/data/default_database.npz"),
        )
        .unwrap(),
    )
});
fn simulator() -> Simulator {
    Simulator::new(
        DB.clone(),
        Site::default(),
        VirtualClock::new(1_768_528_800.0),
    )
}
fn wire(exposure_ms: Option<f64>, gain: Option<f64>, sigma: f64) -> proto::Settings {
    proto::Settings {
        exposure_ms,
        gain,
        detection: Some(proto::Detection {
            sigma,
            binning: 2,
            normalize_rows: true,
            detect_hot_pixels: true,
        }),
    }
}
async fn client(node: &SimNode) -> Client {
    ScopeNodeClient::with_interceptor(
        node.memory_channel().await.unwrap(),
        BearerToken::new("test-token").unwrap(),
    )
}
async fn next(stream: &mut tonic::Streaming<proto::StarFrame>) -> proto::StarFrame {
    tokio::time::timeout(Duration::from_secs(5), stream.message())
        .await
        .unwrap()
        .unwrap()
        .unwrap()
}
fn assert_cedar(frame: &proto::StarFrame) {
    assert_eq!(
        frame.settings.as_ref().unwrap().detection,
        wire(None, None, 8.0).detection
    );
}

#[tokio::test(start_paused = true)]
async fn ten_dense_frames_have_all_fields_top_200_and_ids_count_dropped_exposures() {
    let sim = simulator();
    sim.set_optics(Optics {
        fov_deg: 20.0,
        ..Optics::default()
    });
    // Winter Milky Way, independently specified in J2000.
    let (alt, az) = simulator::sky::horizontal(
        UnitVector::from_radec(90_f64.to_radians(), 25_f64.to_radians()),
        Site::default(),
        sim.clock().now(),
    );
    sim.push_to(alt, az);
    assert!(
        sim.field_stars(1000.0, 1.0).len() > 200,
        "catalog field must exceed the wire cap"
    );
    let settings = Settings {
        exposure_ms: Some(1000.0),
        gain: Some(1.0),
        ..Settings::default()
    };
    let node = SimNode::new(sim.clone(), "test-token", settings).unwrap();
    let mut c = client(&node).await;
    let mut stream = c
        .frames(proto::FramesRequest {})
        .await
        .unwrap()
        .into_inner();
    let mut last = 0;
    for _ in 0..10 {
        let f = next(&mut stream).await;
        assert_eq!(f.id, last + 1);
        last = f.id;
        assert!(f.start_s >= 1000.0 && f.end_s > f.start_s);
        assert!((f.end_s - f.start_s - f.exposure_ms / 1000.0).abs() < 1e-6);
        assert_eq!((f.width, f.height), (1456, 1088));
        assert_eq!((f.exposure_ms, f.gain), (1000.0, 1.0));
        assert_eq!(f.settings, Some(wire(Some(1000.0), Some(1.0), 8.0)));
        assert_eq!(f.centroids.len(), 200);
        assert!(f
            .centroids
            .windows(2)
            .all(|p| p[0].brightness >= p[1].brightness));
        assert!(f.centroids.iter().all(|s| s.x >= 0.0
            && s.x < 1456.0
            && s.y >= 0.0
            && s.y < 1088.0
            && s.brightness > 0.0));
    }
    drop(next(&mut stream).await);
    assert_eq!(next(&mut stream).await.id, last + 2);
}

#[tokio::test(start_paused = true)]
async fn twenty_seconds_at_two_fps_obey_jpeg_size_and_five_second_spacing() {
    let sim = simulator();
    let node = SimNode::new(
        sim,
        "test-token",
        Settings {
            exposure_ms: Some(200.0),
            gain: Some(1.0),
            ..Settings::default()
        },
    )
    .unwrap()
    .with_frame_interval(Duration::from_millis(500));
    let mut c = client(&node).await;
    let mut stream = c
        .frames(proto::FramesRequest {})
        .await
        .unwrap()
        .into_inner();
    let mut previews = Vec::new();
    let mut starts = Vec::new();
    for _ in 0..40 {
        let f = next(&mut stream).await;
        starts.push(f.start_s);
        assert_cedar(&f);
        if let Some(jpeg) = f.preview_jpeg {
            assert_eq!(&jpeg[..2], &[0xff, 0xd8]);
            let image =
                image::load_from_memory_with_format(&jpeg, image::ImageFormat::Jpeg).unwrap();
            assert!(image.width().max(image.height()) <= 640);
            previews.push(f.end_s);
        }
    }
    assert_eq!(starts.len(), 40);
    assert!((starts[39] - starts[0] - 19.5).abs() < 1e-6);
    assert!(!previews.is_empty() && previews.len() <= 5);
    assert!(previews.windows(2).all(|p| p[1] - p[0] >= 5.0));
}

#[tokio::test(start_paused = true)]
async fn preview_request_overrides_limit_and_detection_applies_after_inflight_frame() {
    let sim = simulator();
    let node = SimNode::new(
        sim,
        "test-token",
        Settings {
            exposure_ms: Some(200.0),
            gain: Some(1.0),
            ..Settings::default()
        },
    )
    .unwrap()
    .with_frame_interval(Duration::from_millis(500));
    let mut c = client(&node).await;
    let mut stream = c
        .frames(proto::FramesRequest {})
        .await
        .unwrap()
        .into_inner();
    let first = next(&mut stream).await;
    assert!(first.preview_jpeg.is_some());
    assert_cedar(&first);
    // Read exactly one second of the stream, so the client asks 1 s after the preview.
    let second = next(&mut stream).await;
    let third = next(&mut stream).await;
    assert!(second.preview_jpeg.is_none() && third.preview_jpeg.is_none());
    assert!((third.end_s - first.end_s - 1.0).abs() < 1e-6);
    c.request_preview(proto::PreviewRequest {}).await.unwrap();
    let requested = next(&mut stream).await;
    assert!(requested.preview_jpeg.is_some());
    assert!(requested.end_s - first.end_s < 5.0);
    let response = c
        .configure(wire(Some(200.0), Some(1.0), 5.0))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(response, wire(Some(200.0), Some(1.0), 5.0));
    for i in 0..5 {
        let f = next(&mut stream).await;
        if i >= 1 {
            assert_eq!(f.settings.unwrap().detection.unwrap().sigma, 5.0);
        }
    }
}

// Select an actual catalog field with exactly five independently detected stars
// at the stated exposure. This calibration precedes the real service entirely.
async fn five_star_field(sim: &Simulator, exposure: f64, gain: f64) -> (f64, f64) {
    let mut cam = SimCamera::new(sim.clone());
    cam.apply(&Settings {
        exposure_ms: Some(exposure),
        gain: Some(gain),
        ..Settings::default()
    })
    .unwrap();
    for alt in [30.0, 50.0, 70.0] {
        for az in (0..360).step_by(15) {
            sim.push_to(alt, f64::from(az));
            let f = cam.next_frame().await.unwrap();
            // An automatic controller also needs enough catalog stars to reach 30.
            if detect::detect(&f, &DetectSettings::cedar()).len() == 5
                && (sim.field_stars(200.0, 16.0).len() >= 30 || gain == 4.0)
            {
                return (alt, f64::from(az));
            }
        }
    }
    panic!("the catalog must supply a five-star field at {exposure} ms, gain {gain}");
}
async fn assert_settles(
    stream: &mut tonic::Streaming<proto::StarFrame>,
    fixed_exposure: Option<f64>,
    initial: Option<proto::StarFrame>,
) {
    let mut counts: Vec<_> = initial.iter().map(|f| f.centroids.len()).collect();
    let mut gain_moved = false;
    for _ in 0..12 {
        let f = next(stream).await;
        if let Some(e) = fixed_exposure {
            assert_eq!(f.exposure_ms, e);
            let settings = f.settings.as_ref().unwrap();
            assert_eq!(settings.exposure_ms, Some(e));
            assert!(settings.gain.is_none());
            gain_moved |= f.gain > 1.0;
        }
        counts.push(f.centroids.len());
    }
    let settled = counts
        .iter()
        .position(|n| (25..=40).contains(n))
        .expect("reaches target band");
    assert!(settled < 6, "counts {counts:?}");
    assert!(
        counts[settled..].iter().all(|n| (25..=40).contains(n)),
        "counts {counts:?}"
    );
    if fixed_exposure.is_some() {
        assert!(gain_moved);
    }
}

#[tokio::test(start_paused = true)]
async fn automatic_exposure_settles_from_five_stars_and_stays_in_band() {
    let sim = simulator();
    sim.set_optics(Optics {
        fov_deg: 8.0,
        ..Optics::default()
    });
    five_star_field(&sim, 200.0, 1.0).await;
    let node = SimNode::new(sim, "test-token", Settings::default())
        .unwrap()
        .with_frame_interval(Duration::from_millis(500));
    let mut c = client(&node).await;
    let mut stream = c
        .frames(proto::FramesRequest {})
        .await
        .unwrap()
        .into_inner();
    let first = next(&mut stream).await;
    assert_eq!(first.centroids.len(), 5);
    assert!(first.settings.as_ref().unwrap().exposure_ms.is_none());
    assert_settles(&mut stream, None, Some(first)).await;
}

#[tokio::test(start_paused = true)]
async fn fixed_controls_hold_after_a_push_to_a_five_star_field() {
    let sim = simulator();
    // A narrow field permits five stars even at the fixed high gain.
    sim.set_optics(Optics {
        fov_deg: 3.0,
        ..Optics::default()
    });
    let field = five_star_field(&sim, 200.0, 4.0).await;
    sim.push_to(45.0, 180.0);
    let node = SimNode::new(sim.clone(), "test-token", Settings::default()).unwrap();
    let mut c = client(&node).await;
    c.configure(wire(Some(200.0), Some(4.0), 8.0))
        .await
        .unwrap();
    let mut stream = c
        .frames(proto::FramesRequest {})
        .await
        .unwrap()
        .into_inner();
    next(&mut stream).await;
    sim.push_to(field.0, field.1);
    for i in 0..10 {
        let f = next(&mut stream).await;
        assert_eq!((f.exposure_ms, f.gain), (200.0, 4.0));
        if i == 1 {
            assert_eq!(f.centroids.len(), 5);
        }
    }
}

#[tokio::test(start_paused = true)]
async fn fixed_exposure_and_automatic_gain_settle_after_a_push() {
    let sim = simulator();
    sim.set_optics(Optics {
        fov_deg: 8.0,
        ..Optics::default()
    });
    let field = five_star_field(&sim, 200.0, 1.0).await;
    sim.push_to(45.0, 180.0);
    let node = SimNode::new(sim.clone(), "test-token", Settings::default())
        .unwrap()
        .with_frame_interval(Duration::from_millis(500));
    let mut c = client(&node).await;
    c.configure(wire(Some(200.0), None, 8.0)).await.unwrap();
    let mut stream = c
        .frames(proto::FramesRequest {})
        .await
        .unwrap()
        .into_inner();
    next(&mut stream).await;
    sim.push_to(field.0, field.1);
    assert_settles(&mut stream, Some(200.0), None).await;
}

#[tokio::test(start_paused = true)]
async fn both_transports_describe_the_same_camera_and_heartbeat_uses_node_clock() {
    let sim = simulator();
    let node = SimNode::new(sim.clone(), "test-token", Settings::default()).unwrap();
    let mut memory = client(&node).await;
    let a = memory
        .describe(proto::DescribeRequest {})
        .await
        .unwrap()
        .into_inner();
    let addr = node.listen_loopback().await.unwrap();
    let channel = Channel::from_shared(format!("http://{addr}"))
        .unwrap()
        .connect()
        .await
        .unwrap();
    let mut tcp =
        ScopeNodeClient::with_interceptor(channel, BearerToken::new("test-token").unwrap());
    assert_eq!(
        a,
        tcp.describe(proto::DescribeRequest {})
            .await
            .unwrap()
            .into_inner()
    );
    let before = sim.clock().node_now();
    let hb = memory
        .heartbeat(proto::HeartbeatRequest { t0: 42.0 })
        .await
        .unwrap()
        .into_inner();
    assert_eq!(hb.t0, 42.0);
    assert!(hb.t1 >= before && hb.t2 >= hb.t1 && hb.t2 <= sim.clock().node_now());
    let mut wrong = ScopeNodeClient::with_interceptor(
        node.memory_channel().await.unwrap(),
        BearerToken::new("wrong-token").unwrap(),
    );
    assert_eq!(
        wrong
            .describe(proto::DescribeRequest {})
            .await
            .unwrap_err()
            .code(),
        tonic::Code::Unauthenticated
    );
}

#[tokio::test(start_paused = true)]
async fn settings_refuse_invalid_values_atomically_and_all_calls_require_pairing() {
    let sim = simulator();
    let node = SimNode::new(
        sim,
        "test-token",
        Settings {
            exposure_ms: Some(200.0),
            gain: Some(4.0),
            ..Settings::default()
        },
    )
    .unwrap();
    let mut c = client(&node).await;
    for bad in [
        wire(Some(2001.0), Some(4.0), 8.0),
        wire(Some(200.0), Some(17.0), 8.0),
        wire(Some(200.0), Some(4.0), f64::NAN),
        wire(Some(200.0), Some(4.0), 0.0),
        proto::Settings {
            detection: Some(proto::Detection {
                binning: 3,
                ..wire(None, None, 8.0).detection.unwrap()
            }),
            ..wire(None, None, 8.0)
        },
    ] {
        assert_eq!(
            c.configure(bad).await.unwrap_err().code(),
            tonic::Code::InvalidArgument
        );
    }
    let mut stream = c
        .frames(proto::FramesRequest {})
        .await
        .unwrap()
        .into_inner();
    for _ in 0..3 {
        let f = next(&mut stream).await;
        assert_eq!((f.exposure_ms, f.gain), (200.0, 4.0));
        assert_cedar(&f);
    }
    let mut wrong = ScopeNodeClient::with_interceptor(
        node.memory_channel().await.unwrap(),
        BearerToken::new("wrong-token").unwrap(),
    );
    assert_eq!(
        wrong
            .frames(proto::FramesRequest {})
            .await
            .unwrap_err()
            .code(),
        tonic::Code::Unauthenticated
    );
    assert_eq!(
        wrong
            .heartbeat(proto::HeartbeatRequest { t0: 0.0 })
            .await
            .unwrap_err()
            .code(),
        tonic::Code::Unauthenticated
    );
    assert_eq!(
        wrong
            .configure(wire(None, None, 8.0))
            .await
            .unwrap_err()
            .code(),
        tonic::Code::Unauthenticated
    );
    assert_eq!(
        wrong
            .request_preview(proto::PreviewRequest {})
            .await
            .unwrap_err()
            .code(),
        tonic::Code::Unauthenticated
    );
}

#[tokio::test(start_paused = true)]
async fn push_stop_start_and_unresponsive_controls_reach_the_real_service() {
    let sim = simulator();
    let node = SimNode::new(sim.clone(), "test-token", Settings::default())
        .unwrap()
        .with_frame_interval(Duration::from_millis(500));
    let mut c = client(&node).await;
    let mut stream = c
        .frames(proto::FramesRequest {})
        .await
        .unwrap()
        .into_inner();
    next(&mut stream).await;
    sim.push_by(20.0, -5.0);
    assert_eq!(
        (sim.telescope().camera.alt, sim.telescope().camera.az),
        (40.0, 200.0)
    );
    sim.stop_frames();
    // Drain a possible exposure that was already in progress when stopped.
    let mut last = 0;
    loop {
        match tokio::time::timeout(Duration::from_secs(1), stream.message()).await {
            Ok(Ok(Some(f))) => last = f.id,
            Err(_) => break,
            other => panic!("unexpected stream termination: {other:?}"),
        }
    }
    assert!(
        tokio::time::timeout(Duration::from_secs(2), stream.message())
            .await
            .is_err()
    );
    sim.start_frames();
    let f = next(&mut stream).await;
    assert!(f.id > last);
    sim.set_unresponsive(true);
    assert!(tokio::time::timeout(
        Duration::from_secs(2),
        c.heartbeat(proto::HeartbeatRequest { t0: 0.0 })
    )
    .await
    .is_err());
    sim.set_unresponsive(false);
    let hb = c
        .heartbeat(proto::HeartbeatRequest { t0: 23.0 })
        .await
        .unwrap()
        .into_inner();
    assert_eq!(hb.t0, 23.0);
    next(&mut stream).await;
}
