//! The state of the window without a screen: the program that is open, the settings of the machine and of the checks, the trace, the time and the findings that come out of them (with
//! the same functions as the command line), the pieces to draw, the step that is shown and the status line. The window (`gui`) only draws this and passes on what the person does, so
//! everything that can be decided is decided here, in code that is tested.

use crate::checks::{
    check, has_alerts, point_at, strict as make_strict, Clamp, Finding, Level, Limits,
};
use crate::lang::{fill, Lang, Msg};
use crate::timeline::{dwell_s, format_time, length, timeline, Machine, Timeline};
use crate::viewport::Viewport;
use eh3d_parser_gcode::{
    codes, interpret_with, parse_bytes_with, read_file_limited, Dialect, InputLimits,
    InterpretOptions, Motion, Segment, Trace,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// The most pieces of path that are kept to be drawn.
pub const MAX_DRAWN: usize = 300_000;

/// The most steps a second of the playback.
pub const MAX_RATE: f64 = 1000.0;

/// A setting that is typed in a box.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Accel,
    Rapid,
    FeedDefault,
    TableWidth,
    TableDepth,
    Surface,
    MinZ,
    MaxFeed,
    Ignore,
    Clamps,
}

/// The group of the side panel a setting belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Machine,
    Limits,
    Checks,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    pub key: Key,
    pub label: Msg,
    pub section: Section,
    pub text: String,
}

fn default_fields() -> Vec<Field> {
    let machine = Machine::default();
    let f = |key, label, section, text: &str| Field {
        key,
        label,
        section,
        text: text.to_string(),
    };
    vec![
        f(
            Key::Accel,
            Msg::Accel,
            Section::Machine,
            &machine.accel_mm_s2.to_string(),
        ),
        f(
            Key::Rapid,
            Msg::Rapid,
            Section::Machine,
            &machine.rapid_mm_min.to_string(),
        ),
        f(
            Key::FeedDefault,
            Msg::FeedDefault,
            Section::Machine,
            &machine.default_feed_mm_min.to_string(),
        ),
        f(Key::TableWidth, Msg::TableWidth, Section::Limits, ""),
        f(Key::TableDepth, Msg::TableDepth, Section::Limits, ""),
        f(Key::Surface, Msg::Surface, Section::Limits, "0"),
        f(Key::MinZ, Msg::MinZ, Section::Limits, ""),
        f(Key::MaxFeed, Msg::MaxFeed, Section::Limits, ""),
        f(Key::Clamps, Msg::Clamps, Section::Limits, ""),
        f(Key::Ignore, Msg::Ignore, Section::Checks, ""),
    ]
}

/// A piece of the path of the head, seen from above (an arc is cut in several pieces); `step` is the move it belongs to.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Piece {
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
    pub step: usize,
    pub rapid: bool,
}

/// A finding on the picture: where the move it is about ends.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mark {
    pub x: f64,
    pub y: f64,
    pub level: Level,
    pub line: usize,
    pub step: usize,
}

/// The pieces of the moves to draw (arcs in short pieces) and whether the list was cut short at [`MAX_DRAWN`]. Moves with no length in the plane (a plunge) leave no piece.
pub fn pieces(trace: &Trace) -> (Vec<Piece>, bool) {
    let mut out = Vec::new();
    for (step, s) in trace.segments.iter().enumerate() {
        let rapid = matches!(s.motion, Motion::Rapid);
        let arc = matches!(s.motion, Motion::ArcCw { .. } | Motion::ArcCcw { .. });
        let n = if arc {
            ((length(s) / 0.5).ceil() as usize).clamp(4, 128)
        } else {
            1
        };
        let mut last = point_at(s, 0.0);
        for k in 1..=n {
            let p = point_at(s, k as f64 / n as f64);
            if (p.0 - last.0).hypot(p.1 - last.1) > 0.0 {
                if out.len() >= MAX_DRAWN {
                    return (out, true);
                }
                out.push(Piece {
                    x0: last.0,
                    y0: last.1,
                    x1: p.0,
                    y1: p.1,
                    step,
                    rapid,
                });
            }
            last = p;
        }
    }
    (out, false)
}

/// The program that is open.
#[derive(Debug, Clone)]
pub struct Loaded {
    pub path: PathBuf,
    pub name: String,
    pub bytes: Vec<u8>,
    pub text: String,
    /// Where each line of the text starts.
    pub starts: Vec<usize>,
}

impl Loaded {
    /// The text of a line (the first is 1), without its line end.
    pub fn line(&self, number: usize) -> &str {
        let Some(&from) = number.checked_sub(1).and_then(|i| self.starts.get(i)) else {
            return "";
        };
        let to = self.starts.get(number).copied().unwrap_or(self.text.len());
        self.text[from..to].trim_end_matches(['\r', '\n'])
    }

    pub fn lines(&self) -> usize {
        self.starts.len()
    }
}

/// What the checks found out about the program that is open.
#[derive(Debug, Clone)]
pub struct Analysis {
    pub trace: Trace,
    pub timeline: Timeline,
    pub findings: Vec<Finding>,
    pub waits_s: f64,
    pub limits: Limits,
}

