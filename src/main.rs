// EH3D-CAM-DEBUGGER - command line.
// Copyright (C) 2026 JuanenRac (Electro Hobby 3D). GPL-3.0-or-later.
#![forbid(unsafe_code)]

use eh3d_cam_debugger::checks::{check, has_alerts, strict, Clamp, Level, Limits};
use eh3d_cam_debugger::timeline::{dwell_s, format_time, timeline, Machine};
use eh3d_parser_gcode::{
    codes, interpret_with, parse_bytes_with, read_file_limited, Dialect, InputError, InputLimits,
    InterpretOptions, ReadError,
};
use std::env;
use std::process::ExitCode;

const HELP: &str = "eh3d-camdebug: checks a G-code program before it reaches a machine

  eh3d-camdebug check FILE.nc [options]     the time, the findings; exit code 1 when there is an alert, 2 when there are only warnings, 64 for a wrong option
  eh3d-camdebug steps FILE.nc [options]     every move with its line, length and time
  eh3d-camdebug [gui] [FILE.nc]             the window (a build with --features gui): with no arguments, with a program or with gui it opens; the commands above never open it

Options: --accel MM_S2 (500)  --rapid MM_MIN (3000)  --feed MM_MIN (300, used when a cut has no feed)  --dialect grbl|marlin|planetcnc
         --table WIDTH,DEPTH  --surface Z (0)  --min-z Z  --max-feed MM_MIN  --laser (M3 or M4 with S0 is not burning)  --clamp X0,Y0,X1,Y1,HEIGHT (as many as needed)
         --strict (every warning is an alert too: exit code 1 for a cut with the spindle off, for one)
         --ignore CODE[,CODE] (leave out the notices of the reader with those codes, for example line-too-long)  --max-bytes N (the largest file, 268435456 by default)
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

struct Options {
    machine: Machine,
    limits: Limits,
    dialect: Dialect,
    strict: bool,
    laser: bool,
    max_bytes: usize,
}

/// The codes of an `--ignore` value, each one checked against the codes of the reader: a misspelt code would ignore nothing.
fn ignored_codes(value: &str) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    for c in value.split(',').map(str::trim).filter(|c| !c.is_empty()) {
        if !codes::ALL.contains(&c) {
            return Err(format!("--ignore: '{c}' is not a code of the reader"));
        }
        out.push(c.to_string());
    }
    Ok(out)
}

fn parse_options(args: &[String]) -> Result<Options, String> {
    let (mut m, mut l, mut d) = (Machine::default(), Limits::default(), Dialect::Grbl);
    let mut strict = false;
    let mut laser = false;
    let mut max_bytes = InputLimits::default().max_bytes;
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--strict" {
            strict = true;
            i += 1;
            continue;
        }
        if args[i] == "--laser" {
            laser = true;
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
            "--ignore" => l.ignore_codes.extend(ignored_codes(&value()?)?),
            "--max-bytes" => max_bytes = nums(&value()?, 1, "--max-bytes")?[0] as usize,
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
    Ok(Options {
        machine: m,
        limits: l,
        dialect: d,
        strict,
        laser,
        max_bytes,
    })
}

/// The language of the person from the variables of the system (`en`, `es`, `de`, `fr`, `it`, `ja`, `zh`, `pt`); English when it is none of them.
fn language() -> &'static str {
    for var in ["LC_ALL", "LC_MESSAGES", "LANG"] {
        if let Ok(v) = env::var(var) {
            if v.is_empty() {
                continue;
            }
            let code = v.chars().take(2).collect::<String>().to_ascii_lowercase();
            return ["es", "de", "fr", "it", "ja", "zh", "pt"]
                .into_iter()
                .find(|l| *l == code)
                .unwrap_or("en");
        }
    }
    "en"
}

/// The message of a program that is over the size limit, in the language of the person.
fn too_large(size: usize, limit: usize) -> String {
    let human = |n: usize| {
        if n >= 1_048_576 {
            format!("{} MiB", n / 1_048_576)
        } else {
            format!("{n} bytes")
        }
    };
    let (s, l) = (human(size), human(limit));
    match language() {
        "es" => format!("el fichero supera el límite ({s} frente a {l}); use --max-bytes si es correcto"),
        "de" => format!("die Datei überschreitet die Grenze ({s} gegenüber {l}); --max-bytes verwenden, wenn das richtig ist"),
        "fr" => format!("le fichier dépasse la limite ({s} contre {l}) ; utilisez --max-bytes si c'est voulu"),
        "it" => format!("il file supera il limite ({s} contro {l}); usare --max-bytes se è corretto"),
        "ja" => format!("ファイルが上限を超えています（{s}、上限 {l}）。正しい場合は --max-bytes を使ってください"),
        "zh" => format!("文件超过上限（{s}，上限 {l}）；如无误，请使用 --max-bytes"),
        "pt" => format!("o ficheiro excede o limite ({s} contra {l}); use --max-bytes se estiver correto"),
        _ => format!("the file is above the limit ({s} against {l}); use --max-bytes if that is right"),
    }
}

