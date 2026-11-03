use eh3d_cam_debugger::checks::{check, has_alerts, Clamp, Level, Limits};
use eh3d_cam_debugger::timeline::{format_time, length, move_time, timeline, Machine};
use eh3d_parser_gcode::{interpret, parse, Dialect, Trace};

fn trace(text: &str) -> Trace {
    interpret(&parse(text, Dialect::Grbl))
}

#[test]
fn a_move_takes_the_time_of_a_trapezoid() {
    // 10 mm/s with 100 mm/s^2 needs 0.5 mm to reach speed: a 100 mm move is 10 s of cruise plus 0.1 s of ramp
    assert!((move_time(100.0, 600.0, 100.0) - 10.1).abs() < 1e-9);
    // a move too short to reach the speed is a triangle: 2 * sqrt(L / a)
    assert!((move_time(0.04, 600.0, 100.0) - 2.0 * (0.04_f64 / 100.0).sqrt()).abs() < 1e-12);
    assert_eq!(move_time(0.0, 600.0, 100.0), 0.0);
    assert!(move_time(1.0, 600.0, 100.0) > move_time(0.5, 600.0, 100.0));
}

#[test]
fn a_program_has_a_time_split_in_cutting_and_rapid() {
    let t = trace("G21 G90\nG0 X0 Y0\nG0 X100 Y0\nG1 X100 Y100 F600\nM2\n");
    let m = Machine {
        accel_mm_s2: 100.0,
        rapid_mm_min: 6000.0,
        default_feed_mm_min: 300.0,
    };
    let tl = timeline(&t, &m).unwrap();
    assert_eq!(tl.steps.len(), 3);
    assert_eq!(tl.steps[0].time_s, 0.0, "the first rapid goes nowhere");
    assert!(
        tl.cutting_s > tl.rapid_s,
        "100 mm at 10 mm/s against 100 mm at 100 mm/s"
    );
    assert!((tl.total_s - tl.cutting_s - tl.rapid_s).abs() < 1e-9);
    assert!((tl.steps[2].start_s - tl.steps[1].time_s).abs() < 1e-9);
}

#[test]
fn a_cut_with_no_feed_uses_the_default_feed() {
    let t = trace("G1 X60\n");
    let slow = timeline(
        &t,
        &Machine {
            default_feed_mm_min: 60.0,
            ..Machine::default()
        },
    )
    .unwrap();
    let fast = timeline(
        &t,
        &Machine {
            default_feed_mm_min: 6000.0,
            ..Machine::default()
        },
    )
    .unwrap();
    assert!(slow.total_s > fast.total_s);
}

#[test]
fn the_length_of_an_arc_is_the_length_of_the_arc() {
    let t = trace("G17 G21 G90\nG0 X10 Y0\nG3 X0 Y10 I-10 J0 F300\nM2\n");
    let arc = t
        .segments
        .iter()
        .find(|s| matches!(s.motion, eh3d_parser_gcode::Motion::ArcCcw { .. }))
        .unwrap();
    assert!(
        (length(arc) - 10.0 * std::f64::consts::FRAC_PI_2).abs() < 1e-6,
        "{}",
        length(arc)
    );
    let full = trace("G21 G90\nG0 X10 Y0\nG2 X10 Y0 I-10 J0 F300\nM2\n");
    let circle = full
        .segments
        .iter()
        .find(|s| matches!(s.motion, eh3d_parser_gcode::Motion::ArcCw { .. }))
        .unwrap();
    assert!(
        (length(circle) - 20.0 * std::f64::consts::PI).abs() < 1e-6,
        "a full circle: {}",
        length(circle)
    );
}

#[test]
fn a_machine_that_cannot_move_is_refused() {
    let t = trace("G1 X1 F100\n");
    for bad in [0.0, -5.0, f64::NAN, f64::INFINITY] {
        assert!(timeline(
            &t,
            &Machine {
                accel_mm_s2: bad,
                ..Machine::default()
            }
        )
        .is_err());
        assert!(timeline(
            &t,
            &Machine {
                rapid_mm_min: bad,
                ..Machine::default()
            }
        )
        .is_err());
    }
}

