//! The checks of the plunge feed and of how a program ends, the summary of a program and `check --json`.

use eh3d_cam_debugger::checks::{check, check_json, json_string, summary, Finding, Level, Limits};
use eh3d_parser_gcode::{interpret, parse, Dialect, Trace};
use std::fs;
use std::process::Command;

fn trace(text: &str) -> Trace {
    interpret(&parse(text, Dialect::Grbl))
}

fn warnings(text: &str, limits: &Limits) -> Vec<(usize, String)> {
    check(&trace(text), limits)
        .into_iter()
        .filter(|f| f.level == Level::Warning)
        .map(|f| (f.line, f.message))
        .collect()
}

fn plunge_limit(mm_min: f64) -> Limits {
    Limits {
        max_plunge_mm_min: Some(mm_min),
        ..Limits::default()
    }
}

// ---- the plunge ----

#[test]
fn a_straight_plunge_faster_than_the_limit_is_a_warning_with_its_line_depth_and_feed() {
    let w = warnings(
        "G21 G90\nM3 S1000\nG0 Z5\nG1 Z-1 F200\nM2\n",
        &plunge_limit(100.0),
    );
    let plunges: Vec<_> = w.iter().filter(|(_, m)| m.contains("plunge")).collect();
    assert_eq!(plunges.len(), 1, "{w:?}");
    assert_eq!(plunges[0].0, 4);
    assert_eq!(
        plunges[0].1,
        "the plunge of 6.000 mm goes down at 200 mm/min, above the plunge limit of 100"
    );
}

#[test]
fn a_plunge_at_the_limit_or_slower_is_clear_and_no_limit_checks_nothing() {
    let program = "G21 G90\nM3 S1000\nG0 Z5\nG1 Z-1 F100\nM2\n";
    assert!(warnings(program, &plunge_limit(100.0)).is_empty());
    assert!(warnings(program, &plunge_limit(100.0001)).is_empty());
    assert_eq!(warnings(program, &plunge_limit(99.9)).len(), 1);
    assert!(warnings(program, &Limits::default()).is_empty());
}

#[test]
fn only_a_move_that_goes_down_at_least_as_far_as_it_goes_along_is_a_plunge() {
    // down 2 mm along 2 mm (45 degrees) is one; down 1 mm along 10 mm is a ramp, a cut
    let limits = plunge_limit(50.0);
    let steep = warnings(
        "G21 G90\nM3 S1000\nG0 X0 Y0 Z1\nG1 X2 Z-1 F300\nM2\n",
        &limits,
    );
    assert_eq!(steep.len(), 1, "{steep:?}");
    let ramp = warnings(
        "G21 G90\nM3 S1000\nG0 X0 Y0 Z1\nG1 X10 Z0 F300\nM2\n",
        &limits,
    );
    assert!(ramp.is_empty(), "{ramp:?}");
    // a move along the plane or going up is never a plunge, and neither is a rapid one
    assert!(warnings(
        "G21 G90\nM3 S1000\nG1 X10 F900\nG1 Z2 F900\nG0 Z-1\nM2\n",
        &limits
    )
    .iter()
    .all(|(_, m)| !m.contains("plunge")));
}

#[test]
fn a_plunge_with_no_feed_is_left_to_the_check_of_the_missing_feed() {
    let w = warnings("G21 G90\nM3 S1000\nG1 Z-1\nM2\n", &plunge_limit(50.0));
    assert!(w.iter().all(|(_, m)| !m.contains("plunge")), "{w:?}");
    assert!(w.iter().any(|(_, m)| m.contains("no feed")), "{w:?}");
}

// ---- how a program ends ----

fn end_z(z: f64) -> Limits {
    Limits {
        end_z: Some(z),
        ..Limits::default()
    }
}

#[test]
fn a_program_that_ends_with_the_tool_below_the_end_height_is_a_warning_on_its_last_move() {
    let w = warnings(
        "G21 G90\nM3 S1000\nG0 Z5\nG1 Z-1 F100\nM5\nM2\n",
        &end_z(2.0),
    );
    assert_eq!(w.len(), 1, "{w:?}");
    assert_eq!(w[0].0, 4);
    assert_eq!(
        w[0].1,
        "the program ends with the tool at Z -1.000, below the end height of 2.000"
    );
}

