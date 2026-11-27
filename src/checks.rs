//! What would hurt: each finding has the line of the program, how serious it is and what it is.

use eh3d_parser_gcode::{codes, Motion, Program, Segment, Severity, Trace};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    /// Probably wrong or wasteful, but it does not break anything.
    Warning,
    /// It would break the tool, the board or the machine: do not run the program.
    Alert,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Finding {
    pub line: usize,
    pub level: Level,
    pub message: String,
}

/// A clamp (or any fixture) that the head must not pass over below `height_mm`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Clamp {
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
    pub height_mm: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Limits {
    /// The table: X from 0 to `width_mm` and Y from 0 to `depth_mm`; `None` does not check it.
    pub table: Option<(f64, f64)>,
    /// The top of the material: below it the tool is in the board.
    pub surface_z: f64,
    /// The lowest Z the machine can reach (a negative number); `None` does not check it.
    pub min_z: Option<f64>,
    /// The fastest cutting feed that makes sense, in mm/min; `None` does not check it.
    pub max_feed_mm_min: Option<f64>,
    /// The fastest feed of a plunge (a cutting move that goes down at least as far as it goes along), in mm/min; `None` does not check it.
    pub max_plunge_mm_min: Option<f64>,
    /// The lowest Z the program may end at; ending below it (the tool still in the material or too near to it) is a warning; `None` does not check it.
    pub end_z: Option<f64>,
    /// A program that does not end with M2 or M30 is a warning.
    pub require_end: bool,
    pub clamps: Vec<Clamp>,
    /// The codes of the notices of the reader that are not reported (see the codes of `eh3d_parser_gcode`).
    pub ignore_codes: Vec<String>,
}

impl Default for Limits {
    fn default() -> Self {
        Limits {
            table: None,
            surface_z: 0.0,
            min_z: None,
            max_feed_mm_min: None,
            max_plunge_mm_min: None,
            end_z: None,
            require_end: false,
            clamps: Vec::new(),
            ignore_codes: Vec::new(),
        }
    }
}

const SAMPLE_MM: f64 = 0.5;
const EPS: f64 = 1e-9;

/// Where (x, y) the straight piece from `a` to `b` (x, y, z) enters the clamp below its height, if it does: an exact test of the
/// segment against the box (no sampling), so a clamp thinner than any step is still found.
fn segment_over_clamp(c: &Clamp, a: (f64, f64, f64), b: (f64, f64, f64)) -> Option<(f64, f64)> {
    let (mut t0, mut t1) = (0.0_f64, 1.0_f64);
    for (p0, p1, lo, hi) in [
        (a.0, b.0, c.x0.min(c.x1), c.x0.max(c.x1)),
        (a.1, b.1, c.y0.min(c.y1), c.y0.max(c.y1)),
    ] {
        let d = p1 - p0;
        if d.abs() < 1e-15 {
            if p0 < lo || p0 > hi {
                return None;
            }
        } else {
            let (ta, tb) = ((lo - p0) / d, (hi - p0) / d);
            t0 = t0.max(ta.min(tb));
            t1 = t1.min(ta.max(tb));
            if t0 > t1 {
                return None;
            }
        }
    }
    // z is linear along the piece: its lowest point inside the box is at one end of the part that is inside
    let z_at = |t: f64| a.2 + (b.2 - a.2) * t;
    (z_at(t0).min(z_at(t1)) < c.height_mm)
        .then_some((a.0 + (b.0 - a.0) * t0, a.1 + (b.1 - a.1) * t0))
}

/// The sweep of an arc: its centre, its radius and the angle it starts and ends at (negative sweep for G2). `None` for a straight move or a degenerate arc.
fn arc_of(s: &Segment) -> Option<((f64, f64), f64, f64, f64)> {
    let (center, cw) = match s.motion {
        Motion::ArcCw { center } => (center, true),
        Motion::ArcCcw { center } => (center, false),
        _ => return None,
    };
    let r = ((s.from.x - center.0).powi(2) + (s.from.y - center.1).powi(2)).sqrt();
    if r < 1e-9 {
        return None;
    }
    let a0 = (s.from.y - center.1).atan2(s.from.x - center.0);
    let mut a1 = (s.to.y - center.1).atan2(s.to.x - center.0);
    if cw {
        while a1 >= a0 - 1e-12 {
            a1 -= 2.0 * std::f64::consts::PI;
        }
    } else {
        while a1 <= a0 + 1e-12 {
            a1 += 2.0 * std::f64::consts::PI;
        }
    }
    Some((center, r, a0, a1))
}

