//! What would hurt: each finding has the line of the program, how serious it is and what it is.

use eh3d_parser_gcode::{Motion, Segment, Severity, Trace};

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

/// The point of a move at `t` (0 at the start, 1 at the end): on the arc for G2 and G3, on the straight line for the rest.
fn point_at(s: &Segment, t: f64) -> (f64, f64, f64) {
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
        if !rapid && s.feed <= 0.0 {
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
            // the ends, and for an arc the points along it: an arc can leave the table with both ends inside
            let steps = if arc_of(s).is_some() { 64 } else { 1 };
            for k in 0..=steps {
                let (x, y, _) = point_at(s, k as f64 / steps as f64);
                if x < -EPS || y < -EPS || x > w + EPS || y > d + EPS {
                    out.push(Finding {
                        line: s.line,
                        level: Level::Alert,
                        message: format!("({x:.3}, {y:.3}) is outside the table of {w} x {d} mm"),
                    });
                    break;
                }
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
    // the notices of the reader were collected first: put every finding in the order of the program (a stable sort keeps the order inside a line)
    out.sort_by_key(|f| f.line);
    out.dedup_by(|a, b| a.line == b.line && a.message == b.message);
    out
}

pub fn has_alerts(findings: &[Finding]) -> bool {
    findings.iter().any(|f| f.level == Level::Alert)
}
