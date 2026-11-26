//! Tests of the behaviour of the debugger at the exact edge of each rule: the findings of the checks on traces built by hand, the arcs, the times, the zoom of the
//! drawing, the words that the server link produces and the arguments of the window. Every number is chosen so that the expected result is worked out by hand.

use eh3d_cam_debugger::browse::{matches_extension, size_text, Browser};
use eh3d_cam_debugger::checks::{
    check, check_json, has_alerts, json_string, point_at, strict, summary, Clamp, Finding, Level,
    Limits,
};
use eh3d_cam_debugger::lang::{fill, Lang, Msg};
use eh3d_cam_debugger::server::{
    artifact_name, artifact_request, base64, check_key, is_server_key, media_type, require,
    sent_line, Sent, APP, MAX_SEND_BYTES,
};
use eh3d_cam_debugger::shot::{
    parse_size, parse_window_args, wants_picture, ShotTheme, DEFAULT_SIZE, DEFAULT_WAIT,
};
use eh3d_cam_debugger::timeline::{dwell_s, format_time, length, move_time, timeline, Machine};
use eh3d_cam_debugger::viewport::{Viewport, MAX_SCALE, MIN_SCALE};
use eh3d_parser_gcode::{
    codes, parse, Diagnostic, Dialect, Motion, Point3, Segment, Severity, Trace,
};
use eh3d_serverclient::Settings;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

fn temp(name: &str) -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "eh3d-camdebug-behaviour-{}-{n}-{name}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn strings(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| s.to_string()).collect()
}

fn pt(p: (f64, f64, f64)) -> Point3 {
    Point3 {
        x: p.0,
        y: p.1,
        z: p.2,
    }
}

fn seg(
    line: usize,
    motion: Motion,
    from: (f64, f64, f64),
    to: (f64, f64, f64),
    feed: f64,
    spindle_on: bool,
) -> Segment {
    Segment {
        line,
        motion,
        from: pt(from),
        to: pt(to),
        feed,
        spindle_speed: 0.0,
        spindle_on,
        tool: None,
    }
}

fn trace_of(segments: Vec<Segment>) -> Trace {
    Trace {
        segments,
        ended: true,
        ..Trace::default()
    }
}

/// A straight cut at feed 100 with the spindle on, above the surface, between two points of the plane.
fn cut(line: usize, from: (f64, f64), to: (f64, f64), z: f64) -> Segment {
    seg(
        line,
        Motion::Linear,
        (from.0, from.1, z),
        (to.0, to.1, z),
        100.0,
        true,
    )
}

fn messages(findings: &[Finding]) -> Vec<String> {
    findings.iter().map(|f| f.message.clone()).collect()
}

// ---------------------------------------------------------------- the checks

#[test]
fn an_empty_program_has_no_findings_whatever_the_limits() {
    let limits = Limits {
        table: Some((10.0, 10.0)),
        require_end: true,
        end_z: Some(5.0),
        ..Limits::default()
    };
    assert!(check(&Trace::default(), &limits).is_empty());
}

#[test]
fn a_rapid_move_down_into_the_material_is_an_alert_with_its_line() {
    let t = trace_of(vec![seg(
        7,
        Motion::Rapid,
        (0.0, 0.0, 5.0),
        (0.0, 0.0, -1.0),
        0.0,
        false,
    )]);
    let f = check(&t, &Limits::default());
    assert_eq!(f.len(), 1);
    assert_eq!((f[0].line, f[0].level), (7, Level::Alert));
    assert_eq!(
        f[0].message,
        "a rapid move (G0) goes into the material: Z -1.000 is below the surface (0.000)"
    );
}

#[test]
fn a_rapid_move_that_only_climbs_out_of_the_material_is_allowed_and_a_sideways_one_is_not() {
    let up = trace_of(vec![seg(
        1,
        Motion::Rapid,
        (0.0, 0.0, -1.0),
        (0.0, 0.0, 5.0),
        0.0,
        false,
    )]);
    assert!(check(&up, &Limits::default()).is_empty());
    let side = trace_of(vec![seg(
        1,
        Motion::Rapid,
        (0.0, 0.0, -1.0),
        (5.0, 0.0, -1.0),
        0.0,
        false,
    )]);
    assert_eq!(check(&side, &Limits::default()).len(), 1);
}

#[test]
fn the_surface_height_moves_the_line_between_air_and_material() {
    let t = trace_of(vec![seg(
        1,
        Motion::Rapid,
        (0.0, 0.0, 5.0),
        (0.0, 0.0, -1.0),
        0.0,
        false,
    )]);
    let deep = Limits {
        surface_z: -2.0,
        ..Limits::default()
    };
    assert!(check(&t, &deep).is_empty());
    let exact = Limits {
        surface_z: -1.0,
        ..Limits::default()
    };
    assert!(check(&t, &exact).is_empty());
}

#[test]
fn a_cut_below_the_surface_with_the_spindle_off_is_a_warning_and_with_it_on_is_nothing() {
    let off = trace_of(vec![seg(
        2,
        Motion::Linear,
        (0.0, 0.0, -1.0),
        (5.0, 0.0, -1.0),
        100.0,
        false,
    )]);
    let f = check(&off, &Limits::default());
    assert_eq!(messages(&f), vec!["it cuts with the spindle off"]);
    assert_eq!(f[0].level, Level::Warning);
    let on = trace_of(vec![seg(
        2,
        Motion::Linear,
        (0.0, 0.0, -1.0),
        (5.0, 0.0, -1.0),
        100.0,
        true,
    )]);
    assert!(check(&on, &Limits::default()).is_empty());
    let air = trace_of(vec![seg(
        2,
        Motion::Linear,
        (0.0, 0.0, 1.0),
        (5.0, 0.0, 1.0),
        100.0,
        false,
    )]);
    assert!(check(&air, &Limits::default()).is_empty());
}