#[test]
fn times_are_written_in_the_largest_units() {
    assert_eq!(format_time(12.54), "12.5 s");
    assert_eq!(format_time(200.0), "3 min 20 s");
    assert_eq!(format_time(3725.0), "1 h 02 min 05 s");
    assert_eq!(format_time(f64::NAN), "-");
    assert_eq!(format_time(-1.0), "-");
}

fn alerts(text: &str, limits: &Limits) -> Vec<String> {
    check(&trace(text), limits)
        .into_iter()
        .filter(|f| f.level == Level::Alert)
        .map(|f| f.message)
        .collect()
}

#[test]
fn a_rapid_move_into_the_material_is_an_alert() {
    let bad = alerts("G21 G90\nG0 X0 Y0 Z2\nG0 Z-1\nM2\n", &Limits::default());
    assert_eq!(bad.len(), 1, "{bad:?}");
    assert!(bad[0].contains("rapid"));
    assert!(
        alerts(
            "G21 G90\nG0 X0 Y0 Z2\nG1 Z-1 F60\nG1 X10 F200\nG0 Z2\nM2\n",
            &Limits::default()
        )
        .is_empty(),
        "a plunge with G1 and a lift with G0 are fine"
    );
    assert_eq!(
        alerts("G21 G90\nG0 Z-1\nG0 X10\n", &Limits::default()).len(),
        2,
        "sliding sideways below the surface with G0 is an alert too"
    );
}

#[test]
fn a_cut_with_the_spindle_off_and_a_cut_with_no_feed_are_warnings() {
    let f = check(&trace("G21 G90\nG0 Z2\nG1 Z-1\nM2\n"), &Limits::default());
    assert!(f.iter().any(|f| f.message.contains("spindle off")), "{f:?}");
    assert!(f.iter().any(|f| f.message.contains("no feed")), "{f:?}");
    assert!(!has_alerts(&f));
    let on = check(
        &trace("G21 G90\nM3 S10000\nG1 Z-1 F100\nM2\n"),
        &Limits::default(),
    );
    assert!(!on.iter().any(|f| f.message.contains("spindle")));
}

#[test]
fn a_move_outside_the_table_and_below_the_lowest_z_are_alerts() {
    let limits = Limits {
        table: Some((100.0, 80.0)),
        min_z: Some(-5.0),
        ..Limits::default()
    };
    let a = alerts(
        "G21 G90\nM3\nG1 X50 Y40 F100\nG1 X120 Y40\nG1 X50 Y-1\n",
        &limits,
    );
    assert_eq!(a.len(), 2, "{a:?}");
    assert!(a.iter().all(|m| m.contains("outside the table")));
    let z = alerts("G21 G90\nM3\nG1 Z-6 F50\n", &limits);
    assert!(z.iter().any(|m| m.contains("lowest Z")), "{z:?}");
    assert!(
        alerts("G21 G90\nM3\nG1 X100 Y80 F100\n", &limits).is_empty(),
        "the edge of the table is inside it"
    );
}

#[test]
fn the_feed_limit_is_a_warning() {
    let f = check(
        &trace("G21 G90\nM3\nG1 X10 F5000\n"),
        &Limits {
            max_feed_mm_min: Some(1000.0),
            ..Limits::default()
        },
    );
    assert!(f
        .iter()
        .any(|f| f.level == Level::Warning && f.message.contains("5000")));
}

#[test]
fn a_head_over_a_clamp_is_an_alert_only_below_its_height() {
    let clamp = Clamp {
        x0: 40.0,
        y0: -5.0,
        x1: 50.0,
        y1: 5.0,
        height_mm: 6.0,
    };
    let limits = Limits {
        clamps: vec![clamp],
        ..Limits::default()
    };
    let low = alerts("G21 G90\nM3\nG0 X0 Y0 Z3\nG0 X100 Y0\n", &limits);
    assert_eq!(low.len(), 1, "{low:?}");
    assert!(low[0].contains("clamp 1"));
    assert!(alerts("G21 G90\nM3\nG0 X0 Y0 Z8\nG0 X100 Y0\n", &limits).is_empty());
    assert!(
        alerts("G21 G90\nM3\nG0 X0 Y20 Z3\nG0 X100 Y20\n", &limits).is_empty(),
        "passing beside the clamp is fine"
    );
}