/// The rightmost, highest, leftmost and lowest points of the circle that an arc passes: where it reaches farthest, which points sampled along it can miss. Empty for a
/// straight move.
fn arc_extremes(s: &Segment) -> Vec<(f64, f64)> {
    use std::f64::consts::{FRAC_PI_2, TAU};
    let Some((center, r, a0, a1)) = arc_of(s) else {
        return Vec::new();
    };
    let sweep = a1 - a0;
    (0..4)
        .map(|k| f64::from(k) * FRAC_PI_2)
        .filter(|angle| {
            let gone = if sweep > 0.0 {
                (angle - a0).rem_euclid(TAU)
            } else {
                (a0 - angle).rem_euclid(TAU)
            };
            gone <= sweep.abs()
        })
        .map(|angle| (center.0 + r * angle.cos(), center.1 + r * angle.sin()))
        .collect()
}

/// The point of a move at `t` (0 at the start, 1 at the end): on the arc for G2 and G3, on the straight line for the rest.
pub fn point_at(s: &Segment, t: f64) -> (f64, f64, f64) {
    let z = s.from.z + (s.to.z - s.from.z) * t;
    match arc_of(s) {
        Some((center, r, a0, a1)) => {
            let a = a0 + (a1 - a0) * t;
            (center.0 + r * a.cos(), center.1 + r * a.sin(), z)
        }
        None => (
            s.from.x + (s.to.x - s.from.x) * t,
            s.from.y + (s.to.y - s.from.y) * t,
            z,
        ),
    }
}

/// How far the move goes in the XY plane (along the arc for G2 and G3).
fn plan_length(s: &Segment) -> f64 {
    match arc_of(s) {
        Some((_, r, a0, a1)) => r * (a1 - a0).abs(),
        None => ((s.to.x - s.from.x).powi(2) + (s.to.y - s.from.y).powi(2)).sqrt(),
    }
}

/// Makes every warning an alert: for a flow that uses the exit code as the barrier before the machine.
pub fn strict(findings: &mut [Finding]) {
    for f in findings.iter_mut() {
        f.level = Level::Alert;
    }
}

/// Runs every check on the trace. The findings come in the order of the program; the diagnostics of the reader become findings too.
pub fn check(trace: &Trace, limits: &Limits) -> Vec<Finding> {
    let mut out: Vec<Finding> = Vec::new();
    for d in &trace.diagnostics {
        if limits.ignore_codes.iter().any(|c| c.as_str() == d.code) {
            continue;
        }
        match d.severity {
            Severity::Info => {}
            Severity::Warning => out.push(Finding {
                line: d.line,
                level: Level::Warning,
                message: format!("the reader says: {} [{}]", d.message, d.code),
            }),
            Severity::Error => out.push(Finding {
                line: d.line,
                level: Level::Alert,
                message: format!(
                    "the reader could not understand it: {} [{}]",
                    d.message, d.code
                ),
            }),
        }
    }
    // the reader already says "no feed" for the lines it saw (or the user silenced that code): the check below only adds what the reader did not say
    let feed_told: std::collections::HashSet<usize> = trace
        .diagnostics
        .iter()
        .filter(|d| d.code == codes::FEED_MISSING)
        .map(|d| d.line)
        .collect();
    let feed_ignored = limits
        .ignore_codes
        .iter()
        .any(|c| c.as_str() == codes::FEED_MISSING);
    for s in &trace.segments {
        let rapid = matches!(s.motion, Motion::Rapid);
        let lo = s.from.z.min(s.to.z);
        if rapid
            && lo < limits.surface_z - EPS
            && (s.from.x != s.to.x || s.from.y != s.to.y || s.to.z < s.from.z)
        {
            out.push(Finding { line: s.line, level: Level::Alert, message: format!("a rapid move (G0) goes into the material: Z {:.3} is below the surface ({:.3})", lo, limits.surface_z) });
        }
        if !rapid && !s.spindle_on && lo < limits.surface_z - EPS {
            out.push(Finding {
                line: s.line,
                level: Level::Warning,
                message: "it cuts with the spindle off".into(),
            });
        }
        if !rapid && s.feed <= 0.0 && !feed_ignored && !feed_told.contains(&s.line) {
            out.push(Finding {
                line: s.line,
                level: Level::Warning,
                message: "a cutting move has no feed in the program (the default feed is used)"
                    .into(),
            });
        }
        if let Some(max) = limits.max_feed_mm_min {
            if !rapid && s.feed > max {
                out.push(Finding {
                    line: s.line,
                    level: Level::Warning,
                    message: format!(
                        "the feed {:.0} mm/min is above the limit of {:.0}",
                        s.feed, max
                    ),
                });
            }
        }
        if let Some(max) = limits.max_plunge_mm_min {
            let down = s.from.z - s.to.z;
            if !rapid && down > EPS && plan_length(s) <= down && s.feed > max {
                out.push(Finding {
                    line: s.line,
                    level: Level::Warning,
                    message: format!(
                        "the plunge of {down:.3} mm goes down at {:.0} mm/min, above the plunge limit of {max:.0}",
                        s.feed
                    ),
                });
            }
        }
        if let Some(min_z) = limits.min_z {
            if lo < min_z - EPS {
                out.push(Finding {
                    line: s.line,
                    level: Level::Alert,
                    message: format!("Z {lo:.3} is below the lowest Z of the machine ({min_z:.3})"),
                });
            }
        }
        if let Some((w, d)) = limits.table {
            // the ends, and for an arc the points along it and the farthest ones of its circle: an arc can leave the table with both ends inside
            let steps = if arc_of(s).is_some() { 64 } else { 1 };
            let outside = (0..=steps)
                .map(|k| {
                    let (x, y, _) = point_at(s, k as f64 / steps as f64);
                    (x, y)
                })
                .chain(arc_extremes(s))
                .find(|&(x, y)| x < -EPS || y < -EPS || x > w + EPS || y > d + EPS);
            if let Some((x, y)) = outside {
                out.push(Finding {
                    line: s.line,
                    level: Level::Alert,
                    message: format!("({x:.3}, {y:.3}) is outside the table of {w} x {d} mm"),
                });
            }
        }
        if !limits.clamps.is_empty() {
            // a straight move is tested whole; an arc as chords no longer than the step of the sampling
            let n = if arc_of(s).is_some() {
                ((plan_length(s) / SAMPLE_MM).ceil() as usize).clamp(1, 20_000)
            } else {
                1
            };
            'segment: for k in 0..n {
                let a = point_at(s, k as f64 / n as f64);
                let b = point_at(s, (k + 1) as f64 / n as f64);
                for (i, c) in limits.clamps.iter().enumerate() {
                    if let Some((x, y)) = segment_over_clamp(c, a, b) {
                        out.push(Finding { line: s.line, level: Level::Alert, message: format!("the head passes over the clamp {} at ({x:.2}, {y:.2}) with Z below its height {:.2}", i + 1, c.height_mm) });
                        break 'segment;
                    }
                }
            }
        }
    }
    // how the program ends: where the tool is left and whether the program is closed
    if let Some(last) = trace.segments.last() {
        if let Some(z) = limits.end_z {
            if last.to.z < z - EPS {
                out.push(Finding {
                    line: last.line,
                    level: Level::Warning,
                    message: format!(
                        "the program ends with the tool at Z {:.3}, below the end height of {z:.3}",
                        last.to.z
                    ),
                });
            }
        }
        if limits.require_end && !trace.ended {
            out.push(Finding {
                line: last.line,
                level: Level::Warning,
                message: "the program does not end with M2 or M30".into(),
            });
        }
    }
    // the notices of the reader were collected first: put every finding in the order of the program (a stable sort keeps the order inside a line)
    out.sort_by_key(|f| f.line);
    out.dedup_by(|a, b| a.line == b.line && a.message == b.message);
    out
}