#[test]
fn a_cutting_move_without_a_feed_is_a_warning_that_the_limits_can_silence() {
    let t = trace_of(vec![seg(
        3,
        Motion::Linear,
        (0.0, 0.0, 1.0),
        (5.0, 0.0, 1.0),
        0.0,
        true,
    )]);
    let f = check(&t, &Limits::default());
    assert_eq!(
        messages(&f),
        vec!["a cutting move has no feed in the program (the default feed is used)"]
    );
    let silent = Limits {
        ignore_codes: vec![codes::FEED_MISSING.to_string()],
        ..Limits::default()
    };
    assert!(check(&t, &silent).is_empty());
}

#[test]
fn a_rapid_move_needs_no_feed() {
    let t = trace_of(vec![seg(
        1,
        Motion::Rapid,
        (0.0, 0.0, 5.0),
        (9.0, 0.0, 5.0),
        0.0,
        false,
    )]);
    assert!(check(&t, &Limits::default()).is_empty());
}

#[test]
fn the_notices_of_the_reader_become_findings_by_their_severity() {
    let mut t = trace_of(vec![]);
    t.diagnostics = vec![
        Diagnostic::new(4, Severity::Info, "TEST-INFO", "only to know"),
        Diagnostic::new(5, Severity::Warning, "TEST-WARN", "careful"),
        Diagnostic::new(6, Severity::Error, "TEST-ERR", "lost"),
    ];
    let f = check(&t, &Limits::default());
    assert_eq!(f.len(), 2);
    assert_eq!(
        (f[0].line, f[0].level, f[0].message.as_str()),
        (5, Level::Warning, "the reader says: careful [TEST-WARN]")
    );
    assert_eq!(
        (f[1].line, f[1].level, f[1].message.as_str()),
        (
            6,
            Level::Alert,
            "the reader could not understand it: lost [TEST-ERR]"
        )
    );
    let silent = Limits {
        ignore_codes: strings(&["TEST-WARN", "TEST-ERR"]),
        ..Limits::default()
    };
    assert!(check(&t, &silent).is_empty());
}

#[test]
fn a_missing_feed_that_the_reader_already_told_is_not_told_twice() {
    let mut t = trace_of(vec![seg(
        3,
        Motion::Linear,
        (0.0, 0.0, 1.0),
        (5.0, 0.0, 1.0),
        0.0,
        true,
    )]);
    t.diagnostics = vec![Diagnostic::new(
        3,
        Severity::Warning,
        codes::FEED_MISSING,
        "no feed",
    )];
    let f = check(&t, &Limits::default());
    assert_eq!(f.len(), 1);
    assert!(f[0].message.starts_with("the reader says: no feed"));
}

#[test]
fn the_feed_limit_is_exceeded_only_above_it() {
    let limits = Limits {
        max_feed_mm_min: Some(800.0),
        ..Limits::default()
    };
    let at = trace_of(vec![seg(
        1,
        Motion::Linear,
        (0.0, 0.0, 1.0),
        (5.0, 0.0, 1.0),
        800.0,
        true,
    )]);
    assert!(check(&at, &limits).is_empty());
    let over = trace_of(vec![seg(
        1,
        Motion::Linear,
        (0.0, 0.0, 1.0),
        (5.0, 0.0, 1.0),
        1000.0,
        true,
    )]);
    let f = check(&over, &limits);
    assert_eq!(
        messages(&f),
        vec!["the feed 1000 mm/min is above the limit of 800"]
    );
    assert_eq!(f[0].level, Level::Warning);
}

#[test]
fn the_plunge_limit_counts_only_moves_that_go_mostly_down() {
    let limits = Limits {
        max_plunge_mm_min: Some(300.0),
        ..Limits::default()
    };
    let plunge = trace_of(vec![seg(
        1,
        Motion::Linear,
        (0.0, 0.0, 5.0),
        (0.0, 0.0, 2.0),
        500.0,
        true,
    )]);
    let f = check(&plunge, &limits);
    assert_eq!(
        messages(&f),
        vec!["the plunge of 3.000 mm goes down at 500 mm/min, above the plunge limit of 300"]
    );
    let ramp = trace_of(vec![seg(
        1,
        Motion::Linear,
        (0.0, 0.0, 5.0),
        (10.0, 0.0, 4.0),
        500.0,
        true,
    )]);
    assert!(check(&ramp, &limits).is_empty());
    let slow = trace_of(vec![seg(
        1,
        Motion::Linear,
        (0.0, 0.0, 5.0),
        (0.0, 0.0, 2.0),
        300.0,
        true,
    )]);
    assert!(check(&slow, &limits).is_empty());
}

#[test]
fn the_lowest_z_of_the_machine_is_an_alert_only_below_it() {
    let limits = Limits {
        min_z: Some(-3.0),
        ..Limits::default()
    };
    let at = trace_of(vec![cut(1, (0.0, 0.0), (5.0, 0.0), -3.0)]);
    assert!(check(&at, &limits).is_empty());
    let below = trace_of(vec![cut(1, (0.0, 0.0), (5.0, 0.0), -5.0)]);
    let f = check(&below, &limits);
    assert_eq!(
        messages(&f),
        vec!["Z -5.000 is below the lowest Z of the machine (-3.000)"]
    );
    assert_eq!(f[0].level, Level::Alert);
}

#[test]
fn the_table_is_left_only_beyond_its_edge_with_a_tolerance_of_a_nanometre() {
    let limits = Limits {
        table: Some((100.0, 80.0)),
        ..Limits::default()
    };
    let on_edge = trace_of(vec![cut(1, (0.0, 0.0), (100.0, 80.0), 1.0)]);
    assert!(check(&on_edge, &limits).is_empty());
    let beyond = trace_of(vec![cut(1, (0.0, 0.0), (101.0, 0.0), 1.0)]);
    let f = check(&beyond, &limits);
    assert_eq!(
        messages(&f),
        vec!["(101.000, 0.000) is outside the table of 100 x 80 mm"]
    );
    let negative = trace_of(vec![cut(1, (0.0, 0.0), (-0.00000001, 0.0), 1.0)]);
    assert_eq!(check(&negative, &limits).len(), 1);
    let tiny = trace_of(vec![cut(1, (0.0, 0.0), (-0.0000000001, 0.0), 1.0)]);
    assert!(check(&tiny, &limits).is_empty());
}

