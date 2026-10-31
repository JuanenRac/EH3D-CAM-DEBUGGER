//! What would hurt: each finding has the line of the program, how serious it is and what it is.

use eh3d_parser_gcode::{Motion, Severity, Trace};

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
}

impl Default for Limits {
    fn default() -> Self {
        Limits {
            table: None,
            surface_z: 0.0,
            min_z: None,
            max_feed_mm_min: None,
            clamps: Vec::new(),
        }
    }
}

const SAMPLE_MM: f64 = 0.5;
const EPS: f64 = 1e-9;

fn over_clamp(c: &Clamp, x: f64, y: f64, z: f64) -> bool {
    x >= c.x0.min(c.x1)
        && x <= c.x0.max(c.x1)
        && y >= c.y0.min(c.y1)
        && y <= c.y0.max(c.y1)
        && z < c.height_mm
}

/// Runs every check on the trace. The findings come in the order of the program; the diagnostics of the reader become findings too.
pub fn check(trace: &Trace, limits: &Limits) -> Vec<Finding> {
    let mut out: Vec<Finding> = Vec::new();
    for d in &trace.diagnostics {
        match d.severity {
            Severity::Info => {}
            Severity::Warning => out.push(Finding {
                line: d.line,
                level: Level::Warning,
                message: format!("the reader says: {}", d.message),
            }),
            Severity::Error => out.push(Finding {
                line: d.line,
                level: Level::Alert,
                message: format!("the reader could not understand it: {}", d.message),
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
            for p in [s.from, s.to] {
                if p.x < -EPS || p.y < -EPS || p.x > w + EPS || p.y > d + EPS {
                    out.push(Finding {
                        line: s.line,
                        level: Level::Alert,
                        message: format!(
                            "({:.3}, {:.3}) is outside the table of {w} x {d} mm",
                            p.x, p.y
                        ),
                    });
                    break;
                }
            }
        }
        if !limits.clamps.is_empty() {
            let len = ((s.to.x - s.from.x).powi(2) + (s.to.y - s.from.y).powi(2)).sqrt();
            let n = ((len / SAMPLE_MM).ceil() as usize).clamp(1, 20_000);
            'segment: for k in 0..=n {
                let t = k as f64 / n as f64;
                let (x, y, z) = (
                    s.from.x + (s.to.x - s.from.x) * t,
                    s.from.y + (s.to.y - s.from.y) * t,
                    s.from.z + (s.to.z - s.from.z) * t,
                );
                for (i, c) in limits.clamps.iter().enumerate() {
                    if over_clamp(c, x, y, z) {
                        out.push(Finding { line: s.line, level: Level::Alert, message: format!("the head passes over the clamp {} at ({x:.2}, {y:.2}) with Z {z:.2} below its height {:.2}", i + 1, c.height_mm) });
                        break 'segment;
                    }
                }
            }
        }
    }
    out.dedup_by(|a, b| a.line == b.line && a.message == b.message);
    out
}

pub fn has_alerts(findings: &[Finding]) -> bool {
    findings.iter().any(|f| f.level == Level::Alert)
}
