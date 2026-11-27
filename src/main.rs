// EH3D-CAM-DEBUGGER - command line.
// Copyright (C) 2026 JuanenRac (Electro Hobby 3D). GPL-3.0-or-later.
#![forbid(unsafe_code)]

use eh3d_cam_debugger::checks::{
    add_dialect_hint, add_left_on, check, check_json, has_alerts, parse_program, strict, summary,
    Clamp, Level, Limits,
};
use eh3d_cam_debugger::lang::{Lang, Msg};
use eh3d_cam_debugger::server;
use eh3d_cam_debugger::timeline::{dwell_s, format_time, timeline, Machine};
use eh3d_parser_gcode::{
    codes, interpret_with, read_file_limited, Dialect, InputError, InputLimits, InterpretOptions,
    ReadError,
};
use eh3d_serverclient::Settings as LinkSettings;
use std::env;
use std::process::ExitCode;

const HELP: &str = "eh3d-camdebug: checks a G-code program before it reaches a machine

  eh3d-camdebug check FILE.nc [options]     the time, the findings; exit code 1 when there is an alert, 2 when there are only warnings, 64 for a wrong option
  eh3d-camdebug steps FILE.nc [options]     every move with its line, length and time
  eh3d-camdebug server [status] [--config FILE]   the link to the EH3D server (server_address, server_token, server_machine, server_timeout_s, server_retries, server_mdns and server_max_answer_mib of the file) and what the server answers
  eh3d-camdebug [gui] [FILE.nc]             the window (a build with --features gui): with no arguments, with a program or with gui it opens; the commands above never open it
  eh3d-camdebug gui [FILE...] --shot OUT.png [--size WxH] [--theme forge|dark|light] [--wait N] [--lang CODE] [--replace]   a picture of the window (a build with --features gui)