#[test]
fn what_the_reader_could_not_understand_is_reported() {
    let f = check(&trace("G21 G90\nG41 D1\nG1 X5 F100\n"), &Limits::default());
    assert!(
        f.iter()
            .any(|f| f.level == Level::Alert && f.message.contains("could not understand")),
        "{f:?}"
    );
    assert!(check(&trace(""), &Limits::default()).is_empty());
}

#[test]
fn garbage_does_not_panic() {
    for text in [
        "\u{0}\u{1}",
        "G1 X1e999",
        "G2 X1 Y1 I0 J0",
        "G91 G1 X1e300 F1",
        "M3 S99999999999999999999",
    ] {
        let t = trace(text);
        let _ = timeline(&t, &Machine::default());
        let _ = check(
            &t,
            &Limits {
                table: Some((1.0, 1.0)),
                clamps: vec![Clamp {
                    x0: 0.0,
                    y0: 0.0,
                    x1: 1e9,
                    y1: 1e9,
                    height_mm: 1.0,
                }],
                ..Limits::default()
            },
        );
    }
}

#[test]
fn an_arc_is_checked_along_the_arc_and_not_along_its_chord() {
    // from (20,10) to (30,10) round the centre (25,10), clockwise: it passes through (25,15)
    let t = trace("G21 G90\nG0 X20 Y10 Z5\nG2 X30 Y10 I5 J0 F100\nM2\n");
    let clamp = Clamp {
        x0: 24.0,
        y0: 12.0,
        x1: 26.0,
        y1: 20.0,
        height_mm: 10.0,
    };
    let l = Limits {
        surface_z: -1.0,
        clamps: vec![clamp],
        ..Limits::default()
    };
    let f = check(&t, &l);
    assert!(
        f.iter()
            .any(|f| f.level == Level::Alert && f.message.contains("clamp")),
        "{f:?}"
    );
    // both ends are inside the table, the arc is not
    let l = Limits {
        surface_z: -1.0,
        table: Some((40.0, 12.0)),
        ..Limits::default()
    };
    let f = check(&t, &l);
    assert!(
        f.iter().any(|f| f.message.contains("outside the table")),
        "{f:?}"
    );
}

#[test]
fn the_codes_that_change_the_origin_are_never_reported_as_clean() {
    for code in [
        "G92 X0 Y0 Z0",
        "G53 G0 Z-1",
        "G28 X0 Y0",
        "G30",
        "G10 L20 P1 X0 Y0 Z0",
    ] {
        let t = trace(&format!(
            "G21 G90\nG0 X1 Y1 Z5\n{code}\nG1 X5 Y5 F100\nM2\n"
        ));
        assert!(!check(&t, &Limits::default()).is_empty(), "{code}");
    }
}

#[test]
fn strict_makes_the_warnings_alerts() {
    let t = trace("G21 G90\nG1 Z-1 F100\nG1 X20\nM2\n");
    let mut f = check(&t, &Limits::default());
    assert!(!f.is_empty() && !has_alerts(&f));
    eh3d_cam_debugger::checks::strict(&mut f);
    assert!(has_alerts(&f));
}

// ---- more edges: exact limits, arcs, ordering of the findings, time formatting and the command line ----

use std::process::Command;

fn findings(text: &str, limits: &Limits) -> Vec<(usize, Level, String)> {
    check(&trace(text), limits)
        .into_iter()
        .map(|f| (f.line, f.level, f.message))
        .collect()
}