#[test]
fn an_arc_that_bulges_out_of_the_table_is_found_although_both_ends_are_inside() {
    let limits = Limits {
        table: Some((100.0, 100.0)),
        ..Limits::default()
    };
    let ccw = Motion::ArcCcw {
        center: (50.0, 0.0),
    };
    let out = trace_of(vec![seg(
        1,
        ccw,
        (0.0, 0.0, 1.0),
        (100.0, 0.0, 1.0),
        100.0,
        true,
    )]);
    assert_eq!(check(&out, &limits).len(), 1);
    let cw = Motion::ArcCw {
        center: (50.0, 0.0),
    };
    let inside = trace_of(vec![seg(
        1,
        cw,
        (0.0, 0.0, 1.0),
        (100.0, 0.0, 1.0),
        100.0,
        true,
    )]);
    assert!(check(&inside, &limits).is_empty());
}

#[test]
fn a_large_arc_that_leaves_the_table_by_less_than_a_sampling_step_is_found() {
    // a circle of radius 100.05 about (150, 150) reaches 0.05 mm past each side of the box from 50 to 250; it starts half a step of 1/64 of a turn after the right
    // side, so no point sampled along it falls there
    let a0 = std::f64::consts::PI / 64.0;
    let start = (150.0 + 100.05 * a0.cos(), 150.0 + 100.05 * a0.sin(), 1.0);
    for motion in [
        Motion::ArcCcw {
            center: (150.0, 150.0),
        },
        Motion::ArcCw {
            center: (150.0, 150.0),
        },
    ] {
        let round = trace_of(vec![seg(1, motion, start, start, 100.0, true)]);
        let limits = Limits {
            table: Some((250.0, 400.0)),
            ..Limits::default()
        };
        assert_eq!(
            messages(&check(&round, &limits)),
            vec!["(250.050, 150.000) is outside the table of 250 x 400 mm"]
        );
        let wide = Limits {
            table: Some((250.05, 250.05)),
            ..Limits::default()
        };
        assert!(check(&round, &wide).is_empty());
    }
}

fn clamp() -> Clamp {
    Clamp {
        x0: 10.0,
        y0: 10.0,
        x1: 20.0,
        y1: 20.0,
        height_mm: 5.0,
    }
}

#[test]
fn a_head_that_passes_over_a_clamp_below_its_height_is_an_alert_naming_the_clamp_and_the_place() {
    let limits = Limits {
        clamps: vec![clamp()],
        ..Limits::default()
    };
    let t = trace_of(vec![cut(9, (0.0, 15.0), (30.0, 15.0), 2.0)]);
    let f = check(&t, &limits);
    assert_eq!(f.len(), 1);
    assert_eq!((f[0].line, f[0].level), (9, Level::Alert));
    assert_eq!(
        f[0].message,
        "the head passes over the clamp 1 at (10.00, 15.00) with Z below its height 5.00"
    );
}

#[test]
fn a_clamp_is_cleared_at_its_exact_height_and_beside_it_and_with_its_corners_in_any_order() {
    let limits = Limits {
        clamps: vec![clamp()],
        ..Limits::default()
    };
    let at_height = trace_of(vec![cut(1, (0.0, 15.0), (30.0, 15.0), 5.0)]);
    assert!(check(&at_height, &limits).is_empty());
    let beside = trace_of(vec![cut(1, (0.0, 25.0), (30.0, 25.0), 2.0)]);
    assert!(check(&beside, &limits).is_empty());
    let swapped = Limits {
        clamps: vec![Clamp {
            x0: 20.0,
            y0: 20.0,
            x1: 10.0,
            y1: 10.0,
            height_mm: 5.0,
        }],
        ..Limits::default()
    };
    let over = trace_of(vec![cut(1, (0.0, 15.0), (30.0, 15.0), 2.0)]);
    assert_eq!(check(&over, &swapped).len(), 1);
}

#[test]
fn the_clamp_that_is_named_is_the_one_that_is_hit_and_a_rapid_move_hits_it_too() {
    let far = Clamp {
        x0: 200.0,
        y0: 200.0,
        x1: 210.0,
        y1: 210.0,
        height_mm: 5.0,
    };
    let limits = Limits {
        clamps: vec![far, clamp()],
        ..Limits::default()
    };
    let t = trace_of(vec![seg(
        1,
        Motion::Rapid,
        (0.0, 15.0, 2.0),
        (30.0, 15.0, 2.0),
        0.0,
        false,
    )]);
    let f = check(&t, &limits);
    assert_eq!(f.len(), 1);
    assert!(f[0].message.contains("clamp 2"));
}

#[test]
fn the_end_of_the_program_is_checked_for_the_height_and_for_the_closing_code() {
    let t = trace_of(vec![cut(1, (0.0, 0.0), (5.0, 0.0), -1.0)]);
    let low = Limits {
        end_z: Some(5.0),
        ..Limits::default()
    };
    let f = check(&t, &low);
    assert_eq!(
        messages(&f),
        vec!["the program ends with the tool at Z -1.000, below the end height of 5.000"]
    );
    let mut open = t.clone();
    open.ended = false;
    let need = Limits {
        require_end: true,
        ..Limits::default()
    };
    assert_eq!(
        messages(&check(&open, &need)),
        vec!["the program does not end with M2 or M30"]
    );
    assert!(check(&t, &need).is_empty());
}

#[test]
fn the_findings_come_in_the_order_of_the_program_and_an_identical_one_is_told_once() {
    let off = |line| {
        seg(
            line,
            Motion::Linear,
            (0.0, 0.0, -1.0),
            (5.0, 0.0, -1.0),
            100.0,
            false,
        )
    };
    let t = trace_of(vec![off(5), off(2), off(5)]);
    let f = check(&t, &Limits::default());
    let lines: Vec<usize> = f.iter().map(|x| x.line).collect();
    assert_eq!(lines, vec![2, 5]);
}