Options: --accel MM_S2 (500)  --rapid MM_MIN (3000)  --feed MM_MIN (300, used when a cut has no feed)  --dialect grbl|marlin|planetcnc (none: GRBL, or Marlin, and said, for a program that is surely for a 3D printer)
         --table WIDTH,DEPTH  --surface Z (0)  --min-z Z  --max-feed MM_MIN  --laser (M3 or M4 with S0 is not burning)  --clamp X0,Y0,X1,Y1,HEIGHT (as many as needed)
         --max-plunge MM_MIN (warn of a plunge, a cut that goes down at least as far as it goes along, faster than that)  --end-z Z (warn when the program ends with the tool below Z)
         --require-end (warn when the program does not end with M2 or M30)
         --strict (every warning is an alert too: exit code 1 for a cut with the spindle off, for one)
         --json (check prints one JSON object, with the findings, instead of the text; the exit code is the same)
         --ignore CODE[,CODE] (leave out the notices of the reader with those codes, for example line-too-long)  --max-bytes N (the largest file, 268435456 by default)
         --config FILE (the keys of the link to the EH3D server; the other lines of the file are left alone)  --to-server (keep the JSON report of check, or the text of steps, on the EH3D server as an artifact; needs --config with server_address and server_token and is refused before any work when they are missing)
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
    /// The dialect named with `--dialect`; `None` lets the program decide between GRBL and Marlin.
    dialect: Option<Dialect>,
    strict: bool,
    laser: bool,
    json: bool,
    max_bytes: usize,
    /// The file with the keys of the link to the EH3D server (`--config FILE`).
    config: Option<String>,
    /// Keeps the report on the EH3D server as an artifact (`--to-server`).
    to_server: bool,
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
    let (mut m, mut l, mut d) = (Machine::default(), Limits::default(), None);
    let mut strict = false;
    let mut laser = false;
    let mut json = false;
    let mut max_bytes = InputLimits::default().max_bytes;
    let mut config = None;
    let mut to_server = false;
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--strict" {
            strict = true;
            i += 1;
            continue;
        }
        if args[i] == "--json" {
            json = true;
            i += 1;
            continue;
        }
        if args[i] == "--require-end" {
            l.require_end = true;
            i += 1;
            continue;
        }
        if args[i] == "--laser" {
            laser = true;
            i += 1;
            continue;
        }
        if args[i] == "--to-server" {
            to_server = true;
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
            "--max-plunge" => {
                let v = nums(&value()?, 1, "--max-plunge")?[0];
                if v <= 0.0 {
                    return Err("--max-plunge must be more than 0".into());
                }
                l.max_plunge_mm_min = Some(v);
            }
            "--end-z" => l.end_z = Some(nums(&value()?, 1, "--end-z")?[0]),
            "--ignore" => l.ignore_codes.extend(ignored_codes(&value()?)?),
            "--max-bytes" => max_bytes = nums(&value()?, 1, "--max-bytes")?[0] as usize,
            "--config" => config = Some(value()?),
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
                d = Some(match value()?.to_ascii_lowercase().as_str() {
                    "grbl" => Dialect::Grbl,
                    "marlin" => Dialect::Marlin,
                    "planetcnc" => Dialect::PlanetCnc,
                    other => {
                        return Err(format!(
                            "--dialect '{other}' is not grbl, marlin or planetcnc"
                        ))
                    }
                })
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
        json,
        max_bytes,
        config,
        to_server,
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

/// The link to the EH3D server from the file of `--config`: the keys of the link, checked with their ranges; the other lines of the file are left alone.
fn link_from(config: Option<&str>) -> Result<LinkSettings, String> {
    match config {
        Some(path) => server::settings_from_file(path),
        None => Ok(LinkSettings::default()),
    }
}

/// `--to-server` needs a server: the link of `--config`, with an address and a token.
fn link_for_upload(config: Option<&str>) -> Result<LinkSettings, String> {
    let path =
        config.ok_or("--to-server needs --config FILE with server_address and server_token")?;
    let settings = server::settings_from_file(path)?;
    server::require(&settings)?;
    Ok(settings)
}

/// `server [status] [--config FILE]`: the link, with the token as `***`, and what the server answers.
fn server_command(mut args: Vec<String>) -> Result<u8, Fail> {
    let config = match args.iter().position(|a| a == "--config") {
        Some(i) => {
            let value = args
                .get(i + 1)
                .cloned()
                .ok_or_else(|| Fail::Usage("--config needs a value".into()))?;
            args.drain(i..=i + 1);
            Some(value)
        }
        None => None,
    };
    match args.as_slice() {
        [] => {}
        [word] if word == "status" => {}
        [word] => {
            return Err(Fail::Usage(format!(
                "server {word}: unknown (use: server status)"
            )))
        }
        _ => return Err(Fail::Usage("server takes one word: status".into())),
    }
    let settings = link_from(config.as_deref()).map_err(Fail::Usage)?;
    print!("{}", server::status_report(settings)?);
    Ok(0)
}

/// Keeps a file on the server and says so; the report was already printed, so a refusal is told as such.
fn keep(
    link: &LinkSettings,
    name: &str,
    format: &str,
    data: &[u8],
    params: &[(&str, &str)],
) -> Result<(), String> {
    let line = server::keep(link.clone(), name, format, data, params)
        .map_err(|e| format!("{name}: not kept on the server: {e}"))?;
    println!("{line}");
    Ok(())
}

/// Keeps the JSON report of a check on the server, with the number of alerts and warnings and the dialect as its parameters.
fn keep_report(
    link: &LinkSettings,
    file: &str,
    dialect: &str,
    report: &str,
    findings: &[eh3d_cam_debugger::checks::Finding],
) -> Result<(), String> {
    let alerts = findings.iter().filter(|f| f.level == Level::Alert).count();
    let (alerts_text, warnings_text) = (alerts.to_string(), (findings.len() - alerts).to_string());
    keep(
        link,
        &format!("{}.report.json", file_name_of(file)),
        "application/json",
        report.as_bytes(),
        &[
            ("command", "check"),
            ("program", file_name_of(file).as_str()),
            ("dialect", dialect),
            ("alerts", alerts_text.as_str()),
            ("warnings", warnings_text.as_str()),
        ],
    )
}

/// The last part of a path, as text.
fn file_name_of(path: &str) -> String {
    std::path::Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("program")
        .to_string()
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
    if cmd == "server" {
        return server_command(args);
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
        json,
        max_bytes,
        config,
        to_server,
    } = parse_options(&args).map_err(Fail::Usage)?;
    // the keys of the link are checked whenever a file names them, and the server is there before any work is done
    let link = if to_server {
        Some(link_for_upload(config.as_deref()).map_err(Fail::Usage)?)
    } else {
        link_from(config.as_deref()).map_err(Fail::Usage)?;
        None
    };
    let bytes = read_file_limited(std::path::Path::new(&file), &InputLimits { max_bytes })
        .map_err(|e| match e {
            ReadError::TooLarge(InputError::TooLarge { size, limit }) => {
                format!("{file}: {}", too_large(size, limit))
            }
            other => format!("{file}: {other}"),
        })?;
    let (program, read_as_marlin) = parse_program(&bytes, dialect, &InputLimits { max_bytes })
        .map_err(|e| format!("{file}: {e}"))?;
    let dialect_name = format!("{:?}", program.dialect).to_ascii_lowercase();
    // said before anything else; on the error output where the standard one is a listing or a JSON object that another program reads
    let note = read_as_marlin
        .then(|| Msg::ReadAsMarlin.tr(Lang::from_code(language()).unwrap_or(Lang::En)));
    if let Some(note) = note {
        if cmd == "check" && !json {
            println!("{note}");
        } else {
            eprintln!("{note}");
        }
    }
    let trace = interpret_with(&program, &InterpretOptions { s0_is_off: laser });
    let tl = timeline(&trace, &machine)?;
    match cmd.as_str() {
        "check" if json => {
            let mut findings = check(&trace, &limits);
            add_left_on(&mut findings, &program, laser);
            add_dialect_hint(&mut findings, &program);
            if strict_mode {
                strict(&mut findings);
            }
            let mut report = check_json(
                &trace,
                tl.total_s,
                tl.cutting_s,
                tl.rapid_s,
                dwell_s(&program),
                &findings,
            );
            if read_as_marlin {
                // the report says it too, for the program that reads it
                report = report.replacen('{', "{\"read_as\":\"marlin\",", 1);
            }
            println!("{report}");
            if let Some(link) = &link {
                keep_report(link, &file, &dialect_name, &report, &findings)?;
            }
            Ok(if has_alerts(&findings) {
                1
            } else if findings.is_empty() {
                0
            } else {
                2
            })
        }
        "steps" if json => Err(Fail::Usage("--json is an option of check".into())),
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
            let s = summary(&trace);
            if let Some((lo, hi)) = s.extent {
                println!(
                    "extent: X {:.3} to {:.3}, Y {:.3} to {:.3}, Z {:.3} to {:.3} mm",
                    lo.0, hi.0, lo.1, hi.1, lo.2, hi.2
                );
            }
            println!(
                "{} rapid moves, {} straight cuts, {} arcs",
                s.rapid_moves, s.straight_cuts, s.arcs
            );
            if !s.tools.is_empty() {
                let tools: Vec<String> = s.tools.iter().map(|t| format!("T{t}")).collect();
                println!("tools: {}", tools.join(", "));
            }
            if let Some((lo, hi)) = s.feed_range {
                println!("cutting feeds: {lo:.0} to {hi:.0} mm/min");
            }
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
            add_left_on(&mut findings, &program, laser);
            add_dialect_hint(&mut findings, &program);
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
            if let Some(link) = &link {
                let report = check_json(
                    &trace,
                    tl.total_s,
                    tl.cutting_s,
                    tl.rapid_s,
                    dwell_s(&program),
                    &findings,
                );
                keep_report(link, &file, &dialect_name, &report, &findings)?;
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
            let mut listing = String::new();
            for t in &tl.steps {
                listing.push_str(&format!(
                    "line {:>4}  {:<5} {:>9.3} mm  {:>8.2} s  at {:>9}\n",
                    t.line,
                    if t.rapid { "rapid" } else { "cut" },
                    t.length_mm,
                    t.time_s,
                    format_time(t.start_s)
                ));
            }
            print!("{listing}");
            if let Some(link) = &link {
                let steps = tl.steps.len().to_string();
                keep(
                    link,
                    &format!("{}.steps.txt", file_name_of(&file)),
                    "text/plain",
                    listing.as_bytes(),
                    &[
                        ("command", "steps"),
                        ("program", file_name_of(&file).as_str()),
                        ("dialect", dialect_name.as_str()),
                        ("steps", steps.as_str()),
                    ],
                )?;
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

/// The picture mode: `--shot FILE.png` opens the window, saves what it draws as a PNG file and closes. The exit code is 0 when the file was written, 1 when it was not and 2 when the
/// arguments are wrong.
#[cfg(feature = "gui")]
fn take_picture(args: &[String]) -> ExitCode {
    let parsed = match eh3d_cam_debugger::shot::parse_window_args(args, 1) {
        Ok(parsed) => parsed,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(2);
        }
    };
    let Some(request) = parsed.shot else {
        eprintln!("error: --shot needs a file to write");
        return ExitCode::from(2);
    };
    let lang = parsed
        .lang
        .unwrap_or_else(eh3d_cam_debugger::lang::Lang::from_env);
    match eh3d_cam_debugger::gui::run_shot(parsed.files, lang, request) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: the picture could not be taken: {e}");
            ExitCode::from(1)
        }
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    if eh3d_cam_debugger::shot::wants_picture(&args) {
        #[cfg(feature = "gui")]
        return take_picture(&args);
        #[cfg(not(feature = "gui"))]
        {
            eprintln!("error: this build has no window: build it with --features gui");
            return ExitCode::from(1);
        }
    }
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