/// Adds what the moves alone do not show: a spindle or a laser that is on where nothing moves. At the end of a program with no `M5`, `M2` or `M30` it stays on (an alert for a
/// laser, a warning for a spindle). With `laser`, a wait (`G4`) or a pause (`M0`, `M1`) at constant power (`M3` with `S` above 0) burns one spot and is an alert.
pub fn add_left_on(findings: &mut Vec<Finding>, program: &Program, laser: bool) {
    // the spindle: `Some(true)` at constant power (M3), `Some(false)` with M4
    let mut on: Option<bool> = None;
    let mut speed = 0.0_f64;
    let mut last = 0;
    for block in &program.blocks {
        last = block.line;
        let has = |letter: char, value: f64| {
            block
                .words
                .iter()
                .any(|w| w.letter == letter && w.value == value)
        };
        if let Some(s) = block.get('S').filter(|s| *s >= 0.0) {
            speed = s;
        }
        if has('M', 3.0) {
            on = Some(true);
        }
        if has('M', 4.0) {
            on = Some(false);
        }
        if has('M', 5.0) {
            on = None;
        }
        if laser && on == Some(true) && speed > 0.0 {
            let what = if has('G', 4.0) {
                Some("waits (G4)")
            } else if has('M', 0.0) || has('M', 1.0) {
                Some("pauses (M0 or M1)")
            } else {
                None
            };
            if let Some(what) = what {
                findings.push(Finding {
                    line: block.line,
                    level: Level::Alert,
                    message: format!(
                        "the program {what} with the laser on at constant power (M3): it burns one spot"
                    ),
                });
            }
        }
        // the end of the program stops the spindle; what comes after is told by the reader
        if has('M', 2.0) || has('M', 30.0) {
            on = None;
            break;
        }
    }
    if on.is_some() && (!laser || speed > 0.0) {
        let (level, tool) = if laser {
            (Level::Alert, "laser")
        } else {
            (Level::Warning, "spindle")
        };
        findings.push(Finding {
            line: last,
            level,
            message: format!("the program ends with the {tool} on: no M5, M2 or M30"),
        });
    }
    findings.sort_by_key(|f| f.line);
}

