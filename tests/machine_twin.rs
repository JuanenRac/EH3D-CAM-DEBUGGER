//! A twin of the machine that runs a program the way a controller would, with no help from the reader of the suite, and the debugger held against it: what the twin
//! hits (a rapid move into the material, a point off the table, a Z below the lowest, a cut with the spindle off) is what the debugger reports, and the time the
//! debugger gives is the time the twin takes to move. The programs come from a seeded generator, so every run is the same. Nothing here has run on a real machine.

use eh3d_cam_debugger::checks::{check, has_alerts, Level, Limits};
use eh3d_cam_debugger::timeline::{timeline, Machine};
use eh3d_parser_gcode::{interpret, parse, Dialect};
use std::collections::BTreeSet;

const TABLE: (f64, f64) = (220.0, 160.0);
const MIN_Z: f64 = -5.0;

fn limits() -> Limits {
    Limits {
        table: Some(TABLE),
        min_z: Some(MIN_Z),
        ..Limits::default()
    }
}

/// A small generator of numbers (xorshift64): the same sequence on every machine.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

/// What the twin hits, by line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Hit {
    RapidIntoMaterial,
    OffTable,
    TooLow,
    SpindleOff,
}

/// The twin: it reads the few words the generator writes and moves; the time of each move is integrated in small steps of time.
struct Twin {
    at: (f64, f64, f64),
    spindle: bool,
    feed: f64,
    hits: BTreeSet<(usize, Hit)>,
    seconds: f64,
    timed: bool,
}

impl Twin {
    fn new(timed: bool) -> Twin {
        Twin {
            timed,
            at: (0.0, 0.0, 0.0),
            spindle: false,
            feed: 0.0,
            hits: BTreeSet::new(),
            seconds: 0.0,
        }
    }

    /// The time of a move from rest to rest, by integration: accelerate to the speed, cruise, brake in time to stop where the move ends.
    fn travel(length: f64, speed_mm_min: f64, accel: f64) -> f64 {
        let (vmax, dt) = (speed_mm_min / 60.0, 0.002);
        let (mut s, mut v, mut t) = (0.0, 0.0, 0.0);
        while s < length - 1e-3 {
            let left = length - s;
            if left <= v * v / (2.0 * accel) {
                // brake so as to stop exactly where the move ends
                let need = v * v / (2.0 * left);
                v = (v - need * dt).max(0.02);
            } else {
                v = (v + accel * dt).min(vmax);
            }
            s += v * dt;
            t += dt;
        }
        t
    }

    fn run(&mut self, program: &str, machine: &Machine) {
        for (n, line) in program.lines().enumerate() {
            let number = n + 1;
            let mut rapid = None;
            let mut to = self.at;
            for word in line.split_whitespace() {
                let (letter, value) = word.split_at(1);
                let value: f64 = value.parse().unwrap_or(0.0);
                match (letter, value as i64) {
                    ("G", 0) => rapid = Some(true),
                    ("G", 1) => rapid = Some(false),
                    ("M", 3) => self.spindle = true,
                    ("M", 5) => self.spindle = false,
                    ("X", _) => to.0 = value,
                    ("Y", _) => to.1 = value,
                    ("Z", _) => to.2 = value,
                    ("F", _) => self.feed = value,
                    _ => {}
                }
            }
            let Some(rapid) = rapid else { continue };
            let from = self.at;
            let moved_xy = from.0 != to.0 || from.1 != to.1;
            let lowest = from.2.min(to.2);
            if rapid && lowest < 0.0 && (moved_xy || to.2 < from.2) {
                self.hits.insert((number, Hit::RapidIntoMaterial));
            }
            let outside =
                |p: (f64, f64, f64)| p.0 < 0.0 || p.0 > TABLE.0 || p.1 < 0.0 || p.1 > TABLE.1;
            if outside(from) || outside(to) {
                self.hits.insert((number, Hit::OffTable));
            }
            if lowest < MIN_Z {
                self.hits.insert((number, Hit::TooLow));
            }
            if !rapid && !self.spindle && lowest < 0.0 {
                self.hits.insert((number, Hit::SpindleOff));
            }
            let length =
                ((to.0 - from.0).powi(2) + (to.1 - from.1).powi(2) + (to.2 - from.2).powi(2))
                    .sqrt();
            let speed = if rapid {
                machine.rapid_mm_min
            } else {
                self.feed
            };
            if self.timed {
                self.seconds += Twin::travel(length, speed, machine.accel_mm_s2);
            }
            self.at = to;
        }
    }
}

/// What the debugger reports, by line and kind.
fn reported(program: &str) -> BTreeSet<(usize, Hit)> {
    let trace = interpret(&parse(program, Dialect::Grbl));
    check(&trace, &limits())
        .iter()
        .filter_map(|f| {
            let kind = if f.message.contains("rapid move (G0) goes into the material") {
                Hit::RapidIntoMaterial
            } else if f.message.contains("outside the table") {
                Hit::OffTable
            } else if f.message.contains("lowest Z of the machine") {
                Hit::TooLow
            } else if f.message.contains("spindle off") {
                Hit::SpindleOff
            } else {
                return None;
            };
            Some((f.line, kind))
        })
        .collect()
}