#[test]
fn a_cut_exactly_at_the_surface_or_the_limits_is_not_reported() {
    let limits = Limits {
        min_z: Some(-2.0),
        max_feed_mm_min: Some(300.0),
        table: Some((50.0, 50.0)),
        ..Limits::default()
    };
    // Z0 is the surface, Z-2 the lowest Z, F300 the feed limit and (50, 50) the corner of the table
    assert!(findings(
        "G21 G90\nM3 S1000\nG1 Z0 F300\nG1 Z-2\nG1 X50 Y50 Z-2\nM2\n",
        &limits
    )
    .is_empty());
    // one hundredth beyond each of them is reported
    for (program, word) in [
        ("G21 G90\nM3\nG1 Z-2.01 F300\n", "lowest Z"),
        ("G21 G90\nM3\nG1 X10 F300.01\n", "above the limit"),
        ("G21 G90\nM3\nG1 X50.01 F300\n", "outside the table"),
        ("G21 G90\nM3\nG1 Y-0.01 F300\n", "outside the table"),
    ] {
        let f = findings(program, &limits);
        assert!(
            f.iter().any(|(_, _, m)| m.contains(word)),
            "{program:?}: {f:?}"
        );
    }
}

#[test]
fn the_surface_moves_the_line_between_air_and_material() {
    let deep = Limits {
        surface_z: -1.0,
        ..Limits::default()
    };
    // at Z-0.5 the tool is still above a surface at Z-1: a rapid there is fine and the cut needs no spindle warning
    assert!(findings("G21 G90\nG0 Z-0.5\nG1 Z-0.5 F100\nG0 X10\n", &deep).is_empty());
    let f = findings("G21 G90\nG0 Z-1.5\n", &deep);
    assert!(
        f.iter()
            .any(|(_, l, m)| *l == Level::Alert && m.contains("rapid")),
        "{f:?}"
    );
}

#[test]
fn the_feed_stays_from_one_move_to_the_next_until_the_program_changes_it() {
    let f = findings(
        "G21 G90\nM3\nG1 X10 F200\nG1 X20\nG1 X30 F0\n",
        &Limits::default(),
    );
    assert!(
        !f.is_empty() && f.iter().all(|x| x.0 == 5),
        "only the move where the feed was set to nothing has none: {f:?}"
    );
    assert!(f.iter().any(|x| x.2.contains("no feed")));
}

#[test]
fn a_rapid_never_complains_about_its_feed_and_a_lift_out_of_the_material_is_fine() {
    let limits = Limits {
        max_feed_mm_min: Some(100.0),
        ..Limits::default()
    };
    assert!(findings("G21 G90\nM3\nG1 Z-1 F50\nG0 Z5\nG0 X10\n", &limits).is_empty());
}

#[test]
fn a_clamp_given_with_its_corners_the_wrong_way_round_is_the_same_clamp() {
    let a = Clamp {
        x0: 40.0,
        y0: -5.0,
        x1: 50.0,
        y1: 5.0,
        height_mm: 6.0,
    };
    let b = Clamp {
        x0: 50.0,
        y0: 5.0,
        x1: 40.0,
        y1: -5.0,
        height_mm: 6.0,
    };
    let program = "G21 G90\nM3\nG0 X0 Y0 Z3\nG0 X100 Y0\n";
    let with = |c: Clamp| {
        findings(
            program,
            &Limits {
                clamps: vec![c],
                ..Limits::default()
            },
        )
    };
    assert_eq!(with(a), with(b));
    assert_eq!(with(a).len(), 1);
}

#[test]
fn passing_at_exactly_the_height_of_a_clamp_is_clear_and_below_it_is_not() {
    let c = Clamp {
        x0: 40.0,
        y0: -5.0,
        x1: 50.0,
        y1: 5.0,
        height_mm: 6.0,
    };
    let l = Limits {
        clamps: vec![c],
        ..Limits::default()
    };
    assert!(findings("G21 G90\nM3\nG0 X0 Y0 Z6\nG0 X100 Y0\n", &l).is_empty());
    assert_eq!(
        findings("G21 G90\nM3\nG0 X0 Y0 Z5.99\nG0 X100 Y0\n", &l).len(),
        1
    );
}

#[test]
fn two_clamps_are_numbered_and_the_one_that_is_hit_is_named() {
    let l = Limits {
        clamps: vec![
            Clamp {
                x0: 10.0,
                y0: 40.0,
                x1: 20.0,
                y1: 50.0,
                height_mm: 5.0,
            },
            Clamp {
                x0: 60.0,
                y0: -5.0,
                x1: 70.0,
                y1: 5.0,
                height_mm: 5.0,
            },
        ],
        ..Limits::default()
    };
    let f = findings("G21 G90\nM3\nG0 X0 Y0 Z2\nG0 X100 Y0\n", &l);
    assert_eq!(f.len(), 1, "{f:?}");
    assert!(f[0].2.contains("clamp 2"), "{f:?}");
}