#[test]
fn ending_at_the_end_height_or_above_it_is_clear_and_so_is_a_program_that_never_moves() {
    assert!(warnings("G21 G90\nG0 Z5\nG0 Z2\nM2\n", &end_z(2.0)).is_empty());
    assert!(warnings("G21 G90\nG0 Z5\nM2\n", &end_z(2.0)).is_empty());
    assert!(warnings("G21 G90\nM2\n", &end_z(2.0)).is_empty());
    assert!(warnings("", &end_z(2.0)).is_empty());
}

#[test]
fn a_program_that_does_not_end_with_m2_or_m30_is_a_warning_only_when_asked() {
    let open = "G21 G90\nG0 Z5\nG0 X10\n";
    let asked = Limits {
        require_end: true,
        ..Limits::default()
    };
    let w = warnings(open, &asked);
    assert_eq!(
        w,
        vec![(3, "the program does not end with M2 or M30".to_string())]
    );
    assert!(warnings(open, &Limits::default()).is_empty());
    for end in ["M2", "M30"] {
        assert!(
            warnings(&format!("{open}{end}\n"), &asked).is_empty(),
            "{end}"
        );
    }
    assert!(
        warnings("G21 G90\n", &asked).is_empty(),
        "no move, nothing to say"
    );
}

// ---- the summary ----

#[test]
fn the_summary_counts_the_kinds_of_move_and_gives_the_extent_and_the_feeds() {
    let t = trace(
        "G21 G90\nM3 S1000\nG0 X-2 Y1 Z5\nG1 Z-1 F120\nG1 X10 F300\nG2 X14 Y5 I0 J4\nG0 Z5\nM2\n",
    );
    let s = summary(&t);
    assert_eq!((s.rapid_moves, s.straight_cuts, s.arcs), (2, 2, 1));
    let (lo, hi) = s.extent.unwrap();
    assert_eq!((lo.0, lo.2), (-2.0, -1.0));
    assert!(hi.2 == 5.0 && hi.0 >= 14.0 - 1e-9);
    assert_eq!(s.feed_range, Some((120.0, 300.0)));
}

#[test]
fn the_summary_of_an_empty_program_has_nothing_and_the_tools_are_listed_once_in_order() {
    let s = summary(&trace(""));
    assert_eq!((s.rapid_moves, s.straight_cuts, s.arcs), (0, 0, 0));
    assert_eq!((s.extent, s.feed_range), (None, None));
    assert!(s.tools.is_empty());
    let t = trace("G21 G90\nT3 M6\nG1 X1 F100\nT1 M6\nG1 X2\nT3 M6\nG1 X3\nM2\n");
    assert_eq!(summary(&t).tools, vec![1, 3]);
}

// ---- the JSON ----

#[test]
fn a_text_is_written_as_a_json_string() {
    assert_eq!(json_string("x"), "\"x\"");
    assert_eq!(json_string("a\"b\\c\n\t"), "\"a\\\"b\\\\c\\n\\t\"");
    assert_eq!(json_string("\u{2}"), "\"\\u0002\"");
    assert_eq!(json_string("línea ✓"), "\"línea ✓\"");
}

#[test]
fn the_json_of_a_check_has_the_numbers_the_extent_and_every_finding() {
    let t = trace("G21 G90\nM3 S1000\nG0 Z5\nG1 Z-1 F200\nG0 Z5\nM2\n");
    let findings = vec![
        Finding {
            line: 4,
            level: Level::Warning,
            message: "it says \"hello\"".into(),
        },
        Finding {
            line: 5,
            level: Level::Alert,
            message: "stop".into(),
        },
    ];
    let j = check_json(&t, 12.5, 10.0, 2.5, 1.0, &findings);
    for part in [
        "\"moves\":3,\"rapid_moves\":2,\"straight_cuts\":1,\"arcs\":0",
        "\"total_s\":12.5,\"cutting_s\":10,\"rapid_s\":2.5,\"waits_s\":1",
        "\"extent\":{\"min\":[0,0,-1],\"max\":[0,0,5]}",
        "\"feed_range\":[200,200]",
        "\"ended\":true",
        "{\"line\":4,\"level\":\"warning\",\"message\":\"it says \\\"hello\\\"\"}",
        "{\"line\":5,\"level\":\"alert\",\"message\":\"stop\"}",
    ] {
        assert!(j.contains(part), "{part} in {j}");
    }
    assert!(!j.contains('\n') && j.starts_with('{') && j.ends_with("]}"));
}