impl Analysis {
    pub fn alerts(&self) -> usize {
        self.findings
            .iter()
            .filter(|f| f.level == Level::Alert)
            .count()
    }

    pub fn warnings(&self) -> usize {
        self.findings.len() - self.alerts()
    }
}

/// What the status line says.
#[derive(Debug, Clone, PartialEq)]
pub enum Status {
    Ready,
    Analysed {
        moves: usize,
        alerts: usize,
        warnings: usize,
    },
    NeedProgram,
    BadValue(Msg),
    Failed(String),
}

impl Status {
    pub fn is_problem(&self) -> bool {
        matches!(
            self,
            Status::NeedProgram | Status::BadValue(_) | Status::Failed(_)
        )
    }

    pub fn text(&self, lang: Lang) -> String {
        match self {
            Status::Ready => Msg::Ready.tr(lang).to_string(),
            Status::Analysed {
                moves,
                alerts,
                warnings,
            } => fill(
                Msg::Analysed.tr(lang),
                &[
                    &moves.to_string(),
                    &alerts.to_string(),
                    &warnings.to_string(),
                ],
            ),
            Status::NeedProgram => Msg::NeedProgram.tr(lang).to_string(),
            Status::BadValue(label) => fill(Msg::BadValue.tr(lang), &[label.tr(lang)]),
            Status::Failed(e) => fill(Msg::Failed.tr(lang), &[e]),
        }
    }
}

/// The state of the whole window.
#[derive(Debug)]
pub struct Window {
    pub lang: Lang,
    pub fields: Vec<Field>,
    pub dialect: Dialect,
    pub laser: bool,
    pub strict: bool,
    pub program: Option<Loaded>,
    pub analysis: Option<Analysis>,
    pub pieces: Vec<Piece>,
    pub marks: Vec<Mark>,
    /// The picture was cut short at [`MAX_DRAWN`].
    pub truncated: bool,
    /// The move that is shown (an index into the moves of the trace).
    pub current: usize,
    pub playing: bool,
    /// Steps a second when it plays.
    pub rate: f64,
    accumulated: f64,
    /// A setting changed after the program was analysed.
    pub stale: bool,
    pub status: Status,
    pub viewport: Viewport,
}

impl Window {
    pub fn new(lang: Lang) -> Window {
        Window {
            lang,
            fields: default_fields(),
            dialect: Dialect::Grbl,
            laser: false,
            strict: false,
            program: None,
            analysis: None,
            pieces: Vec::new(),
            marks: Vec::new(),
            truncated: false,
            current: 0,
            playing: false,
            rate: 10.0,
            accumulated: 0.0,
            stale: false,
            status: Status::Ready,
            viewport: Viewport::default(),
        }
    }

    pub fn has_work(&self) -> bool {
        self.program.is_some()
    }

    fn mark_stale(&mut self) {
        if self.analysis.is_some() {
            self.stale = true;
        }
    }

    /// Writes a setting that the person typed.
    pub fn set_field(&mut self, i: usize, text: String) {
        if let Some(f) = self.fields.get_mut(i) {
            if f.text != text {
                f.text = text;
                self.mark_stale();
            }
        }
    }

    pub fn set_dialect(&mut self, dialect: Dialect) {
        if self.dialect != dialect {
            self.dialect = dialect;
            self.mark_stale();
        }
    }

    pub fn set_laser(&mut self, on: bool) {
        if self.laser != on {
            self.laser = on;
            self.mark_stale();
        }
    }

    pub fn set_strict(&mut self, on: bool) {
        if self.strict != on {
            self.strict = on;
            self.mark_stale();
        }
    }

    fn field(&self, key: Key) -> &Field {
        // every key has its field: `default_fields` makes one for each
        self.fields
            .iter()
            .find(|f| f.key == key)
            .unwrap_or(&self.fields[0])
    }

    fn number(&self, key: Key) -> Result<f64, Status> {
        let f = self.field(key);
        f.text
            .trim()
            .parse::<f64>()
            .ok()
            .filter(|v| v.is_finite())
            .ok_or(Status::BadValue(f.label))
    }

    /// A number that may be left empty (then the check is not made).
    fn optional(&self, key: Key) -> Result<Option<f64>, Status> {
        if self.field(key).text.trim().is_empty() {
            Ok(None)
        } else {
            self.number(key).map(Some)
        }
    }

    /// The machine of the settings.
    pub fn machine(&self) -> Result<Machine, Status> {
        let m = Machine {
            accel_mm_s2: self.number(Key::Accel)?,
            rapid_mm_min: self.number(Key::Rapid)?,
            default_feed_mm_min: self.number(Key::FeedDefault)?,
        };
        m.validate().map_err(Status::Failed)?;
        Ok(m)
    }

