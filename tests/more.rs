//! More tests: the times of the moves and how they are written, the machine that is refused, the length of arcs, and the findings that become alerts.

use eh3d_cam_debugger::checks::{check, has_alerts, strict, Finding, Level, Limits};
use eh3d_cam_debugger::timeline::{format_time, length, move_time, timeline, Machine};
use eh3d_parser_gcode::{interpret, parse, Dialect, Trace};

fn trace(text: &str) -> Trace {
    interpret(&parse(text, Dialect::Grbl))
}

#[test]
fn the_time_is_written_in_the_unit_that_fits_and_never_says_sixty_seconds() {
    assert_eq!(format_time(0.0), "0.0 s");
    assert_eq!(format_time(12.5), "12.5 s");
    assert_eq!(format_time(59.9), "59.9 s");
    // 59.96 s would be written 60.0 s: it is a minute
    assert_eq!(format_time(59.96), "1 min 00 s");
    assert_eq!(format_time(60.0), "1 min 00 s");
    assert_eq!(format_time(200.0), "3 min 20 s");
    assert_eq!(format_time(3600.0), "1 h 00 min 00 s");
    assert_eq!(format_time(3725.0), "1 h 02 min 05 s");
}

#[test]
fn a_time_that_makes_no_sense_is_a_dash() {
    for bad in [-1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(format_time(bad), "-", "{bad}");
    }
}

#[test]
fn a_move_that_does_not_move_or_is_not_a_length_takes_no_time_and_a_longer_one_takes_longer() {
    assert_eq!(move_time(0.0, 600.0, 100.0), 0.0);
    assert_eq!(move_time(-5.0, 600.0, 100.0), 0.0);
    let mut last = 0.0;
    for l in [0.001, 0.01, 0.1, 1.0, 10.0, 100.0, 1000.0] {
        let t = move_time(l, 600.0, 100.0);
        assert!(t > last, "{l}: {t} after {last}");
        last = t;
    }
    // more acceleration, less time; more speed on a long move, less time
    assert!(move_time(50.0, 600.0, 400.0) < move_time(50.0, 600.0, 100.0));
    assert!(move_time(500.0, 1200.0, 100.0) < move_time(500.0, 600.0, 100.0));
}

#[test]
fn the_ramp_and_the_cruise_meet_where_the_move_is_just_long_enough() {
    // 10 mm/s with 100 mm/s^2: 1 mm to reach the speed
    let at = move_time(1.0, 600.0, 100.0);
    assert!((at - 0.2).abs() < 1e-9, "{at}");
    let below = move_time(0.999999, 600.0, 100.0);
    assert!((below - at).abs() < 1e-3);
}

#[test]
fn a_machine_with_a_value_that_is_not_more_than_zero_is_refused_naming_it() {
    let good = Machine::default();
    assert!(good.validate().is_ok());
    for (machine, name) in [
        (
            Machine {
                accel_mm_s2: 0.0,
                ..good
            },
            "accel_mm_s2",
        ),
        (
            Machine {
                rapid_mm_min: -5.0,
                ..good
            },
            "rapid_mm_min",
        ),
        (
            Machine {
                default_feed_mm_min: f64::NAN,
                ..good
            },
            "default_feed_mm_min",
        ),
        (
            Machine {
                accel_mm_s2: f64::INFINITY,
                ..good
            },
            "accel_mm_s2",
        ),
    ] {
        let e = machine.validate().unwrap_err();
        assert!(e.contains(name), "{e}");
        // and the timeline does not start with it
        assert!(timeline(&trace("G0 X10\n"), &machine).is_err());
    }
}

#[test]
fn a_program_with_nothing_to_move_has_an_empty_timeline() {
    let tl = timeline(&trace("G21 G90\nM2\n"), &Machine::default()).unwrap();
    assert!(tl.steps.is_empty());
    assert_eq!((tl.total_s, tl.cutting_s, tl.rapid_s), (0.0, 0.0, 0.0));
}

#[test]
fn the_steps_begin_where_the_one_before_ended_and_the_total_is_cutting_plus_rapid() {
    let t = trace("G21 G90\nG0 X0 Y0\nG0 X50 Y0\nG1 X50 Y50 F300\nG1 X0 Y50\nG0 X0 Y0\nM2\n");
    let tl = timeline(&t, &Machine::default()).unwrap();
    assert!(tl.steps.len() >= 4);
    let mut at = 0.0;
    for s in &tl.steps {
        assert!((s.start_s - at).abs() < 1e-9);
        at += s.time_s;
    }
    assert!((tl.total_s - at).abs() < 1e-9);
    assert!((tl.total_s - (tl.cutting_s + tl.rapid_s)).abs() < 1e-9);
    assert!(tl.steps.iter().any(|s| s.rapid) && tl.steps.iter().any(|s| !s.rapid));
}