#[test]
fn strict_turns_every_warning_into_an_alert() {
    let mut f = vec![
        Finding {
            line: 1,
            level: Level::Warning,
            message: "a".into(),
        },
        Finding {
            line: 2,
            level: Level::Alert,
            message: "b".into(),
        },
    ];
    assert!(has_alerts(&f));
    f.remove(1);
    assert!(!has_alerts(&f));
    strict(&mut f);
    assert!(has_alerts(&f));
    let mut none: Vec<Finding> = Vec::new();
    strict(&mut none);
    assert!(!has_alerts(&none));
}

#[test]
fn the_check_gives_the_same_findings_every_time() {
    let t = trace_of(vec![
        seg(
            1,
            Motion::Rapid,
            (0.0, 0.0, 5.0),
            (0.0, 0.0, -1.0),
            0.0,
            false,
        ),
        seg(
            2,
            Motion::Linear,
            (0.0, 0.0, -1.0),
            (50.0, 0.0, -1.0),
            0.0,
            false,
        ),
    ]);
    let limits = Limits {
        table: Some((10.0, 10.0)),
        clamps: vec![clamp()],
        ..Limits::default()
    };
    assert_eq!(check(&t, &limits), check(&t, &limits));
}

// ---------------------------------------------------------------- the summary and the report

#[test]
fn the_summary_counts_the_kinds_of_move_the_tools_and_the_feeds() {
    let mut a = seg(
        1,
        Motion::Rapid,
        (0.0, 0.0, 5.0),
        (1.0, 0.0, 5.0),
        9999.0,
        false,
    );
    a.tool = Some(2);
    let mut b = seg(
        2,
        Motion::Linear,
        (1.0, 0.0, 1.0),
        (2.0, 0.0, 1.0),
        100.0,
        true,
    );
    b.tool = Some(2);
    let mut c = seg(
        3,
        Motion::ArcCw { center: (2.0, 1.0) },
        (2.0, 0.0, 1.0),
        (3.0, 1.0, 1.0),
        300.0,
        true,
    );
    c.tool = Some(1);
    let d = seg(
        4,
        Motion::Linear,
        (3.0, 1.0, 1.0),
        (4.0, 1.0, 1.0),
        50.0,
        true,
    );
    let mut t = trace_of(vec![a, b, c, d]);
    t.min = pt((0.0, 0.0, 1.0));
    t.max = pt((4.0, 1.0, 5.0));
    let s = summary(&t);
    assert_eq!((s.rapid_moves, s.straight_cuts, s.arcs), (1, 2, 1));
    assert_eq!(s.tools, vec![1, 2]);
    assert_eq!(s.feed_range, Some((50.0, 300.0)));
    assert_eq!(s.extent, Some(((0.0, 0.0, 1.0), (4.0, 1.0, 5.0))));
}

#[test]
fn the_summary_of_nothing_has_no_extent_no_tools_and_no_feeds() {
    let s = summary(&Trace::default());
    assert_eq!((s.rapid_moves, s.straight_cuts, s.arcs), (0, 0, 0));
    assert!(s.extent.is_none() && s.tools.is_empty() && s.feed_range.is_none());
}

#[test]
fn text_in_the_report_is_escaped_for_json() {
    assert_eq!(json_string(""), "\"\"");
    assert_eq!(json_string("plain"), "\"plain\"");
    assert_eq!(json_string("a\"b\\c"), "\"a\\\"b\\\\c\"");
    assert_eq!(json_string("l1\nl2\r\tx"), "\"l1\\nl2\\r\\tx\"");
    assert_eq!(json_string("\u{1}\u{1f}"), "\"\\u0001\\u001f\"");
    assert_eq!(json_string("é日 \u{7f}"), "\"é日 \u{7f}\"");
}

#[test]
fn the_report_of_an_empty_program_is_this_exact_text() {
    let json = check_json(&Trace::default(), 1.5, 1.0, 0.5, 0.0, &[]);
    assert_eq!(
        json,
        "{\"moves\":0,\"rapid_moves\":0,\"straight_cuts\":0,\"arcs\":0,\"cutting_mm\":0,\"rapid_mm\":0,\"total_s\":1.5,\"cutting_s\":1,\"rapid_s\":0.5,\"waits_s\":0,\"extent\":null,\"tools\":[],\"feed_range\":null,\"ended\":false,\"findings\":[]}"
    );
}

#[test]
fn a_number_that_is_not_finite_is_null_in_the_report() {
    let json = check_json(
        &Trace::default(),
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        0.0,
        &[],
    );
    assert!(json.contains("\"total_s\":null,\"cutting_s\":null,\"rapid_s\":null,\"waits_s\":0"));
}

#[test]
fn the_report_lists_the_findings_escaped_and_keeps_its_keys_in_order() {
    let findings = vec![Finding {
        line: 3,
        level: Level::Alert,
        message: "say \"hi\"\n".into(),
    }];
    let json = check_json(
        &trace_of(vec![cut(3, (0.0, 0.0), (1.0, 0.0), 1.0)]),
        1.0,
        1.0,
        0.0,
        0.0,
        &findings,
    );
    assert!(json.ends_with(
        "\"findings\":[{\"line\":3,\"level\":\"alert\",\"message\":\"say \\\"hi\\\"\\n\"}]}"
    ));
    let keys = [
        "\"moves\"",
        "\"arcs\"",
        "\"total_s\"",
        "\"extent\"",
        "\"tools\"",
        "\"ended\":true",
        "\"findings\"",
    ];
    let mut last = 0;
    for key in keys {
        let at = json.find(key).unwrap_or_else(|| panic!("{key}"));
        assert!(at >= last, "{key}");
        last = at;
    }
    assert_eq!(json.matches('{').count(), json.matches('}').count());
}