#[test]
fn a_thin_clamp_between_two_samples_of_a_long_move_is_still_found() {
    let c = Clamp {
        x0: 40.1,
        y0: -5.0,
        x1: 40.4,
        y1: 5.0,
        height_mm: 6.0,
    };
    let l = Limits {
        clamps: vec![c],
        ..Limits::default()
    };
    assert_eq!(
        findings("G21 G90\nM3\nG0 X0 Y0 Z3\nG0 X100 Y0\n", &l).len(),
        1
    );
}

#[test]
fn the_findings_come_in_the_order_of_the_program_even_when_the_reader_speaks_later() {
    // line 2 is a cut with the spindle off; line 4 is a code the reader does not know
    let f = findings(
        "G21 G90\nG1 Z-1 F100\nG1 X5\nG123\nM2\n",
        &Limits::default(),
    );
    assert!(f.len() >= 2, "{f:?}");
    assert!(
        f.windows(2).all(|w| w[0].0 <= w[1].0),
        "not in program order: {f:?}"
    );
}

#[test]
fn the_same_program_checked_twice_gives_the_same_findings() {
    let text = "G21 G90\nG0 X0 Y0 Z-1\nG1 X100 F5000\nG123\nG2 X0 Y0 I50 J0\nM2\n";
    let l = Limits {
        table: Some((60.0, 60.0)),
        max_feed_mm_min: Some(1000.0),
        min_z: Some(-0.5),
        ..Limits::default()
    };
    assert_eq!(findings(text, &l), findings(text, &l));
}

#[test]
fn an_ignored_code_hides_only_the_notices_of_that_code() {
    let text = "G21 G90\nG123\nG1 Z-1 F100\n";
    let all = findings(text, &Limits::default());
    let hidden = findings(
        text,
        &Limits {
            ignore_codes: vec!["unknown-g-code".into()],
            ..Limits::default()
        },
    );
    assert!(all.iter().any(|(_, _, m)| m.contains("unknown-g-code")));
    assert!(!hidden.iter().any(|(_, _, m)| m.contains("unknown-g-code")));
    assert!(
        hidden.iter().any(|(_, _, m)| m.contains("spindle off")),
        "the checks of the program are not notices of the reader"
    );
}

#[test]
fn the_time_of_a_move_grows_with_the_length_and_falls_with_the_speed_and_the_acceleration() {
    let mut last = 0.0;
    for k in 1..=200 {
        let t = move_time(f64::from(k) * 0.25, 1200.0, 200.0);
        assert!(t > last, "length {k}");
        last = t;
    }
    for pair in [100.0, 200.0, 600.0, 1200.0, 6000.0].windows(2) {
        assert!(move_time(80.0, pair[1], 300.0) < move_time(80.0, pair[0], 300.0));
    }
    for pair in [50.0, 100.0, 500.0, 5000.0].windows(2) {
        assert!(move_time(80.0, 900.0, pair[1]) < move_time(80.0, 900.0, pair[0]));
    }
}

#[test]
fn the_trapezoid_and_the_triangle_meet_where_the_speed_is_just_reached() {
    // 600 mm/min is 10 mm/s; with 50 mm/s^2 the ramp is 2 mm
    let at = move_time(2.0, 600.0, 50.0);
    assert!((at - 0.4).abs() < 1e-9, "{at}");
    assert!((move_time(2.0 - 1e-9, 600.0, 50.0) - at).abs() < 1e-6);
    assert!((move_time(2.0 + 1e-9, 600.0, 50.0) - at).abs() < 1e-6);
    assert_eq!(move_time(-3.0, 600.0, 50.0), 0.0);
}