    /// The limits of the settings: the table (both sides or none), the surface, the lowest Z, the highest feed, the clamps and the codes that are left out.
    pub fn limits(&self) -> Result<Limits, Status> {
        let table = match (
            self.optional(Key::TableWidth)?,
            self.optional(Key::TableDepth)?,
        ) {
            (Some(w), Some(d)) => Some((w, d)),
            (None, None) => None,
            (Some(_), None) => return Err(Status::BadValue(Msg::TableDepth)),
            (None, Some(_)) => return Err(Status::BadValue(Msg::TableWidth)),
        };
        let mut clamps = Vec::new();
        for line in self.field(Key::Clamps).text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let v: Vec<f64> = line
                .split(',')
                .map(|p| p.trim().parse::<f64>().ok().filter(|x| x.is_finite()))
                .collect::<Option<Vec<f64>>>()
                .filter(|v| v.len() == 5)
                .ok_or(Status::BadValue(Msg::Clamps))?;
            clamps.push(Clamp {
                x0: v[0],
                y0: v[1],
                x1: v[2],
                y1: v[3],
                height_mm: v[4],
            });
        }
        let mut ignore_codes = Vec::new();
        for c in self
            .field(Key::Ignore)
            .text
            .split(',')
            .map(str::trim)
            .filter(|c| !c.is_empty())
        {
            if !codes::ALL.contains(&c) {
                return Err(Status::BadValue(Msg::Ignore));
            }
            ignore_codes.push(c.to_string());
        }
        Ok(Limits {
            table,
            surface_z: self.number(Key::Surface)?,
            min_z: self.optional(Key::MinZ)?,
            max_feed_mm_min: self.optional(Key::MaxFeed)?,
            clamps,
            ignore_codes,
        })
    }

    /// The name of the program that is open.
    pub fn program_name(&self) -> Option<String> {
        self.program.as_ref().map(|p| p.name.clone())
    }

    /// The folder of the program that is open, where the window of files starts.
    pub fn work_folder(&self) -> Option<PathBuf> {
        self.program
            .as_ref()
            .and_then(|p| p.path.parent().map(Path::to_path_buf))
            .filter(|p| !p.as_os_str().is_empty())
    }

    /// Opens a G-code program and analyses it with the settings.
    pub fn open_program(&mut self, path: &Path) -> bool {
        let name = path.file_name().map_or_else(
            || path.display().to_string(),
            |n| n.to_string_lossy().into_owned(),
        );
        let bytes = match read_file_limited(path, &InputLimits::default()) {
            Ok(b) => b,
            Err(e) => {
                self.status = Status::Failed(format!("{}: {e}", path.display()));
                return false;
            }
        };
        let text = String::from_utf8_lossy(&bytes).into_owned();
        let mut starts = vec![0];
        starts.extend(text.match_indices('\n').map(|(i, _)| i + 1));
        // a text that ends with a line end has no line after it
        if starts.last() == Some(&text.len()) && starts.len() > 1 {
            starts.pop();
        }
        if text.is_empty() {
            starts.clear();
        }
        self.program = Some(Loaded {
            path: path.to_path_buf(),
            name,
            bytes,
            text,
            starts,
        });
        self.analysis = None;
        self.pieces.clear();
        self.marks.clear();
        self.current = 0;
        self.playing = false;
        self.stale = false;
        self.analyse()
    }

    /// The name of the program that is open and how many lines it has.
    pub fn program_info(&self) -> Option<String> {
        let p = self.program.as_ref()?;
        Some(fill(
            Msg::ProgramOpened.tr(self.lang),
            &[&p.name, &p.lines().to_string()],
        ))
    }

    /// Opens a file dropped on the window: the only thing that can be opened is a program.
    pub fn open_dropped(&mut self, path: &Path) -> bool {
        self.open_program(path)
    }

    /// Reads, interprets, times and checks the program that is open, like the command `check`.
    pub fn analyse(&mut self) -> bool {
        match self.analysed() {
            Ok(a) => {
                let (pieces, cut) = pieces(&a.trace);
                let first_step: HashMap<usize, usize> = {
                    let mut m = HashMap::new();
                    for (i, s) in a.trace.segments.iter().enumerate() {
                        m.entry(s.line).or_insert(i);
                    }
                    m
                };
                self.marks = a
                    .findings
                    .iter()
                    .filter_map(|f| {
                        let step = *first_step.get(&f.line)?;
                        let s: &Segment = &a.trace.segments[step];
                        Some(Mark {
                            x: s.to.x,
                            y: s.to.y,
                            level: f.level,
                            line: f.line,
                            step,
                        })
                    })
                    .collect();
                self.pieces = pieces;
                self.truncated = cut;
                self.status = Status::Analysed {
                    moves: a.trace.segments.len(),
                    alerts: a.alerts(),
                    warnings: a.warnings(),
                };
                self.current = self.current.min(a.trace.segments.len().saturating_sub(1));
                self.analysis = Some(a);
                self.stale = false;
                true
            }
            Err(s) => {
                self.status = s;
                false
            }
        }
    }

    fn analysed(&self) -> Result<Analysis, Status> {
        let program = self.program.as_ref().ok_or(Status::NeedProgram)?;
        let machine = self.machine()?;
        let limits = self.limits()?;
        let parsed = parse_bytes_with(&program.bytes, self.dialect, &InputLimits::default())
            .map_err(|e| Status::Failed(format!("{}: {e}", program.name)))?;
        let trace = interpret_with(
            &parsed,
            &InterpretOptions {
                s0_is_off: self.laser,
            },
        );
        let tl = timeline(&trace, &machine).map_err(Status::Failed)?;
        let mut findings = check(&trace, &limits);
        if self.strict {
            make_strict(&mut findings);
        }
        Ok(Analysis {
            waits_s: dwell_s(&parsed),
            trace,
            timeline: tl,
            findings,
            limits,
        })
    }

    /// How many moves there are to step through.
    pub fn steps(&self) -> usize {
        self.analysis.as_ref().map_or(0, |a| a.trace.segments.len())
    }

    /// The move that is shown.
    pub fn segment(&self) -> Option<&Segment> {
        self.analysis.as_ref()?.trace.segments.get(self.current)
    }

    /// Goes to a move (held between the first and the last).
    pub fn set_step(&mut self, i: usize) {
        self.current = i.min(self.steps().saturating_sub(1));
    }

    pub fn first(&mut self) {
        self.set_step(0);
    }

    pub fn last(&mut self) {
        self.set_step(usize::MAX);
    }

    pub fn next(&mut self) {
        self.set_step(self.current.saturating_add(1));
    }

    pub fn prev(&mut self) {
        self.set_step(self.current.saturating_sub(1));
    }

    /// Goes to the first move of a line of the program; false when no move comes from that line.
    pub fn go_to_line(&mut self, line: usize) -> bool {
        let found = self
            .analysis
            .as_ref()
            .and_then(|a| a.trace.segments.iter().position(|s| s.line == line));
        match found {
            Some(i) => {
                self.current = i;
                true
            }
            None => false,
        }
    }

    /// Goes to the move a finding is about; false when the finding is not about a move.
    pub fn go_to_finding(&mut self, i: usize) -> bool {
        let line = self
            .analysis
            .as_ref()
            .and_then(|a| a.findings.get(i))
            .map(|f| f.line);
        line.is_some_and(|l| self.go_to_line(l))
    }

    pub fn toggle_play(&mut self) {
        if self.steps() == 0 {
            self.playing = false;
        } else if self.playing {
            self.playing = false;
        } else {
            // from the end, playing starts again from the first move
            if self.current + 1 >= self.steps() {
                self.current = 0;
            }
            self.accumulated = 0.0;
            self.playing = true;
        }
    }

    pub fn set_rate(&mut self, rate: f64) {
        self.rate = if rate.is_finite() {
            rate.clamp(0.1, MAX_RATE)
        } else {
            10.0
        };
    }

    /// Lets `seconds` go by while it plays: the shown move goes forward at the rate chosen, and the playing stops at the last move.
    pub fn tick(&mut self, seconds: f64) {
        if !self.playing || !seconds.is_finite() || seconds <= 0.0 {
            return;
        }
        self.accumulated += seconds * self.rate;
        let whole = self.accumulated.floor();
        self.accumulated -= whole;
        let target = self.current.saturating_add(whole as usize);
        let last = self.steps().saturating_sub(1);
        self.current = target.min(last);
        if self.current >= last {
            self.playing = false;
        }
    }

    /// The text of the line of the program that the shown move comes from.
    pub fn current_line_text(&self) -> Option<String> {
        let s = self.segment()?;
        Some(self.program.as_ref()?.line(s.line).trim().to_string())
    }

    /// The facts about the move that is shown, as lines of text.
    pub fn step_lines(&self) -> Vec<String> {
        let (Some(a), Some(s)) = (&self.analysis, self.segment()) else {
            return Vec::new();
        };
        let lang = self.lang;
        let step = a.timeline.steps.get(self.current);
        let mut out = vec![
            fill(
                Msg::StepOf.tr(lang),
                &[&(self.current + 1).to_string(), &self.steps().to_string()],
            ),
            fill(
                Msg::StepLine.tr(lang),
                &[
                    &s.line.to_string(),
                    &self.current_line_text().unwrap_or_default(),
                ],
            ),
        ];
        let p = |v: f64| format!("{v:.3}");
        out.push(fill(
            Msg::StepFrom.tr(lang),
            &[&p(s.from.x), &p(s.from.y), &p(s.from.z)],
        ));
        out.push(fill(
            Msg::StepTo.tr(lang),
            &[&p(s.to.x), &p(s.to.y), &p(s.to.z)],
        ));
        if let Some(t) = step {
            let kind = if t.rapid {
                Msg::StepKindRapid
            } else {
                Msg::StepKindCut
            };
            out.push(fill(
                kind.tr(lang),
                &[
                    &format!("{:.3}", t.length_mm),
                    &format!("{:.2}", t.time_s),
                    &format_time(t.start_s),
                ],
            ));
        }
        if !matches!(s.motion, Motion::Rapid) {
            let feed = if s.feed > 0.0 {
                s.feed
            } else {
                self.number(Key::FeedDefault).unwrap_or(0.0)
            };
            let msg = if s.spindle_on {
                Msg::StepFeedOn
            } else {
                Msg::StepFeedOff
            };
            out.push(fill(msg.tr(lang), &[&format!("{feed:.0}")]));
        }
        out
    }

    /// The overview of the program: the moves and the lengths, the time and the waits.
    pub fn overview_lines(&self) -> Vec<String> {
        let Some(a) = &self.analysis else {
            return Vec::new();
        };
        let lang = self.lang;
        let mut out = vec![
            fill(
                Msg::Overview.tr(lang),
                &[
                    &a.trace.segments.len().to_string(),
                    &format!("{:.1}", a.trace.cutting_length_mm),
                    &format!("{:.1}", a.trace.rapid_length_mm),
                ],
            ),
            fill(
                Msg::TimeLine.tr(lang),
                &[
                    &format_time(a.timeline.total_s),
                    &format_time(a.timeline.cutting_s),
                    &format_time(a.timeline.rapid_s),
                ],
            ),
        ];
        if a.waits_s > 0.0 {
            out.push(fill(
                Msg::WaitsLine.tr(lang),
                &[
                    &format_time(a.waits_s),
                    &format_time(a.timeline.total_s + a.waits_s),
                ],
            ));
        }
        out
    }

    /// One finding as a line of text, like the command `check` prints it.
    pub fn finding_text(&self, f: &Finding) -> String {
        let level = match f.level {
            Level::Alert => Msg::LegendAlert,
            Level::Warning => Msg::LegendWarning,
        };
        fill(
            Msg::FindingLine.tr(self.lang),
            &[level.tr(self.lang), &f.line.to_string(), &f.message],
        )
    }

    /// The exit code the command line would give for this program: 1 with an alert, 2 with only warnings, 0 with nothing.
    pub fn exit_code(&self) -> Option<u8> {
        let a = self.analysis.as_ref()?;
        Some(if has_alerts(&a.findings) {
            1
        } else if a.findings.is_empty() {
            0
        } else {
            2
        })
    }

    /// The area the work covers: the moves, the table and the clamps.
    pub fn bounds(&self) -> Option<((f64, f64), (f64, f64))> {
        let a = self.analysis.as_ref()?;
        let mut min = (f64::INFINITY, f64::INFINITY);
        let mut max = (f64::NEG_INFINITY, f64::NEG_INFINITY);
        let mut grow = |x: f64, y: f64| {
            min = (min.0.min(x), min.1.min(y));
            max = (max.0.max(x), max.1.max(y));
        };
        if !a.trace.segments.is_empty() {
            grow(a.trace.min.x, a.trace.min.y);
            grow(a.trace.max.x, a.trace.max.y);
        }
        if let Some((w, d)) = a.limits.table {
            grow(0.0, 0.0);
            grow(w, d);
        }
        for c in &a.limits.clamps {
            grow(c.x0, c.y0);
            grow(c.x1, c.y1);
        }
        (min.0 <= max.0 && min.1 <= max.1).then_some((min, max))
    }

    /// Shows everything in a picture of the size given.
    pub fn fit(&mut self, width_px: f64, height_px: f64) {
        if let Some((min, max)) = self.bounds() {
            self.viewport = Viewport::fit(min, max, width_px, height_px, 24.0);
        }
    }

    /// What the pointer is over.
    pub fn cursor_text(&self, x: f64, y: f64) -> String {
        fill(
            Msg::CursorPlain.tr(self.lang),
            &[&format!("{x:.2}"), &format!("{y:.2}")],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// A plate with a rapid move sideways inside the material at the end (line 8).
    const PROGRAM: &str = "G21 G90\nM3 S12000\nG0 Z5\nG0 X10 Y10\nG1 Z-1 F100\nG1 X20 Y10 F200\nG1 X20 Y20\nG0 X30 Y20\nM5\nM2\n";

    /// A folder of its own for each test, removed at the end.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new(tag: &str) -> Scratch {
            let dir = std::env::temp_dir()
                .join(format!("eh3d-debugger-window-{}-{tag}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).unwrap();
            Scratch(dir)
        }

        fn write(&self, name: &str, text: &str) -> PathBuf {
            let p = self.0.join(name);
            fs::write(&p, text).unwrap();
            p
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn set(w: &mut Window, key: Key, text: &str) {
        let i = w.fields.iter().position(|f| f.key == key).unwrap();
        w.set_field(i, text.to_string());
    }

    fn ready(tag: &str) -> (Scratch, Window) {
        let s = Scratch::new(tag);
        let p = s.write("job.nc", PROGRAM);
        let mut w = Window::new(Lang::En);
        assert!(w.open_program(&p));
        (s, w)
    }

    #[test]
    fn opening_a_program_reads_it_and_checks_it_at_once() {
        let (_s, w) = ready("open");
        assert_eq!(w.program_name().as_deref(), Some("job.nc"));
        assert_eq!(w.program.as_ref().unwrap().lines(), 10);
        assert!(w.steps() >= 4);
        assert!(matches!(w.status, Status::Analysed { .. }));
        assert!(w.has_work() && !w.stale);
    }

    #[test]
    fn the_window_finds_what_the_command_line_finds() {
        let (_s, w) = ready("same");
        let parsed =
            parse_bytes_with(PROGRAM.as_bytes(), Dialect::Grbl, &InputLimits::default()).unwrap();
        let trace = interpret_with(&parsed, &InterpretOptions::default());
        let expected = check(&trace, &Limits::default());
        let a = w.analysis.as_ref().unwrap();
        assert_eq!(a.findings, expected);
        let tl = timeline(&trace, &Machine::default()).unwrap();
        assert_eq!(a.timeline, tl);
        assert_eq!(a.trace, trace);
        // the rapid move sideways inside the material is an alert on line 8
        assert!(a
            .findings
            .iter()
            .any(|f| f.line == 8 && f.level == Level::Alert));
        assert_eq!(w.exit_code(), Some(1));
        assert_eq!(a.timeline.steps.len(), a.trace.segments.len());
    }

    #[test]
    fn a_program_with_nothing_to_say_has_exit_code_zero() {
        let s = Scratch::new("clean");
        let p = s.write("ok.nc", "G21 G90\nM3 S1000\nG0 Z5\nG0 X1 Y1\nM5\nM2\n");
        let mut w = Window::new(Lang::En);
        assert!(w.open_program(&p));
        assert_eq!(w.exit_code(), Some(0));
        assert_eq!(w.analysis.as_ref().unwrap().findings.len(), 0);
    }

    #[test]
    fn a_file_that_is_not_there_is_an_error_with_its_name() {
        let mut w = Window::new(Lang::En);
        assert!(!w.open_program(Path::new("/no/such/folder/job.nc")));
        assert!(w.program.is_none());
        assert!(w.status.is_problem());
        assert!(w.status.text(Lang::En).contains("job.nc"));
        assert!(!w.analyse());
        assert_eq!(w.status, Status::NeedProgram);
    }

    #[test]
    fn the_text_of_a_line_is_found_by_its_number() {
        let (_s, w) = ready("lines");
        let p = w.program.as_ref().unwrap();
        assert_eq!(p.line(1), "G21 G90");
        assert_eq!(p.line(8), "G0 X30 Y20");
        assert_eq!(p.line(10), "M2");
        assert_eq!(p.line(0), "");
        assert_eq!(p.line(11), "");
    }

    #[test]
    fn a_program_with_windows_line_ends_and_no_last_line_end_is_read_the_same() {
        let s = Scratch::new("crlf");
        let p = s.write("w.nc", "G0 X1\r\nG1 X2 F100");
        let mut w = Window::new(Lang::En);
        assert!(w.open_program(&p));
        let l = w.program.as_ref().unwrap();
        assert_eq!(l.lines(), 2);
        assert_eq!(l.line(1), "G0 X1");
        assert_eq!(l.line(2), "G1 X2 F100");
        let e = s.write("empty.nc", "");
        assert!(w.open_program(&e));
        assert_eq!(w.program.as_ref().unwrap().lines(), 0);
        assert_eq!(w.steps(), 0);
        assert_eq!(w.bounds(), None);
    }

    #[test]
    fn stepping_goes_through_the_moves_and_stops_at_both_ends() {
        let (_s, mut w) = ready("steps");
        let last = w.steps() - 1;
        assert_eq!(w.current, 0);
        w.prev();
        assert_eq!(w.current, 0);
        w.next();
        assert_eq!(w.current, 1);
        w.last();
        assert_eq!(w.current, last);
        w.next();
        assert_eq!(w.current, last);
        w.set_step(usize::MAX);
        assert_eq!(w.current, last);
        w.first();
        assert_eq!(w.current, 0);
    }

    #[test]
    fn a_line_and_a_finding_take_the_view_to_their_move() {
        let (_s, mut w) = ready("goto");
        assert!(w.go_to_line(8));
        assert_eq!(w.segment().unwrap().line, 8);
        assert!(!w.go_to_line(1));
        let i = w
            .analysis
            .as_ref()
            .unwrap()
            .findings
            .iter()
            .position(|f| f.line == 8)
            .unwrap();
        w.first();
        assert!(w.go_to_finding(i));
        assert_eq!(w.segment().unwrap().line, 8);
        assert!(!w.go_to_finding(999));
    }

    #[test]
    fn every_finding_about_a_move_is_marked_where_the_move_ends() {
        let (_s, w) = ready("marks");
        let m = w.marks.iter().find(|m| m.line == 8).unwrap();
        assert_eq!((m.x, m.y), (30.0, 20.0));
        assert_eq!(m.level, Level::Alert);
        assert_eq!(w.analysis.as_ref().unwrap().trace.segments[m.step].line, 8);
    }

    #[test]
    fn playing_goes_forward_at_the_rate_and_stops_at_the_end() {
        let (_s, mut w) = ready("play");
        w.set_rate(10.0);
        w.toggle_play();
        assert!(w.playing);
        w.tick(0.25);
        assert_eq!(w.current, 2);
        w.tick(0.04);
        assert_eq!(w.current, 2, "part of a step is kept for later");
        w.tick(0.02);
        assert_eq!(w.current, 3);
        w.tick(100.0);
        assert_eq!(w.current, w.steps() - 1);
        assert!(!w.playing);
        // from the end, playing starts again from the first move
        w.toggle_play();
        assert!(w.playing);
        assert_eq!(w.current, 0);
        w.toggle_play();
        assert!(!w.playing);
        w.tick(1.0);
        assert_eq!(w.current, 0);
    }

    #[test]
    fn the_rate_is_held_between_the_limits_and_nonsense_gives_the_default() {
        let mut w = Window::new(Lang::En);
        w.set_rate(1e9);
        assert_eq!(w.rate, MAX_RATE);
        w.set_rate(-3.0);
        assert_eq!(w.rate, 0.1);
        w.set_rate(f64::NAN);
        assert_eq!(w.rate, 10.0);
        w.toggle_play();
        assert!(!w.playing, "nothing to play");
    }

    #[test]
    fn a_setting_that_changes_after_the_analysis_makes_it_stale() {
        let (_s, mut w) = ready("stale");
        set(&mut w, Key::Accel, "800");
        assert!(w.stale);
        assert!(w.analyse());
        assert!(!w.stale);
        w.set_laser(true);
        assert!(w.stale);
        assert!(w.analyse());
        w.set_strict(true);
        w.set_dialect(Dialect::Marlin);
        assert!(w.stale);
        assert!(w.analyse());
    }

    #[test]
    fn a_table_that_is_too_small_gives_findings() {
        let (_s, mut w) = ready("table");
        let before = w.analysis.as_ref().unwrap().findings.len();
        set(&mut w, Key::TableWidth, "25");
        set(&mut w, Key::TableDepth, "25");
        assert!(w.analyse());
        assert!(w.analysis.as_ref().unwrap().findings.len() > before);
        // only one side of the table is not a table
        set(&mut w, Key::TableDepth, "");
        assert!(!w.analyse());
        assert_eq!(w.status, Status::BadValue(Msg::TableDepth));
        set(&mut w, Key::TableWidth, "");
        set(&mut w, Key::TableDepth, "25");
        assert!(!w.analyse());
        assert_eq!(w.status, Status::BadValue(Msg::TableWidth));
    }

    #[test]
    fn strict_makes_every_warning_an_alert() {
        let s = Scratch::new("strict");
        // a cut with the spindle off is a warning
        let p = s.write("w.nc", "G21 G90\nG0 Z5\nG1 Z-1 F100\nG1 X5 F100\nM2\n");
        let mut w = Window::new(Lang::En);
        assert!(w.open_program(&p));
        assert_eq!(w.exit_code(), Some(2));
        w.set_strict(true);
        assert!(w.analyse());
        assert_eq!(w.exit_code(), Some(1));
        assert_eq!(w.analysis.as_ref().unwrap().warnings(), 0);
    }

    #[test]
    fn clamps_are_read_one_to_a_line_and_a_wrong_one_is_refused() {
        let (_s, mut w) = ready("clamps");
        set(
            &mut w,
            Key::Clamps,
            "# a clamp\n5,5,15,15,10\n\n 20 , 0 , 25 , 5 , 3 ",
        );
        let limits = w.limits().unwrap();
        assert_eq!(limits.clamps.len(), 2);
        assert_eq!(limits.clamps[0].height_mm, 10.0);
        assert!(w.analyse());
        assert_eq!(w.analysis.as_ref().unwrap().limits.clamps.len(), 2);
        set(&mut w, Key::Clamps, "1,2,3");
        assert_eq!(w.limits().unwrap_err(), Status::BadValue(Msg::Clamps));
        set(&mut w, Key::Clamps, "1,2,3,4,x");
        assert_eq!(w.limits().unwrap_err(), Status::BadValue(Msg::Clamps));
    }

    #[test]
    fn the_codes_to_leave_out_must_be_codes_of_the_reader() {
        let mut w = Window::new(Lang::En);
        set(&mut w, Key::Ignore, "no-such-code");
        assert_eq!(w.limits().unwrap_err(), Status::BadValue(Msg::Ignore));
        let code = codes::ALL[0];
        set(&mut w, Key::Ignore, &format!("{code}, {code}"));
        assert_eq!(w.limits().unwrap().ignore_codes.len(), 2);
    }

    #[test]
    fn a_machine_that_cannot_move_is_refused_with_the_reason() {
        let mut w = Window::new(Lang::En);
        set(&mut w, Key::Accel, "0");
        assert!(matches!(w.machine().unwrap_err(), Status::Failed(_)));
        set(&mut w, Key::Accel, "fast");
        assert_eq!(w.machine().unwrap_err(), Status::BadValue(Msg::Accel));
        set(&mut w, Key::Accel, "500");
        set(&mut w, Key::MinZ, "low");
        assert_eq!(w.limits().unwrap_err(), Status::BadValue(Msg::MinZ));
    }

    #[test]
    fn the_pieces_of_the_picture_leave_out_the_plunges_and_follow_the_arcs() {
        let s = Scratch::new("arcs");
        let p = s.write(
            "a.nc",
            "G21 G90\nM3 S1000\nG0 X10 Y0\nG1 Z-1 F100\nG2 X0 Y10 I-10 J0 F200\nG0 Z5\nM2\n",
        );
        let mut w = Window::new(Lang::En);
        assert!(w.open_program(&p));
        assert!(!w.truncated);
        // the arc is several pieces of the same move, and a plunge or a lift has none
        let arc_step = w
            .analysis
            .as_ref()
            .unwrap()
            .trace
            .segments
            .iter()
            .position(|s| matches!(s.motion, Motion::ArcCw { .. }))
            .unwrap();
        let arc_pieces = w.pieces.iter().filter(|p| p.step == arc_step).count();
        assert!(arc_pieces >= 4, "{arc_pieces}");
        let plunge = w
            .analysis
            .as_ref()
            .unwrap()
            .trace
            .segments
            .iter()
            .position(|s| s.to.z < s.from.z && s.from.x == s.to.x && s.from.y == s.to.y)
            .unwrap();
        assert!(w.pieces.iter().all(|p| p.step != plunge));
        // the pieces of the arc join and end where the move ends
        let last = w.pieces.iter().rfind(|p| p.step == arc_step).unwrap();
        assert!((last.x1 - 0.0).abs() < 1e-6 && (last.y1 - 10.0).abs() < 1e-6);
    }

    #[test]
    fn a_program_of_too_many_moves_is_cut_short_for_the_picture() {
        let mut text = String::from("G21 G90\nM3 S1000\n");
        for k in 0..(MAX_DRAWN / 2 + 5) {
            text.push_str(&format!("G1 X{} Y0 F100\nG1 X{} Y1 F100\n", k + 1, k + 1));
        }
        let s = Scratch::new("many");
        let p = s.write("big.nc", &text);
        let mut w = Window::new(Lang::En);
        assert!(w.open_program(&p));
        assert!(w.truncated);
        assert_eq!(w.pieces.len(), MAX_DRAWN);
    }

    #[test]
    fn the_shown_move_is_told_in_lines() {
        let (_s, mut w) = ready("lines2");
        assert!(w.go_to_line(6));
        let lines = w.step_lines();
        assert!(lines[0].starts_with("Step "));
        assert!(lines.iter().any(|l| l == "Line 6: G1 X20 Y10 F200"));
        assert!(lines
            .iter()
            .any(|l| l.starts_with("From X 10.000 Y 10.000 Z -1.000")));
        assert!(lines
            .iter()
            .any(|l| l.starts_with("To X 20.000 Y 10.000 Z -1.000")));
        assert!(lines
            .iter()
            .any(|l| l.starts_with("Cutting move, 10.000 mm")));
        assert!(lines.iter().any(|l| l == "Feed 200 mm/min, spindle on"));
        assert!(w.go_to_line(8));
        assert!(w
            .step_lines()
            .iter()
            .any(|l| l.starts_with("Rapid move, 10.000 mm")));
        assert!(Window::new(Lang::En).step_lines().is_empty());
    }

    #[test]
    fn the_overview_has_the_time_of_the_command_line() {
        let (_s, w) = ready("overview");
        let lines = w.overview_lines();
        assert!(lines[0].contains("moves:"));
        let total = w.analysis.as_ref().unwrap().timeline.total_s;
        assert!(lines[1].contains(&format_time(total)));
        assert_eq!(lines.len(), 2, "no waits in this program");
        let s = Scratch::new("waits");
        let p = s.write("d.nc", "G21 G90\nM3 S1000\nG0 X1\nG4 P2\nG0 X2\nM5\nM2\n");
        let mut d = Window::new(Lang::En);
        assert!(d.open_program(&p));
        assert_eq!(d.overview_lines().len(), 3);
        assert!(d.overview_lines()[2].starts_with("Waits (G4): 2.0 s"));
    }

    #[test]
    fn a_finding_is_written_like_the_command_line_writes_it() {
        let (_s, w) = ready("finding");
        let f = w
            .analysis
            .as_ref()
            .unwrap()
            .findings
            .iter()
            .find(|f| f.line == 8)
            .unwrap();
        let text = w.finding_text(f);
        assert!(text.starts_with("Alert line 8: "));
        assert!(text.ends_with(&f.message));
    }

    #[test]
    fn the_picture_fits_the_moves_the_table_and_the_clamps() {
        let (_s, mut w) = ready("fit");
        let ((x0, y0), (x1, y1)) = w.bounds().unwrap();
        assert!(x0 <= 10.0 && y0 <= 10.0 && x1 >= 30.0 && y1 >= 20.0);
        set(&mut w, Key::Clamps, "100,100,120,110,5");
        assert!(w.analyse());
        let ((_, _), (x1, y1)) = w.bounds().unwrap();
        assert!(x1 >= 120.0 && y1 >= 110.0);
        w.fit(800.0, 600.0);
        let (x, y) = w.viewport.to_screen(30.0, 20.0, 800.0, 600.0);
        assert!((0.0..=800.0).contains(&x) && (0.0..=600.0).contains(&y));
    }

    #[test]
    fn the_status_speaks_the_language_chosen() {
        let s = Status::Analysed {
            moves: 5,
            alerts: 1,
            warnings: 2,
        };
        assert_eq!(s.text(Lang::En), "5 moves, 1 alerts, 2 warnings");
        assert_eq!(s.text(Lang::Es), "5 movimientos, 1 alertas, 2 advertencias");
        assert!(Status::BadValue(Msg::Accel)
            .text(Lang::De)
            .contains("Beschleunigung"));
        assert_eq!(
            Status::NeedProgram.text(Lang::Pt),
            "Abra primeiro um programa G-code"
        );
    }
}
