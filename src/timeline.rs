//! How long a program takes. Every segment is worked out as a move from rest to rest with a constant acceleration (a trapezoid of speed): it is an upper bound,
//! because a real controller joins the segments without stopping, so the true time is a little shorter. The waits of `G4` are not moves and are added apart (`dwell_s`).

use eh3d_parser_gcode::{Dialect, Motion, Program, Segment, Trace};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Machine {
    /// Acceleration in mm/s^2.
    pub accel_mm_s2: f64,
    /// The speed of a rapid move in mm/min.
    pub rapid_mm_min: f64,
    /// The speed used when a cutting move has no feed in the program, in mm/min.
    pub default_feed_mm_min: f64,
}

impl Default for Machine {
    fn default() -> Self {
        Machine {
            accel_mm_s2: 500.0,
            rapid_mm_min: 3000.0,
            default_feed_mm_min: 300.0,
        }
    }
}

impl Machine {
    pub fn validate(&self) -> Result<(), String> {
        for (name, v) in [
            ("accel_mm_s2", self.accel_mm_s2),
            ("rapid_mm_min", self.rapid_mm_min),
            ("default_feed_mm_min", self.default_feed_mm_min),
        ] {
            if !v.is_finite() || v <= 0.0 {
                return Err(format!("{name} is {v}: it must be more than 0"));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Step {
    pub line: usize,
    pub rapid: bool,
    pub length_mm: f64,
    pub time_s: f64,
    /// The time at which the step starts, from the start of the program.
    pub start_s: f64,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Timeline {
    pub steps: Vec<Step>,
    pub total_s: f64,
    pub cutting_s: f64,
    pub rapid_s: f64,
}

/// The length of a segment: the straight distance, or the length of the arc for G2 and G3.
pub fn length(s: &Segment) -> f64 {
    let straight =
        ((s.to.x - s.from.x).powi(2) + (s.to.y - s.from.y).powi(2) + (s.to.z - s.from.z).powi(2))
            .sqrt();
    match s.motion {
        Motion::ArcCw { center } | Motion::ArcCcw { center } => {
            let cw = matches!(s.motion, Motion::ArcCw { .. });
            let r = ((s.from.x - center.0).powi(2) + (s.from.y - center.1).powi(2)).sqrt();
            if r < 1e-9 {
                return straight;
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
            let flat = r * (a1 - a0).abs();
            flat.hypot(s.to.z - s.from.z)
        }
        _ => straight,
    }
}

/// The time of a move of `length_mm` at `speed_mm_min` from rest to rest with `accel`.
pub fn move_time(length_mm: f64, speed_mm_min: f64, accel_mm_s2: f64) -> f64 {
    if length_mm <= 0.0 {
        return 0.0;
    }
    let v = speed_mm_min / 60.0;
    let ramp = v * v / accel_mm_s2;
    if length_mm >= ramp {
        length_mm / v + v / accel_mm_s2
    } else {
        2.0 * (length_mm / accel_mm_s2).sqrt()
    }
}

pub fn timeline(trace: &Trace, machine: &Machine) -> Result<Timeline, String> {
    machine.validate()?;
    let mut t = Timeline::default();
    for s in &trace.segments {
        let rapid = matches!(s.motion, Motion::Rapid);
        let l = length(s);
        let speed = if rapid {
            machine.rapid_mm_min
        } else if s.feed > 0.0 {
            s.feed
        } else {
            machine.default_feed_mm_min
        };
        let time_s = move_time(l, speed, machine.accel_mm_s2);
        t.steps.push(Step {
            line: s.line,
            rapid,
            length_mm: l,
            time_s,
            start_s: t.total_s,
        });
        t.total_s += time_s;
        if rapid {
            t.rapid_s += time_s;
        } else {
            t.cutting_s += time_s;
        }
    }
    Ok(t)
}

/// The seconds the program waits with `G4` (a dwell), which the moves do not count: `P` in seconds for GRBL and PlanetCNC; in Marlin `P` is milliseconds and `S` seconds.
pub fn dwell_s(program: &Program) -> f64 {
    let mut total = 0.0;
    for b in &program.blocks {
        if b.words.iter().any(|w| w.letter == 'G' && w.value == 4.0) {
            let p = b.get('P').unwrap_or(0.0);
            let s = match program.dialect {
                Dialect::Marlin => p / 1000.0 + b.get('S').unwrap_or(0.0),
                _ => p,
            };
            if s.is_finite() && s > 0.0 {
                total += s;
            }
        }
    }
    total
}

/// `1 h 02 min 05 s`, `3 min 20 s` or `12.5 s`.
pub fn format_time(seconds: f64) -> String {
    if !seconds.is_finite() || seconds < 0.0 {
        return "-".to_string();
    }
    // below 59.95 s the text with one decimal still says less than a minute
    if seconds < 59.95 {
        return format!("{seconds:.1} s");
    }
    let total = seconds.round() as u64;
    let (h, m, s) = (total / 3600, (total % 3600) / 60, total % 60);
    if h > 0 {
        format!("{h} h {m:02} min {s:02} s")
    } else {
        format!("{m} min {s:02} s")
    }
}