#[test]
fn the_steps_add_up_to_the_total_and_start_where_the_one_before_ended() {
    let t = trace("G21 G90\nG0 X30 Y40\nG1 Z-1 F120\nG1 X60 F240\nG2 X30 Y40 I-15 J0\nG0 Z5\nM2\n");
    let tl = timeline(&t, &Machine::default()).unwrap();
    assert!(tl.steps.len() >= 5);
    let mut at = 0.0;
    for s in &tl.steps {
        assert!((s.start_s - at).abs() < 1e-9);
        at += s.time_s;
    }
    assert!(
        (at - tl.total_s).abs() < 1e-9 && (tl.cutting_s + tl.rapid_s - tl.total_s).abs() < 1e-9
    );
    assert!(tl.steps.windows(2).all(|w| w[0].line <= w[1].line));
}

#[test]
fn a_clockwise_quarter_turn_the_long_way_round_is_three_quarters_of_the_circle() {
    let t = trace("G17 G21 G90\nG0 X10 Y0\nG2 X0 Y10 I-10 J0 F300\nM2\n");
    let arc = t
        .segments
        .iter()
        .find(|s| matches!(s.motion, eh3d_parser_gcode::Motion::ArcCw { .. }))
        .unwrap();
    assert!(
        (length(arc) - 15.0 * std::f64::consts::PI).abs() < 1e-6,
        "{}",
        length(arc)
    );
}

#[test]
fn a_helix_is_longer_than_its_flat_arc_by_the_climb() {
    let flat = trace("G17 G21 G90\nG0 X10 Y0\nG3 X0 Y10 I-10 J0 F300\nM2\n");
    let helix = trace("G17 G21 G90\nG0 X10 Y0\nG3 X0 Y10 Z-8 I-10 J0 F300\nM2\n");
    let pick = |t: &Trace| {
        t.segments
            .iter()
            .find(|s| matches!(s.motion, eh3d_parser_gcode::Motion::ArcCcw { .. }))
            .map(length)
            .unwrap()
    };
    let (a, b) = (pick(&flat), pick(&helix));
    assert!((b - a.hypot(8.0)).abs() < 1e-6, "{a} {b}");
}

#[test]
fn a_machine_with_no_default_feed_is_refused_and_the_message_names_the_field() {
    let t = trace("G1 X1\n");
    for (m, name) in [
        (
            Machine {
                default_feed_mm_min: 0.0,
                ..Machine::default()
            },
            "default_feed_mm_min",
        ),
        (
            Machine {
                accel_mm_s2: f64::NAN,
                ..Machine::default()
            },
            "accel_mm_s2",
        ),
        (
            Machine {
                rapid_mm_min: -1.0,
                ..Machine::default()
            },
            "rapid_mm_min",
        ),
    ] {
        assert!(timeline(&t, &m).unwrap_err().contains(name));
    }
}

#[test]
fn the_time_text_rolls_over_cleanly_at_the_edges_of_a_minute_and_an_hour() {
    assert_eq!(format_time(0.0), "0.0 s");
    assert_eq!(format_time(59.9), "59.9 s");
    assert_eq!(
        format_time(59.96),
        "1 min 00 s",
        "59.96 s must not be written as 60.0 s"
    );
    assert_eq!(format_time(60.0), "1 min 00 s");
    assert_eq!(format_time(3599.4), "59 min 59 s");
    assert_eq!(format_time(3599.6), "1 h 00 min 00 s");
    assert_eq!(format_time(f64::INFINITY), "-");
    assert!(!format_time(1.0e30).is_empty());
}