/// A program of moves on half-millimetres (so that nothing lies on the edge of the table), some of them wrong.
fn program(rng: &mut Rng) -> String {
    const ZS: [f64; 6] = [2.0, 0.5, -0.1, -1.5, -3.5, -8.5];
    let mut text = String::from("G90 G21\n");
    if rng.below(10) < 7 {
        text.push_str("M3 S10000\n");
    }
    text.push_str("G0 Z2\n");
    // a move that goes nowhere is never written, so that the reader and the twin agree on what a move is
    let mut height = 2.0;
    for _ in 0..4 + rng.below(12) {
        let x = rng.below(260) as f64 - 20.0 + 0.5;
        let y = rng.below(200) as f64 - 20.0 + 0.5;
        let z = ZS[rng.below(ZS.len())];
        let kind = rng.below(4);
        let feed = 60 + 60 * rng.below(5);
        let cut = 300 + 100 * rng.below(6);
        if (kind == 1 || kind == 2) && z == height {
            continue;
        }
        match kind {
            0 => text.push_str(&format!("G0 X{x} Y{y}\n")),
            1 => text.push_str(&format!("G0 Z{z}\n")),
            2 => text.push_str(&format!("G1 Z{z} F{feed}\n")),
            _ => text.push_str(&format!("G1 X{x} Y{y} F{cut}\n")),
        }
        if kind == 1 || kind == 2 {
            height = z;
        }
    }
    text.push_str("M5\nM2\n");
    text
}

#[test]
fn what_the_twin_hits_is_what_the_debugger_reports() {
    let machine = Machine::default();
    let mut rng = Rng(0x2545_F491_4F6C_DD1D);
    let mut kinds = BTreeSet::new();
    for n in 0..120 {
        let text = program(&mut rng);
        let mut twin = Twin::new(false);
        twin.run(&text, &machine);
        assert_eq!(reported(&text), twin.hits, "program {n}:\n{text}");
        kinds.extend(twin.hits.iter().map(|h| h.1));
    }
    // the generator really does reach all four kinds of mistake, so the comparison above means something
    assert_eq!(kinds.len(), 4, "{kinds:?}");
}

#[test]
fn the_time_of_the_debugger_is_the_time_the_twin_takes_to_move() {
    let machine = Machine::default();
    let mut rng = Rng(77);
    for n in 0..12 {
        let text = program(&mut rng);
        let mut twin = Twin::new(true);
        twin.run(&text, &machine);
        let trace = interpret(&parse(&text, Dialect::Grbl));
        let t = timeline(&trace, &machine).unwrap().total_s;
        let tolerance = 0.02 * twin.seconds + 0.1;
        assert!(
            (t - twin.seconds).abs() <= tolerance,
            "program {n}: the debugger says {t:.3} s and the twin took {:.3} s\n{text}",
            twin.seconds
        );
    }
}

/// A mistake and the program once it is put right.
const REPAIRS: [(&str, &str, &str, bool); 4] = [
    (
        "rapid into the material",
        "G90 G21\nM3 S10000\nG0 Z2\nG0 X10.5 Y10.5\nG0 Z-1.5\nM5\nM2\n",
        "G90 G21\nM3 S10000\nG0 Z2\nG0 X10.5 Y10.5\nG1 Z-1.5 F60\nM5\nM2\n",
        true,
    ),
    (
        "off the table",
        "G90 G21\nM3 S10000\nG0 Z2\nG1 X300.5 Y10.5 F200\nM5\nM2\n",
        "G90 G21\nM3 S10000\nG0 Z2\nG1 X200.5 Y10.5 F200\nM5\nM2\n",
        true,
    ),
    (
        "too low",
        "G90 G21\nM3 S10000\nG0 Z2\nG1 Z-8.5 F60\nM5\nM2\n",
        "G90 G21\nM3 S10000\nG0 Z2\nG1 Z-1.5 F60\nM5\nM2\n",
        true,
    ),
    (
        "spindle off",
        "G90 G21\nG0 Z2\nG1 Z-1.5 F60\nM2\n",
        "G90 G21\nM3 S10000\nG0 Z2\nG1 Z-1.5 F60\nM5\nM2\n",
        false,
    ),
];

#[test]
fn every_mistake_is_found_and_the_program_put_right_is_clean() {
    for (name, bad, good, alert) in REPAIRS {
        let found = reported(bad);
        assert!(!found.is_empty(), "{name}: nothing was found");
        let trace = interpret(&parse(bad, Dialect::Grbl));
        let findings = check(&trace, &limits());
        assert_eq!(has_alerts(&findings), alert, "{name}");
        assert!(
            findings.iter().any(|f| f.level == Level::Alert) == alert,
            "{name}"
        );
        assert!(
            reported(good).is_empty(),
            "{name}: still found after the repair"
        );
        let trace = interpret(&parse(good, Dialect::Grbl));
        assert!(!has_alerts(&check(&trace, &limits())), "{name}");
    }
}