#[test]
fn the_extent_and_the_feed_range_appear_in_the_report_when_there_are_moves() {
    let mut t = trace_of(vec![cut(1, (0.0, 0.0), (4.0, 3.0), 1.0)]);
    t.min = pt((0.0, 0.0, 1.0));
    t.max = pt((4.0, 3.0, 1.0));
    let json = check_json(&t, 0.0, 0.0, 0.0, 0.0, &[]);
    assert!(json.contains("\"extent\":{\"min\":[0,0,1],\"max\":[4,3,1]}"));
    assert!(json.contains("\"feed_range\":[100,100]"));
    assert!(json.contains("\"moves\":1"));
}

// ---------------------------------------------------------------- arcs and times

#[test]
fn a_point_of_a_move_goes_from_its_start_to_its_end() {
    let s = seg(
        1,
        Motion::Linear,
        (1.0, 2.0, 3.0),
        (5.0, 6.0, 7.0),
        100.0,
        true,
    );
    assert_eq!(point_at(&s, 0.0), (1.0, 2.0, 3.0));
    assert_eq!(point_at(&s, 1.0), (5.0, 6.0, 7.0));
    assert_eq!(point_at(&s, 0.5), (3.0, 4.0, 5.0));
}

#[test]
fn a_point_of_an_arc_follows_the_circle() {
    let s = seg(
        1,
        Motion::ArcCcw {
            center: (50.0, 0.0),
        },
        (0.0, 0.0, 1.0),
        (100.0, 0.0, 1.0),
        100.0,
        true,
    );
    let (x0, y0, _) = point_at(&s, 0.0);
    let (x1, y1, _) = point_at(&s, 1.0);
    let (xm, ym, zm) = point_at(&s, 0.5);
    assert!(x0.abs() < 1e-9 && y0.abs() < 1e-9);
    assert!((x1 - 100.0).abs() < 1e-9 && y1.abs() < 1e-9);
    assert!((xm - 50.0).abs() < 1e-9 && (ym + 50.0).abs() < 1e-9 && zm == 1.0);
}

#[test]
fn the_length_of_a_straight_move_is_the_distance_in_three_dimensions() {
    let flat = seg(
        1,
        Motion::Linear,
        (0.0, 0.0, 0.0),
        (3.0, 4.0, 0.0),
        1.0,
        true,
    );
    let high = seg(
        1,
        Motion::Rapid,
        (0.0, 0.0, 0.0),
        (3.0, 4.0, 12.0),
        0.0,
        false,
    );
    assert_eq!(length(&flat), 5.0);
    assert_eq!(length(&high), 13.0);
    let still = seg(
        1,
        Motion::Linear,
        (1.0, 1.0, 1.0),
        (1.0, 1.0, 1.0),
        1.0,
        true,
    );
    assert_eq!(length(&still), 0.0);
}

#[test]
fn the_length_of_an_arc_is_the_arc_and_a_full_circle_is_a_full_turn() {
    let quarter = seg(
        1,
        Motion::ArcCcw { center: (0.0, 0.0) },
        (10.0, 0.0, 0.0),
        (0.0, 10.0, 0.0),
        1.0,
        true,
    );
    assert!((length(&quarter) - 10.0 * std::f64::consts::FRAC_PI_2).abs() < 1e-9);
    let cw_long = seg(
        1,
        Motion::ArcCw { center: (0.0, 0.0) },
        (10.0, 0.0, 0.0),
        (0.0, 10.0, 0.0),
        1.0,
        true,
    );
    assert!((length(&cw_long) - 10.0 * 1.5 * std::f64::consts::PI).abs() < 1e-9);
    let full = seg(
        1,
        Motion::ArcCcw { center: (0.0, 0.0) },
        (10.0, 0.0, 0.0),
        (10.0, 0.0, 0.0),
        1.0,
        true,
    );
    assert!((length(&full) - 20.0 * std::f64::consts::PI).abs() < 1e-9);
}

#[test]
fn a_helix_adds_the_height_and_an_arc_without_a_radius_is_the_straight_line() {
    let helix = seg(
        1,
        Motion::ArcCcw { center: (0.0, 0.0) },
        (10.0, 0.0, 0.0),
        (0.0, 10.0, 5.0),
        1.0,
        true,
    );
    let flat = 10.0 * std::f64::consts::FRAC_PI_2;
    assert!((length(&helix) - flat.hypot(5.0)).abs() < 1e-9);
    let no_radius = seg(
        1,
        Motion::ArcCw { center: (3.0, 4.0) },
        (3.0, 4.0, 0.0),
        (6.0, 8.0, 0.0),
        1.0,
        true,
    );
    assert_eq!(length(&no_radius), 5.0);
}

#[test]
fn a_short_move_never_reaches_the_speed_and_a_long_one_cruises() {
    // 600 mm/min = 10 mm/s, 100 mm/s^2: 1 mm of ramp
    let short = move_time(0.1, 600.0, 100.0);
    assert!((short - 2.0 * (0.1_f64 / 100.0).sqrt()).abs() < 1e-12);
    let long = move_time(101.0, 600.0, 100.0);
    assert!((long - (10.1 + 0.1)).abs() < 1e-9);
}

#[test]
fn the_timeline_adds_the_steps_in_order_and_tells_rapids_from_cuts() {
    let t = trace_of(vec![
        seg(
            1,
            Motion::Rapid,
            (0.0, 0.0, 0.0),
            (100.0, 0.0, 0.0),
            0.0,
            false,
        ),
        seg(
            2,
            Motion::Linear,
            (100.0, 0.0, 0.0),
            (100.0, 50.0, 0.0),
            600.0,
            true,
        ),
        seg(
            3,
            Motion::Linear,
            (100.0, 50.0, 0.0),
            (100.0, 60.0, 0.0),
            0.0,
            true,
        ),
    ]);
    let tl = timeline(&t, &Machine::default()).unwrap();
    assert_eq!(tl.steps.len(), 3);
    assert!((tl.steps[0].time_s - 2.1).abs() < 1e-9);
    assert!((tl.steps[1].time_s - 5.02).abs() < 1e-9);
    assert!((tl.steps[2].time_s - 2.01).abs() < 1e-9);
    assert_eq!(
        tl.steps.iter().map(|s| s.rapid).collect::<Vec<_>>(),
        vec![true, false, false]
    );
    assert_eq!(
        tl.steps.iter().map(|s| s.line).collect::<Vec<_>>(),
        vec![1, 2, 3]
    );
    assert_eq!(tl.steps[0].start_s, 0.0);
    assert!((tl.steps[1].start_s - 2.1).abs() < 1e-9);
    assert!((tl.steps[2].start_s - 7.12).abs() < 1e-9);
    assert!((tl.total_s - (tl.rapid_s + tl.cutting_s)).abs() < 1e-9);
    assert!((tl.rapid_s - 2.1).abs() < 1e-9);
}