fn camdebug(args: &[&str]) -> (Option<i32>, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_eh3d-camdebug"))
        .args(args)
        .output()
        .unwrap();
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn program_file(name: &str, text: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("eh3d_camdebug_more_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join(name);
    std::fs::write(&file, text).unwrap();
    file
}

#[test]
fn the_exit_code_is_zero_for_a_clean_program_one_for_an_alert_and_for_a_failure() {
    let clean = program_file("clean.nc", "G21 G90\nM3 S1000\nG1 X10 F100\nM2\n");
    let alert = program_file("alert.nc", "G21 G90\nG0 Z-1\nM2\n");
    let (c, o, _) = camdebug(&["check", clean.to_str().unwrap()]);
    assert_eq!(c, Some(0), "{o}");
    assert!(o.contains("no findings"));
    let (c, o, _) = camdebug(&["check", alert.to_str().unwrap()]);
    assert_eq!(c, Some(1), "{o}");
    assert!(o.contains("ALERT") && o.contains("line 2"));
    // a file that cannot be read is a failure
    let (c, _, e) = camdebug(&["check", "none-such-file.nc"]);
    assert_eq!(c, Some(1));
    assert!(e.contains("none-such-file.nc"));
}

#[test]
fn a_warning_is_exit_code_two_and_strict_turns_it_into_one() {
    let f = program_file("warn.nc", "G21 G90\nG1 Z-1 F100\nM2\n");
    let path = f.to_str().unwrap();
    assert_eq!(camdebug(&["check", path]).0, Some(2));
    let (c, o, _) = camdebug(&["check", path, "--strict"]);
    assert_eq!(c, Some(1));
    assert!(o.contains("ALERT") && !o.contains("warning line"), "{o}");
    assert_eq!(
        camdebug(&["check", "--strict", path]).0,
        Some(1),
        "the file comes first: --strict is not a file"
    );
}

#[test]
fn bad_options_are_mistakes_that_name_the_option() {
    let f = program_file("opts.nc", "G21 G90\nG1 X1 F100\n");
    let p = f.to_str().unwrap();
    for (args, word) in [
        (vec!["check", p, "--table", "100"], "--table"),
        (vec!["check", p, "--table", "100,abc"], "--table"),
        (vec!["check", p, "--clamp", "1,2,3,4"], "--clamp"),
        (vec!["check", p, "--dialect", "fanuc"], "fanuc"),
        (vec!["check", p, "--accel", "0"], "accel_mm_s2"),
        (vec!["check", p, "--feed", "-5"], "default_feed_mm_min"),
        (vec!["check", p, "--rapid"], "needs a value"),
        (vec!["check", p, "--powr", "5"], "--powr"),
        (vec!["check", p, "--max-feed", "fast"], "--max-feed"),
        (vec!["check"], "needs a G-code file"),
    ] {
        let (c, _, e) = camdebug(&args);
        assert_eq!(c, Some(64), "{args:?}");
        assert!(e.contains(word), "{args:?}: {e}");
    }
    // an unknown command is refused before the file is read
    assert_eq!(camdebug(&["frobnicate", p]).0, Some(64));
    let (c, _, e) = camdebug(&["frobnicate", "no-such-file.nc"]);
    assert_eq!(c, Some(64));
    assert!(e.contains("unknown command"), "{e}");
}

#[test]
fn the_steps_command_lists_every_move_and_the_dialect_changes_what_is_understood() {
    let f = program_file("steps.nc", "G21 G90\nG0 X10 Y0\nG1 X20 F600\nG1 Z-1\nM2\n");
    let (c, o, _) = camdebug(&["steps", f.to_str().unwrap(), "--dialect", "marlin"]);
    assert_eq!(c, Some(0), "{o}");
    assert_eq!(o.lines().count(), 3, "{o}");
    assert!(
        o.lines().next().unwrap().contains("rapid") && o.lines().nth(1).unwrap().contains("cut")
    );
    let (c, _, _) = camdebug(&["steps", f.to_str().unwrap(), "--dialect", "PLANETCNC"]);
    assert_eq!(c, Some(0), "the name of the dialect is not case sensitive");
}

#[test]
fn a_huge_or_negative_size_limit_is_an_error_or_accepted_but_never_a_crash() {
    let f = program_file("limit.nc", "G21 G90\nG1 X1 F100\n");
    let p = f.to_str().unwrap();
    for v in ["-5", "0", "1e30", "18446744073709551616", "2.5"] {
        let (c, _, e) = camdebug(&["check", p, "--max-bytes", v]);
        assert!(
            matches!(c, Some(0) | Some(1) | Some(64)),
            "--max-bytes {v}: {c:?} {e}"
        );
    }
    assert_eq!(
        camdebug(&["check", p, "--max-bytes", "0"]).0,
        Some(1),
        "a file bigger than zero bytes is above the limit"
    );
}

#[test]
fn a_program_with_non_ascii_comments_and_a_byte_order_mark_is_checked() {
    let f = program_file(
        "utf8.nc",
        "\u{feff}(fabricación ñandú 日本語)\nG21 G90\nM3 S1000\nG1 X5 F100\nM2\n",
    );
    let (c, o, e) = camdebug(&["check", f.to_str().unwrap()]);
    assert_eq!(c, Some(0), "{o} {e}");
}

#[test]
fn a_thin_clamp_is_found_on_a_diagonal_move_and_not_when_the_head_is_above_it() {
    let clamp = |height_mm| Limits {
        clamps: vec![Clamp {
            x0: 49.97,
            y0: 49.0,
            x1: 50.0,
            y1: 51.0,
            height_mm,
        }],
        ..Limits::default()
    };
    let program = "G21 G90\nM3\nG0 X0 Y0 Z3\nG0 X100 Y100\n";
    assert_eq!(findings(program, &clamp(6.0)).len(), 1);
    // the head travels at Z 3, above a clamp of 2 mm
    assert!(findings(program, &clamp(2.0)).is_empty());
    // a move that runs next to the clamp without touching it
    assert!(findings("G21 G90\nM3\nG0 X0 Y0 Z3\nG0 X100 Y2\n", &clamp(6.0)).is_empty());
}

#[test]
fn the_waits_of_g4_are_counted_apart_from_the_moves() {
    use eh3d_cam_debugger::timeline::dwell_s;
    let grbl = parse("G21\nG4 P0.5\nG1 X10 F600\nG4 P2\n", Dialect::Grbl);
    assert!((dwell_s(&grbl) - 2.5).abs() < 1e-9);
    // in Marlin P is in milliseconds and S in seconds
    let marlin = parse("G4 P500\nG4 S2\n", Dialect::Marlin);
    assert!((dwell_s(&marlin) - 2.5).abs() < 1e-9);
    assert_eq!(dwell_s(&parse("G1 X1 F100\n", Dialect::Grbl)), 0.0);
}

#[test]
fn every_exit_code_of_the_convention_is_given_for_what_it_means() {
    let clean = program_file("code_clean.nc", "G21 G90\nM3 S1000\nG1 X10 F100\nM2\n");
    let warning = program_file("code_warning.nc", "G21 G90\nG1 Z-1 F100\nM2\n");
    let alert = program_file("code_alert.nc", "G21 G90\nG0 Z-1\nM2\n");
    let (clean, warning, alert) = (
        clean.to_str().unwrap(),
        warning.to_str().unwrap(),
        alert.to_str().unwrap(),
    );
    assert_eq!(camdebug(&["check", clean]).0, Some(0), "nothing to say");
    assert_eq!(camdebug(&["check", alert]).0, Some(1), "an alert");
    assert_eq!(camdebug(&["check", warning]).0, Some(2), "only warnings");
    assert_eq!(
        camdebug(&["check", "none-such.nc"]).0,
        Some(1),
        "cannot be read"
    );
    assert_eq!(
        camdebug(&["check", clean, "--powr", "1"]).0,
        Some(64),
        "wrong use"
    );
    assert_eq!(camdebug(&["help"]).0, Some(0));
    assert_eq!(
        camdebug(&["steps", warning]).0,
        Some(0),
        "steps has nothing to warn about"

fn a_missing_feed_is_told_once_per_move_and_can_be_silenced_by_its_code() {
    let program = "G21 G90\nM3 S10000\nG1 Z-1\nG1 X5\nM2\n";
    let told = check(&trace(program), &Limits::default());
    let lines: Vec<usize> = told
        .iter()
        .filter(|f| f.message.contains("feed"))
        .map(|f| f.line)
        .collect();
    assert_eq!(lines, vec![3, 4], "{told:?}");
    let silenced = check(
        &trace(program),
        &Limits {
            ignore_codes: vec!["feed-missing".to_string()],
            ..Limits::default()
        },
    );
    assert!(
        silenced.iter().all(|f| !f.message.contains("feed")),
        "{silenced:?}"
    );
}