#[test]
fn a_cutting_move_without_a_feed_goes_at_the_default_feed() {
    let t = trace("G21 G90\nG0 X0 Y0\nG1 X60 Y0\nM2\n");
    let slow = Machine {
        default_feed_mm_min: 60.0,
        ..Machine::default()
    };
    let fast = Machine {
        default_feed_mm_min: 600.0,
        ..Machine::default()
    };
    let (a, b) = (timeline(&t, &slow).unwrap(), timeline(&t, &fast).unwrap());
    assert!(a.cutting_s > b.cutting_s);
    // the same program with a feed of its own does not care for the default
    let t2 = trace("G21 G90\nG0 X0 Y0\nG1 X60 Y0 F600\nM2\n");
    assert!(
        (timeline(&t2, &slow).unwrap().cutting_s - timeline(&t2, &fast).unwrap().cutting_s).abs()
            < 1e-9
    );
}

#[test]
fn a_straight_move_has_the_length_of_the_line_and_a_full_circle_the_length_of_the_circumference() {
    let t = trace("G21 G90\nG0 X0 Y0\nG1 X3 Y4 F100\nM2\n");
    let seg = t.segments.last().unwrap();
    assert!((length(seg) - 5.0).abs() < 1e-9);
    let t = trace("G21 G90\nG0 X10 Y0\nG2 X10 Y0 I-10 J0 F100\nM2\n");
    let circle = t
        .segments
        .iter()
        .find(|s| !matches!(s.motion, eh3d_parser_gcode::Motion::Rapid))
        .unwrap();
    assert!(
        (length(circle) - 2.0 * std::f64::consts::PI * 10.0).abs() < 1e-6,
        "{}",
        length(circle)
    );
}

#[test]
fn a_helix_is_longer_than_the_flat_arc_it_climbs() {
    let flat = trace("G21 G90\nG0 X10 Y0 Z0\nG3 X-10 Y0 I-10 J0 F100\nM2\n");
    let helix = trace("G21 G90\nG0 X10 Y0 Z0\nG3 X-10 Y0 Z-5 I-10 J0 F100\nM2\n");
    let pick = |t: &Trace| {
        length(
            t.segments
                .iter()
                .rev()
                .find(|s| !matches!(s.motion, eh3d_parser_gcode::Motion::Rapid))
                .unwrap(),
        )
    };
    assert!((pick(&flat) - std::f64::consts::PI * 10.0).abs() < 1e-6);
    assert!(pick(&helix) > pick(&flat));
}

fn finding(level: Level) -> Finding {
    Finding {
        line: 1,
        level,
        message: "x".to_string(),
    }
}

#[test]
fn only_alerts_count_as_alerts_and_strict_turns_every_warning_into_one() {
    assert!(!has_alerts(&[]));
    assert!(!has_alerts(&[
        finding(Level::Warning),
        finding(Level::Warning)
    ]));
    assert!(has_alerts(&[
        finding(Level::Warning),
        finding(Level::Alert)
    ]));
    let mut all = [finding(Level::Warning), finding(Level::Warning)];
    strict(&mut all);
    assert!(all.iter().all(|f| f.level == Level::Alert));
    assert!(has_alerts(&all));
    // on nothing it does nothing
    strict(&mut []);
}

#[test]
fn a_clean_program_inside_the_limits_has_no_findings_and_the_findings_come_in_the_order_of_the_program(
) {
    let clean = trace("G21 G90\nG0 Z5\nG0 X10 Y10\nG1 Z-0.1 F100\nG1 X20 Y10\nG0 Z5\nM2\n");
    let limits = Limits {
        table: Some((100.0, 100.0)),
        min_z: Some(-2.0),
        ..Limits::default()
    };
    assert!(!has_alerts(&check(&clean, &limits)));
    let bad = trace("G21 G90\nG0 X10 Y10\nG1 X500 Y10 F100\nG1 X20 Y900\nM2\n");
    let f = check(&bad, &limits);
    assert!(f.len() >= 2, "{f:?}");
    assert!(f.windows(2).all(|w| w[0].line <= w[1].line), "{f:?}");
}