#[test]
fn the_timeline_of_nothing_is_zero_and_a_machine_that_is_not_valid_is_refused_first() {
    let tl = timeline(&Trace::default(), &Machine::default()).unwrap();
    assert!(tl.steps.is_empty() && tl.total_s == 0.0);
    let bad = Machine {
        accel_mm_s2: 0.0,
        ..Machine::default()
    };
    let e = timeline(&Trace::default(), &bad).unwrap_err();
    assert!(e.contains("accel_mm_s2"), "{e}");
}

#[test]
fn a_move_that_does_not_move_is_a_step_of_no_time() {
    let t = trace_of(vec![cut(1, (1.0, 1.0), (1.0, 1.0), 1.0)]);
    let tl = timeline(&t, &Machine::default()).unwrap();
    assert_eq!(tl.steps.len(), 1);
    assert_eq!(tl.steps[0].time_s, 0.0);
}

#[test]
fn a_wait_is_in_seconds_for_most_dialects_and_in_milliseconds_for_marlin() {
    assert_eq!(dwell_s(&parse("G4 P2.5\n", Dialect::Grbl)), 2.5);
    assert_eq!(dwell_s(&parse("G4 P500\n", Dialect::Marlin)), 0.5);
    assert_eq!(dwell_s(&parse("G4 P1\nG4 P2\n", Dialect::Grbl)), 3.0);
    assert_eq!(dwell_s(&parse("G4\n", Dialect::Grbl)), 0.0);
    assert_eq!(dwell_s(&parse("G1 X1 F100\n", Dialect::Grbl)), 0.0);
}

#[test]
fn the_time_rounds_up_to_the_next_unit_at_the_edges() {
    assert_eq!(format_time(119.5), "2 min 00 s");
    assert_eq!(format_time(3599.6), "1 h 00 min 00 s");
    assert_eq!(format_time(86_400.0), "24 h 00 min 00 s");
}

// ---------------------------------------------------------------- the zoom of the drawing

#[test]
fn the_view_that_fits_a_box_has_the_exact_scale_and_centre() {
    let v = Viewport::fit((0.0, 0.0), (100.0, 50.0), 800.0, 600.0, 20.0);
    assert_eq!((v.centre_x, v.centre_y), (50.0, 25.0));
    assert!((v.px_per_mm - 7.6).abs() < 1e-12);
}

#[test]
fn a_box_that_cannot_be_drawn_gives_the_default_view() {
    let d = Viewport::default();
    for (min, max) in [
        ((f64::NAN, 0.0), (1.0, 1.0)),
        ((0.0, 0.0), (f64::INFINITY, 1.0)),
        ((5.0, 0.0), (1.0, 1.0)),
        ((0.0, 5.0), (1.0, 1.0)),
    ] {
        assert_eq!(Viewport::fit(min, max, 800.0, 600.0, 10.0), d);
    }
}

#[test]
fn a_point_box_is_drawn_at_the_largest_zoom_and_a_huge_margin_at_the_smallest() {
    let point = Viewport::fit((3.0, 3.0), (3.0, 3.0), 800.0, 600.0, 20.0);
    assert_eq!(point.px_per_mm, MAX_SCALE);
    let crowded = Viewport::fit((0.0, 0.0), (100.0, 100.0), 10.0, 10.0, 100.0);
    assert_eq!(crowded.px_per_mm, MIN_SCALE);
}

#[test]
fn the_screen_and_the_board_are_each_the_inverse_of_the_other() {
    let v = Viewport::fit((-10.0, -5.0), (30.0, 15.0), 640.0, 480.0, 16.0);
    for x in [-10.0, 0.0, 12.5, 30.0] {
        for y in [-5.0, 0.0, 7.25, 15.0] {
            let (sx, sy) = v.to_screen(x, y, 640.0, 480.0);
            let (bx, by) = v.to_world(sx, sy, 640.0, 480.0);
            assert!((bx - x).abs() < 1e-9 && (by - y).abs() < 1e-9, "{x},{y}");
        }
    }
}

#[test]
fn zooming_keeps_the_point_under_the_pointer_where_it_was() {
    let mut v = Viewport::default();
    let before = v.to_world(300.0, 200.0, 800.0, 600.0);
    v.zoom_at(300.0, 200.0, 800.0, 600.0, 2.0);
    let after = v.to_world(300.0, 200.0, 800.0, 600.0);
    assert!((before.0 - after.0).abs() < 1e-9 && (before.1 - after.1).abs() < 1e-9);
    assert_eq!(v.px_per_mm, 20.0);
}

#[test]
fn a_zoom_factor_that_makes_no_sense_changes_nothing() {
    for factor in [0.0, -2.0, f64::NAN, f64::INFINITY] {
        let mut v = Viewport::default();
        v.zoom_at(10.0, 10.0, 100.0, 100.0, factor);
        assert_eq!(v, Viewport::default(), "{factor}");
    }
}

#[test]
fn the_zoom_stops_at_both_ends() {
    let mut big = Viewport {
        px_per_mm: 19_000.0,
        ..Viewport::default()
    };
    big.zoom_at(0.0, 0.0, 100.0, 100.0, 10.0);
    assert_eq!(big.px_per_mm, MAX_SCALE);
    let mut small = Viewport {
        px_per_mm: 0.02,
        ..Viewport::default()
    };
    small.zoom_at(0.0, 0.0, 100.0, 100.0, 0.1);
    assert_eq!(small.px_per_mm, MIN_SCALE);
}