#[test]
fn the_json_of_an_empty_program_has_null_extent_and_feeds_and_no_findings() {
    let j = check_json(&trace(""), 0.0, 0.0, 0.0, 0.0, &[]);
    assert!(
        j.contains("\"extent\":null") && j.contains("\"feed_range\":null"),
        "{j}"
    );
    assert!(
        j.contains("\"tools\":[]") && j.ends_with("\"findings\":[]}"),
        "{j}"
    );
    // a time that is not a number is null, never a word JSON does not know
    let nan = check_json(&trace(""), f64::NAN, 0.0, f64::INFINITY, 0.0, &[]);
    assert!(
        nan.contains("\"total_s\":null") && nan.contains("\"rapid_s\":null"),
        "{nan}"
    );
}

// ---- the command line ----

fn run(args: &[&str]) -> (Option<i32>, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_eh3d-camdebug"))
        .args(args)
        .output()
        .expect("the program runs");
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn program(name: &str, text: &str) -> (std::path::PathBuf, String) {
    let dir = std::env::temp_dir().join(format!(
        "eh3d_camdebug_checks_{}_{name}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let file = dir.join("p.nc");
    fs::write(&file, text).unwrap();
    let s = file.to_str().unwrap().to_string();
    (dir, s)
}

#[test]
fn check_prints_the_extent_the_kinds_of_move_the_tools_and_the_feeds() {
    let (dir, f) = program(
        "summary",
        "G21 G90\nT2 M6\nM3 S1000\nG0 X0 Y0 Z5\nG1 Z-0.5 F150\nG1 X20 Y10 F400\nG0 Z5\nM5\nM2\n",
    );
    let (code, out, err) = run(&["check", &f]);
    assert_eq!(code, Some(0), "{out}{err}");
    assert!(
        out.contains("extent: X 0.000 to 20.000, Y 0.000 to 10.000, Z -0.500 to 5.000 mm"),
        "{out}"
    );
    assert!(
        out.contains("2 rapid moves, 2 straight cuts, 0 arcs"),
        "{out}"
    );
    assert!(out.contains("tools: T2"), "{out}");
    assert!(out.contains("cutting feeds: 150 to 400 mm/min"), "{out}");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn the_new_options_check_a_plunge_the_end_height_and_the_closing_of_the_program() {
    let (dir, f) = program(
        "options",
        "G21 G90\nM3 S1000\nG0 Z5\nG1 Z-1 F200\nG1 X5 F200\n",
    );
    let (code, out, _) = run(&[
        "check",
        &f,
        "--max-plunge",
        "100",
        "--end-z",
        "1",
        "--require-end",
    ]);
    assert_eq!(code, Some(2), "{out}");
    assert!(
        out.contains("warning line 4: the plunge of 6.000 mm goes down at 200 mm/min"),
        "{out}"
    );
    assert!(out.contains("warning line 5: the program ends with the tool at Z -1.000, below the end height of 1.000"), "{out}");
    assert!(
        out.contains("warning line 5: the program does not end with M2 or M30"),
        "{out}"
    );
    // the strict mode makes them alerts
    let (code, _, _) = run(&["check", &f, "--max-plunge", "100", "--strict"]);
    assert_eq!(code, Some(1));
    // without the options nothing of this is said
    let (_, out, _) = run(&["check", &f]);
    assert!(
        !out.contains("plunge")
            && !out.contains("ends with the tool")
            && !out.contains("does not end"),
        "{out}"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_plunge_limit_that_is_not_more_than_zero_or_not_a_number_is_a_wrong_use() {
    let (dir, f) = program("wrong", "G21 G90\nM2\n");
    for (value, what) in [
        ("0", "more than 0"),
        ("-5", "more than 0"),
        ("x", "not a number"),
    ] {
        let (code, _, err) = run(&["check", &f, "--max-plunge", value]);
        assert_eq!(code, Some(64), "{value}");
        assert!(err.contains(what), "{value}: {err}");
    }
    let (code, _, err) = run(&["check", &f, "--end-z", "up"]);
    assert_eq!(code, Some(64));
    assert!(err.contains("--end-z"), "{err}");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn check_json_is_one_object_with_the_same_exit_code_and_steps_does_not_take_it() {
    let (dir, f) = program("json", "G21 G90\nM3 S1000\nG0 Z5\nG1 Z-1 F200\nG0 Z5\nM2\n");
    let (code, out, err) = run(&["check", &f, "--json"]);
    assert_eq!(code, Some(0), "{err}");
    assert_eq!(out.lines().count(), 1, "{out}");
    assert!(
        out.starts_with("{\"moves\":3,") && out.ends_with("\"findings\":[]}\n"),
        "{out}"
    );
    let (code, out, _) = run(&["check", &f, "--json", "--max-plunge", "100"]);
    assert_eq!(code, Some(2));
    assert!(
        out.contains("\"level\":\"warning\"") && out.contains("the plunge of 6.000 mm"),
        "{out}"
    );
    let (code, out, _) = run(&["check", &f, "--json", "--min-z", "-0.5"]);
    assert_eq!(code, Some(1));
    assert!(out.contains("\"level\":\"alert\""), "{out}");
    let (code, _, err) = run(&["steps", &f, "--json"]);
    assert_eq!(code, Some(64));
    assert!(err.contains("--json is an option of check"), "{err}");
    let _ = fs::remove_dir_all(&dir);
}

/// The exit code and what was printed for `check` of `program` with the options given.
fn check_file(name: &str, program: &str, options: &[&str]) -> (Option<i32>, String) {
    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("left_on");
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join(format!("{name}.nc"));
    std::fs::write(&file, program).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_eh3d-camdebug"))
        .arg("check")
        .arg(&file)
        .args(options)
        .env("LANG", "en")
        .output()
        .expect("the program runs");
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
    )
}

#[test]
fn a_laser_left_on_at_the_end_is_an_alert_and_a_spindle_a_warning() {
    let program = "G21 G90\nM3 S100\nG1 X5 F100\n";
    let (code, out) = check_file("end_laser", program, &["--laser"]);
    assert_eq!(code, Some(1), "{out}");
    assert!(out.contains("ends with the laser on"), "{out}");
    let (code, out) = check_file("end_spindle", program, &[]);
    assert_eq!(code, Some(2), "{out}");
    assert!(out.contains("ends with the spindle on"), "{out}");
    // off, ended or at no power: nothing to say
    for (name, program) in [
        ("off", "G21 G90\nM3 S100\nG1 X5 F100\nM5\n"),
        ("ended", "G21 G90\nM3 S100\nG1 X5 F100\nM2\n"),
        ("no_power", "G21 G90\nM3 S100\nG1 X5 F100\nS0\n"),
    ] {
        let (code, out) = check_file(name, program, &["--laser"]);
        assert_eq!(code, Some(0), "{name}: {out}");
    }
}

#[test]
fn constant_laser_power_in_a_wait_or_a_pause_is_an_alert() {
    for (name, line, word) in [
        ("dwell", "G4 P2", "waits (G4)"),
        ("m0", "M0", "pauses"),
        ("m1", "M1", "pauses"),
    ] {
        let program = format!("G21 G90\nM3 S100\nG1 X5 F100\n{line}\nM5\nM2\n");
        let (code, out) = check_file(name, &program, &["--laser"]);
        assert_eq!(code, Some(1), "{line}: {out}");
        assert!(out.contains(word), "{line}: {out}");
        // a mill waits with its spindle on all the time
        let (code, out) = check_file(&format!("mill_{name}"), &program, &[]);
        assert_eq!(code, Some(0), "{line}: {out}");
        // M4 follows the movement: the beam is out while nothing moves
        let (code, out) = check_file(
            &format!("dynamic_{name}"),
            &program.replace("M3", "M4"),
            &["--laser"],
        );
        assert_eq!(code, Some(0), "{line}: {out}");
    }
}