pub fn has_alerts(findings: &[Finding]) -> bool {
    findings.iter().any(|f| f.level == Level::Alert)
}

/// The smallest and the largest corner of a box, each as X, Y and Z.
pub type Corners = ((f64, f64, f64), (f64, f64, f64));

/// What a program is made of, in numbers: how many moves of each kind, the box that everything that moved fits in, the tools it selects and the range of the cutting feeds.
#[derive(Debug, Clone, PartialEq)]
pub struct Summary {
    pub rapid_moves: usize,
    pub straight_cuts: usize,
    pub arcs: usize,
    /// The smallest and the largest corner of everything that moved (`None` when nothing moved).
    pub extent: Option<Corners>,
    /// The numbers of the tools the program selects, from the smallest, once each.
    pub tools: Vec<u32>,
    /// The slowest and the fastest feed of the cutting moves that have one (`None` when none has).
    pub feed_range: Option<(f64, f64)>,
}

/// The summary of a trace.
pub fn summary(trace: &Trace) -> Summary {
    let mut s = Summary {
        rapid_moves: 0,
        straight_cuts: 0,
        arcs: 0,
        extent: None,
        tools: Vec::new(),
        feed_range: None,
    };
    for seg in &trace.segments {
        match seg.motion {
            Motion::Rapid => s.rapid_moves += 1,
            Motion::Linear => s.straight_cuts += 1,
            Motion::ArcCw { .. } | Motion::ArcCcw { .. } => s.arcs += 1,
        }
        if let Some(t) = seg.tool {
            if !s.tools.contains(&t) {
                s.tools.push(t);
            }
        }
        if !matches!(seg.motion, Motion::Rapid) && seg.feed > 0.0 {
            s.feed_range = Some(match s.feed_range {
                Some((lo, hi)) => (lo.min(seg.feed), hi.max(seg.feed)),
                None => (seg.feed, seg.feed),
            });
        }
    }
    s.tools.sort_unstable();
    if !trace.segments.is_empty() {
        s.extent = Some((
            (trace.min.x, trace.min.y, trace.min.z),
            (trace.max.x, trace.max.y, trace.max.z),
        ));
    }
    s
}

/// A text as a JSON string, with its quotes.
pub fn json_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn json_number(v: f64) -> String {
    if v.is_finite() {
        format!("{v}")
    } else {
        "null".into()
    }
}

/// What `check --json` prints: one JSON object with the moves, the lengths, the times, the extent, the tools, the range of the feeds and the findings (line, level and message
/// of each). The times are in seconds; `waits_s` is the time of the `G4` waits, which is not in `total_s`.
pub fn check_json(
    trace: &Trace,
    total_s: f64,
    cutting_s: f64,
    rapid_s: f64,
    waits_s: f64,
    findings: &[Finding],
) -> String {
    let s = summary(trace);
    let extent = match s.extent {
        Some((lo, hi)) => format!(
            "{{\"min\":[{},{},{}],\"max\":[{},{},{}]}}",
            json_number(lo.0),
            json_number(lo.1),
            json_number(lo.2),
            json_number(hi.0),
            json_number(hi.1),
            json_number(hi.2)
        ),
        None => "null".into(),
    };
    let feeds = match s.feed_range {
        Some((lo, hi)) => format!("[{},{}]", json_number(lo), json_number(hi)),
        None => "null".into(),
    };
    let tools: Vec<String> = s.tools.iter().map(|t| t.to_string()).collect();
    let items: Vec<String> = findings
        .iter()
        .map(|f| {
            format!(
                "{{\"line\":{},\"level\":{},\"message\":{}}}",
                f.line,
                json_string(if f.level == Level::Alert {
                    "alert"
                } else {
                    "warning"
                }),
                json_string(&f.message)
            )
        })
        .collect();
    format!(
        "{{\"moves\":{},\"rapid_moves\":{},\"straight_cuts\":{},\"arcs\":{},\"cutting_mm\":{},\"rapid_mm\":{},\"total_s\":{},\"cutting_s\":{},\"rapid_s\":{},\"waits_s\":{},\"extent\":{},\"tools\":[{}],\"feed_range\":{},\"ended\":{},\"findings\":[{}]}}",
        trace.segments.len(),
        s.rapid_moves,
        s.straight_cuts,
        s.arcs,
        json_number(trace.cutting_length_mm),
        json_number(trace.rapid_length_mm),
        json_number(total_s),
        json_number(cutting_s),
        json_number(rapid_s),
        json_number(waits_s),
        extent,
        tools.join(","),
        feeds,
        trace.ended,
        items.join(",")
    )
}
