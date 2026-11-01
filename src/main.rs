// EH3D-CAM-DEBUGGER - command line.
// Copyright (C) 2026 JuanenRac (Electro Hobby 3D). GPL-3.0-or-later.
#![forbid(unsafe_code)]

use eh3d_cam_debugger::checks::{check, has_alerts, strict, Clamp, Level, Limits};
use eh3d_cam_debugger::timeline::{format_time, timeline, Machine};
use eh3d_parser_gcode::{interpret, parse, Dialect};
use std::env;
use std::fs;
use std::process::ExitCode;

const HELP: &str = "eh3d-camdebug: checks a G-code program before it reaches a machine

  eh3d-camdebug check FILE.nc [options]     the time, the findings; exit code 1 when there is an alert
  eh3d-camdebug steps FILE.nc [options]     every move with its line, length and time

Options: --accel MM_S2 (500)  --rapid MM_MIN (3000)  --feed MM_MIN (300, used when a cut has no feed)  --dialect grbl|marlin|planetcnc
         --table WIDTH,DEPTH  --surface Z (0)  --min-z Z  --max-feed MM_MIN  --clamp X0,Y0,X1,Y1,HEIGHT (as many as needed)
         --strict (every warning is an alert too: exit code 1 for a cut with the spindle off, for one)
The time is an upper bound: every move is counted from rest to rest.";

fn nums(text: &str, n: usize, what: &str) -> Result<Vec<f64>, String> {
    let v: Vec<f64> = text
        .split(',')
        .map(|p| p.trim().parse::<f64>().ok().filter(|x| x.is_finite()))
        .collect::<Option<_>>()
        .ok_or_else(|| format!("{what} '{text}' has something that is not a number"))?;
    if v.len() != n {
        return Err(format!("{what} needs {n} numbers separated by commas"));
    }
    Ok(v)
}

fn parse_options(args: &[String]) -> Result<(Machine, Limits, Dialect, bool), String> {
    let (mut m, mut l, mut d) = (Machine::default(), Limits::default(), Dialect::Grbl);
    let mut strict = false;
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--strict" {
            strict = true;
            i += 1;
            continue;
        }
        let value = || {
            args.get(i + 1)
                .cloned()
                .ok_or_else(|| format!("{} needs a value", args[i]))
        };
        match args[i].as_str() {
            "--accel" => m.accel_mm_s2 = nums(&value()?, 1, "--accel")?[0],
            "--rapid" => m.rapid_mm_min = nums(&value()?, 1, "--rapid")?[0],
            "--feed" => m.default_feed_mm_min = nums(&value()?, 1, "--feed")?[0],
            "--surface" => l.surface_z = nums(&value()?, 1, "--surface")?[0],
            "--min-z" => l.min_z = Some(nums(&value()?, 1, "--min-z")?[0]),
            "--max-feed" => l.max_feed_mm_min = Some(nums(&value()?, 1, "--max-feed")?[0]),
            "--table" => {
                let v = nums(&value()?, 2, "--table")?;
                l.table = Some((v[0], v[1]));
            }
            "--clamp" => {
                let v = nums(&value()?, 5, "--clamp")?;
                l.clamps.push(Clamp {
                    x0: v[0],
                    y0: v[1],
                    x1: v[2],
                    y1: v[3],
                    height_mm: v[4],
                });
            }
            "--dialect" => {
                d = match value()?.to_ascii_lowercase().as_str() {
                    "grbl" => Dialect::Grbl,
                    "marlin" => Dialect::Marlin,
                    "planetcnc" => Dialect::PlanetCnc,
                    other => {
                        return Err(format!(
                            "--dialect '{other}' is not grbl, marlin or planetcnc"
                        ))
                    }
                }
            }
            other => return Err(format!("{other}: unknown option")),
        }
        i += 2;
    }
    m.validate()?;
    Ok((m, l, d, strict))
}

fn run(mut args: Vec<String>) -> Result<bool, String> {
    if args.is_empty() || matches!(args[0].as_str(), "help" | "--help" | "-h") {
        println!("{HELP}");
        return Ok(true);
    }
    let cmd = args.remove(0);
    if args.is_empty() {
        return Err(format!("{cmd} needs a G-code file"));
    }
    let file = args.remove(0);
    let (machine, limits, dialect, strict_mode) = parse_options(&args)?;
    let bytes = fs::read(&file).map_err(|e| format!("{file}: {e}"))?;
    let trace = interpret(&parse(&String::from_utf8_lossy(&bytes), dialect));
    let tl = timeline(&trace, &machine)?;
    match cmd.as_str() {
        "check" => {
            println!(
                "{} moves: {:.1} mm of cut, {:.1} mm of rapid",
                trace.segments.len(),
                trace.cutting_length_mm,
                trace.rapid_length_mm
            );
            println!(
                "time (upper bound): {}  (cutting {}, rapid {})",
                format_time(tl.total_s),
                format_time(tl.cutting_s),
                format_time(tl.rapid_s)
            );
            let mut findings = check(&trace, &limits);
            if strict_mode {
                strict(&mut findings);
            }
            for f in &findings {
                println!(
                    "{} line {}: {}",
                    if f.level == Level::Alert {
                        "ALERT  "
                    } else {
                        "warning"
                    },
                    f.line,
                    f.message
                );
            }
            if findings.is_empty() {
                println!("no findings");
            }
            Ok(!has_alerts(&findings))
        }
        "steps" => {
            for t in &tl.steps {
                println!(
                    "line {:>4}  {:<5} {:>9.3} mm  {:>8.2} s  at {:>9}",
                    t.line,
                    if t.rapid { "rapid" } else { "cut" },
                    t.length_mm,
                    t.time_s,
                    format_time(t.start_s)
                );
            }
            Ok(true)
        }
        other => Err(format!("{other}: unknown command (see help)")),
    }
}

fn main() -> ExitCode {
    match run(env::args().skip(1).collect()) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::from(1),
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(2)
        }
    }
}
