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