#[test]
fn dragging_moves_the_centre_the_other_way_and_dragging_back_returns() {
    let mut v = Viewport::default();
    v.pan_px(100.0, 50.0);
    assert_eq!((v.centre_x, v.centre_y), (-10.0, 5.0));
    v.pan_px(-100.0, -50.0);
    assert_eq!((v.centre_x, v.centre_y), (0.0, 0.0));
}

// ---------------------------------------------------------------- the link with the server

#[test]
fn base64_gives_the_standard_text_of_the_standard_examples() {
    for (plain, coded) in [
        ("", ""),
        ("f", "Zg=="),
        ("fo", "Zm8="),
        ("foo", "Zm9v"),
        ("foob", "Zm9vYg=="),
        ("fooba", "Zm9vYmE="),
        ("foobar", "Zm9vYmFy"),
    ] {
        assert_eq!(base64(plain.as_bytes()), coded, "{plain:?}");
    }
    assert_eq!(base64(&[0xFF, 0xFE, 0xFD]), "//79");
}

#[test]
fn base64_has_the_length_of_four_for_every_three_and_only_its_alphabet() {
    for n in 0..60usize {
        let data: Vec<u8> = (0..n).map(|i| (i * 37 % 256) as u8).collect();
        let text = base64(&data);
        assert_eq!(text.len(), n.div_ceil(3) * 4, "{n}");
        assert!(
            text.bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"+/=".contains(&b)),
            "{n}"
        );
        assert!(text.trim_end_matches('=').len() + 2 >= text.len());
    }
}

#[test]
fn the_name_of_an_artifact_is_the_last_part_of_a_path_trimmed() {
    assert_eq!(artifact_name("a/b\\c.nc").unwrap(), "c.nc");
    assert_eq!(artifact_name("  plain.nc ").unwrap(), "plain.nc");
    assert_eq!(artifact_name(&"x".repeat(120)).unwrap().len(), 120);
}

#[test]
fn a_name_that_is_empty_too_long_or_with_a_control_character_is_refused_naming_the_input() {
    for bad in ["", "   ", "dir/", "dir\\", "a\u{7}b", "a\tb"] {
        let e = artifact_name(bad).unwrap_err();
        assert!(
            e.contains(&format!("'{bad}'")) && e.contains("cannot be the name"),
            "{bad:?}: {e}"
        );
    }
    assert!(artifact_name(&"x".repeat(121)).is_err());
    assert!(artifact_name(&"é".repeat(121)).is_err());
    assert!(artifact_name(&"é".repeat(120)).is_ok());
}

#[test]
fn the_media_type_comes_from_the_extension_in_any_letter_case() {
    for (file, kind) in [
        ("a.NC", "text/x-gcode"),
        ("a.gcode", "text/x-gcode"),
        ("a.plt", "application/vnd.hp-hpgl"),
        ("a.DXF", "image/vnd.dxf"),
        ("a.drl", "application/x-excellon"),
        ("a.GTL", "application/x-gerber"),
        ("a.svg", "image/svg+xml"),
        ("a.png", "image/png"),
        ("a.json", "application/json"),
        ("a.csv", "text/csv"),
        ("a.log", "text/plain"),
        ("a.zip", "application/octet-stream"),
        ("noextension", "application/octet-stream"),
        ("dir.nc/file", "application/octet-stream"),
        ("C:\\x\\y.svg", "image/svg+xml"),
    ] {
        assert_eq!(media_type(file), kind, "{file}");
    }
}

#[test]
fn the_request_of_an_artifact_has_the_name_the_format_the_data_the_producer_and_the_parameters() {
    let body = artifact_request("p.nc", "gcode", b"foo", &[("a", "1"), ("b", "x\"y")]);
    assert!(body.starts_with("{\"name\":\"p.nc\",\"format\":\"gcode\",\"data_base64\":\"Zm9v\""));
    assert!(body.contains(&format!("\"producer\":{{\"name\":\"eh3d-{APP}\"")));
    assert!(body.ends_with("\"params\":{\"a\":\"1\",\"b\":\"x\\\"y\"}}"));
    assert_eq!(
        body,
        artifact_request("p.nc", "gcode", b"foo", &[("a", "1"), ("b", "x\"y")])
    );
}

#[test]
fn a_request_without_parameters_has_an_empty_object_and_an_empty_file_has_empty_data() {
    let body = artifact_request("e.txt", "text", b"", &[]);
    assert!(body.contains("\"data_base64\":\"\""));
    assert!(body.ends_with("\"params\":{}}"));
}

#[test]
fn the_line_of_a_kept_artifact_names_everything_the_server_answered() {
    let sent = Sent {
        artifact_id: "art-1".into(),
        blob_sha256: "ab".repeat(32),
        size: 42,
    };
    assert_eq!(
        sent_line("p.nc", &sent),
        format!(
            "kept on the server: p.nc as art-1 (42 bytes, sha256 {})",
            "ab".repeat(32)
        )
    );
    assert_eq!(MAX_SEND_BYTES, 8 * 1024 * 1024);
}

#[test]
fn the_server_mode_needs_an_address_and_a_token_and_says_which_is_missing() {
    let mut s = Settings::default();
    assert!(require(&s).unwrap_err().contains("server mode is off"));
    s.set("server_address", "127.0.0.1:9").unwrap();
    assert!(require(&s).unwrap_err().contains("server_token is empty"));
    s.set("server_token", "secret-token").unwrap();
    assert!(require(&s).is_ok());
}

#[test]
fn the_keys_of_the_link_are_known_in_any_letter_case_and_the_others_are_not() {
    for key in [
        "server_address",
        " SERVER_TOKEN ",
        "Server_Machine",
        "server_timeout_s",
        "server_retries",
        "server_mdns",
        "server_max_answer_mib",
    ] {
        assert!(is_server_key(key), "{key}");
    }
    for key in ["", "server", "address", "language"] {
        assert!(!is_server_key(key), "{key}");
    }
}