/// How a run ends badly, for the exit code of the convention of the suite: a wrong use is 64, a failure (a file that cannot be read, a limit, a check that cannot run) is 1.
enum Fail {
    Usage(String),
    Failed(String),
}

impl From<String> for Fail {
    fn from(message: String) -> Fail {
        Fail::Failed(message)
    }
}

/// The exit code of a run: 0 nothing to say, 1 an alert, 2 only warnings (64 and 1 for the runs that failed are in `main`).
fn run(mut args: Vec<String>) -> Result<u8, Fail> {
    if args.is_empty() || matches!(args[0].as_str(), "help" | "--help" | "-h") {
        println!("{HELP}");
        return Ok(0);
    }
    let cmd = args.remove(0);
    if cmd == "gui" {
        // a build with the window never gets here (main opens it): this one is only a command line
        return Err(Fail::Failed(
            "this build has no window: build it with --features gui".into(),
        ));
    }
    // an unknown command is refused before any file is read
    if !matches!(cmd.as_str(), "check" | "steps") {
        return Err(Fail::Usage(format!("{cmd}: unknown command (see help)")));
    }
    if args.is_empty() {
        return Err(Fail::Usage(format!("{cmd} needs a G-code file")));
    }
    let file = args.remove(0);
    // the file comes first, as the help says: an option in its place is a wrong use, not a file that cannot be read
    if file.starts_with("--") {
        return Err(Fail::Usage(format!(
            "{file}: the file comes first, before the options (see help)"
        )));
    }
    let Options {
        machine,
        limits,
        dialect,
        strict: strict_mode,
        laser,
        max_bytes,
    } = parse_options(&args).map_err(Fail::Usage)?;
    let bytes = read_file_limited(std::path::Path::new(&file), &InputLimits { max_bytes })
        .map_err(|e| match e {
            ReadError::TooLarge(InputError::TooLarge { size, limit }) => {
                format!("{file}: {}", too_large(size, limit))
            }
            other => format!("{file}: {other}"),
        })?;
    let program = parse_bytes_with(&bytes, dialect, &InputLimits { max_bytes })
        .map_err(|e| format!("{file}: {e}"))?;
    let trace = interpret_with(&program, &InterpretOptions { s0_is_off: laser });
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
            // the waits (G4) are not moves: they are added apart so that the time is not understated
            let waits = dwell_s(&program);
            if waits > 0.0 {
                println!(
                    "waits (G4): {}  (with the waits: {})",
                    format_time(waits),
                    format_time(tl.total_s + waits)
                );
            }
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
            Ok(if has_alerts(&findings) {
                1
            } else if findings.is_empty() {
                0
            } else {
                2
            })
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
            Ok(0)
        }
        other => Err(Fail::Usage(format!("{other}: unknown command (see help)"))),
    }
}

/// Whether the program opens its window instead of running a command, and the program to open in it: no arguments, `gui` (with a program or without), or a single program given by
/// the system to open (a name with an extension that is a file; the commands have none).
#[cfg(feature = "gui")]
fn window_request(args: &[String]) -> Option<Option<String>> {
    match args {
        [] => Some(None),
        [a] if a == "gui" => Some(None),
        [a, f] if a == "gui" => Some(Some(f.clone())),
        [f] if !f.starts_with('-')
            && std::path::Path::new(f).extension().is_some()
            && std::path::Path::new(f).is_file() =>
        {
            Some(Some(f.clone()))
        }
        _ => None,
    }
}

#[cfg(feature = "gui")]
fn open_window(program: Option<String>) -> ExitCode {
    let lang = eh3d_cam_debugger::lang::Lang::from_env();
    match eh3d_cam_debugger::gui::run(program.map(std::path::PathBuf::from), lang) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: the window could not be opened: {e}");
            ExitCode::from(1)
        }
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    #[cfg(feature = "gui")]
    if let Some(program) = window_request(&args) {
        return open_window(program);
    }
    match run(args) {
        Ok(code) => ExitCode::from(code),
        Err(Fail::Usage(e)) => {
            eprintln!("error: {e}");
            ExitCode::from(64)
        }
        Err(Fail::Failed(e)) => {
            eprintln!("error: {e}");
            ExitCode::from(1)
        }
    }
}
