//! A spindle or a laser that is still on where nothing moves: at the end of the program, in a wait or in a pause.

use std::process::Command;

/// The exit code and what was printed for `check` of `program` with the options given.
fn check(name: &str, program: &str, options: &[&str]) -> (Option<i32>, String) {
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
    let (code, out) = check("end_laser", program, &["--laser"]);
    assert_eq!(code, Some(1), "{out}");
    assert!(out.contains("ends with the laser on"), "{out}");
    let (code, out) = check("end_spindle", program, &[]);
    assert_eq!(code, Some(2), "{out}");
    assert!(out.contains("ends with the spindle on"), "{out}");
    // off, ended or at no power: nothing to say
    for (name, program) in [
        ("off", "G21 G90\nM3 S100\nG1 X5 F100\nM5\n"),
        ("ended", "G21 G90\nM3 S100\nG1 X5 F100\nM2\n"),
        ("no_power", "G21 G90\nM3 S100\nG1 X5 F100\nS0\n"),
    ] {
        let (code, out) = check(name, program, &["--laser"]);
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
        let (code, out) = check(name, &program, &["--laser"]);
        assert_eq!(code, Some(1), "{line}: {out}");
        assert!(out.contains(word), "{line}: {out}");
        // a mill waits with its spindle on all the time
        let (code, out) = check(&format!("mill_{name}"), &program, &[]);
        assert_eq!(code, Some(0), "{line}: {out}");
        // M4 follows the movement: the beam is out while nothing moves
        let (code, out) = check(
            &format!("dynamic_{name}"),
            &program.replace("M3", "M4"),
            &["--laser"],
        );
        assert_eq!(code, Some(0), "{line}: {out}");
    }
}