#[test]
fn the_values_of_the_link_are_checked_at_both_edges_of_their_ranges() {
    for (key, ok, bad) in [
        ("server_timeout_s", ["1", "120"], ["0", "121"]),
        ("server_retries", ["0", "5"], ["-1", "6"]),
        ("server_max_answer_mib", ["1", "256"], ["0", "257"]),
    ] {
        for value in ok {
            assert!(check_key(key, value).is_ok(), "{key}={value}");
        }
        for value in bad {
            assert!(check_key(key, value).is_err(), "{key}={value}");
        }
    }
    assert!(check_key("server_mdns", "maybe").is_err());
    assert!(check_key("server_address", "has space").is_err());
}

// ---------------------------------------------------------------- the window

#[test]
fn a_size_is_two_sides_inside_the_limits_of_a_picture() {
    assert_eq!(parse_size("320x320"), Some((320, 320)));
    assert_eq!(parse_size(" 8192 X 8192 "), Some((8192, 8192)));
    for text in ["319x500", "500x8193", "1280", "axb", "-5x500", "x720", ""] {
        assert_eq!(parse_size(text), None, "{text:?}");
    }
}

#[test]
fn a_picture_has_the_defaults_of_this_program() {
    let p = parse_window_args(&strings(&["--shot", "x.png"]), 1).unwrap();
    let shot = p.shot.unwrap();
    assert_eq!(
        (shot.size, shot.wait, shot.theme),
        (DEFAULT_SIZE, DEFAULT_WAIT, ShotTheme::Forge)
    );
    assert_eq!(DEFAULT_SIZE, (1280, 780));
    assert!(wants_picture(&strings(&["a.nc", "--shot", "x.png"])));
}

#[test]
fn the_options_of_a_picture_without_the_picture_are_refused_and_the_files_are_limited() {
    for opt in [
        &["--size", "800x600"][..],
        &["--theme", "light"],
        &["--wait", "9"],
        &["--replace"],
    ] {
        assert!(parse_window_args(&strings(opt), 1).is_err(), "{opt:?}");
    }
    assert_eq!(
        parse_window_args(&strings(&["a.nc", "b.nc"]), 1).unwrap_err(),
        "only one file can be opened"
    );
    assert_eq!(
        parse_window_args(&strings(&["--bogus"]), 1).unwrap_err(),
        "unknown option --bogus"
    );
}

#[test]
fn the_wait_and_the_theme_of_a_picture_are_bounded() {
    let ok = parse_window_args(
        &strings(&["--shot", "x.png", "--wait", "3600", "--theme", "DARK"]),
        1,
    )
    .unwrap();
    let shot = ok.shot.unwrap();
    assert_eq!((shot.wait, shot.theme), (3600, ShotTheme::Dark));
    for bad in [
        &["--wait", "3601"][..],
        &["--wait", "-1"],
        &["--theme", "blue"],
    ] {
        let mut args = vec!["--shot", "x.png"];
        args.extend_from_slice(bad);
        assert!(parse_window_args(&strings(&args), 1).is_err(), "{bad:?}");
    }
}

#[test]
fn the_browser_filters_by_extension_and_goes_up_and_down() {
    assert!(matches_extension("a.NC", &["nc"]) && !matches_extension(".nc", &["nc"]));
    assert!(matches_extension("anything", &[]));
    let dir = temp("browser");
    fs::create_dir(dir.join("sub")).unwrap();
    fs::write(dir.join("a.nc"), b"x").unwrap();
    fs::write(dir.join("b.txt"), b"x").unwrap();
    let mut b = Browser::new(dir.clone(), &["nc"]);
    let names: Vec<&str> = b.entries.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(names, vec!["sub", "a.nc"]);
    assert_eq!(b.enter("sub"), None);
    assert_eq!(b.dir, dir.join("sub"));
    b.up();
    assert_eq!(b.dir, dir);
    assert_eq!(b.enter("a.nc"), Some(dir.join("a.nc")));
}

#[test]
fn a_typed_path_that_is_not_there_is_an_error_in_the_browser_and_a_missing_folder_has_no_entries() {
    let dir = temp("browser-typed");
    let mut b = Browser::new(dir.clone(), &[]);
    b.path_text = dir.join("nothing").display().to_string();
    assert_eq!(b.go_typed(), None);
    assert!(b.error.as_deref().unwrap().ends_with("not found"));
    let gone = Browser::new(dir.join("nope"), &[]);
    assert!(gone.error.is_some() && gone.entries.is_empty());
}

#[test]
fn a_size_is_written_with_the_edges_exact() {
    assert_eq!(size_text(1023), "1023 B");
    assert_eq!(size_text(1024), "1.0 KiB");
    assert_eq!(size_text(1_048_575), "1024.0 KiB");
    assert_eq!(size_text(1_048_576), "1.0 MiB");
}

#[test]
fn the_eight_languages_are_in_a_fixed_order_with_distinct_names() {
    let codes: Vec<&str> = Lang::ALL.iter().map(|l| l.code()).collect();
    assert_eq!(codes, vec!["en", "es", "de", "fr", "it", "ja", "zh", "pt"]);
    let mut names: Vec<&str> = Lang::ALL.iter().map(|l| l.name()).collect();
    names.sort();
    names.dedup();
    assert_eq!(names.len(), 8);
    assert_eq!(Lang::from_code("PT_br"), Some(Lang::Pt));
    assert_eq!(Lang::from_code("e"), None);
}

#[test]
fn a_template_is_filled_by_place_and_the_sentences_differ_between_languages() {
    assert_eq!(fill("{0}{0}-{1}", &["a", "b"]), "aa-b");
    assert_eq!(fill("{1}", &["a"]), "{1}");
    let differs = |m: Msg| {
        Lang::ALL
            .iter()
            .skip(1)
            .filter(|l| m.tr(**l) != m.tr(Lang::En))
            .count()
    };
    let total: usize = Msg::ALL.iter().map(|m| differs(*m)).sum();
    assert!(total > Msg::ALL.len() * 3, "{total}");
}
