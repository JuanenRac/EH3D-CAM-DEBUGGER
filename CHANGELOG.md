# Changelog

All notable changes to this project are documented here.

## [Unreleased]

- **The CI validation checks the workflows and the lessons of past CI failures against a baseline:** `tools/workflow_hygiene.py` and `tools/lessons_lint.py` (copies of the shared tools) run in the validation, and only findings that are not in `tools/workflow_hygiene_baseline.json` and `tools/lessons_lint_baseline.json` fail it, so what exists today stays as it is and a new one is caught. `python tools/workflow_hygiene.py --root . --detail` and `python tools/lessons_lint.py --repo .` list the findings.
- **The validation of the CI is the reusable one of the documentation hub:** the job `validate` calls `validate.yml` of EH3D-DOCS (pinned by commit) and the check of the CHANGELOG is a job of its own.
## [0.0.5] - The window that plays a toolpath move by move, the shared reader versions and the Windows icon

- **Ready-made programs for every system:** the `Release binaries` workflow, started by hand by the owner from main, builds the window version for Windows (64 and 32 bits), Linux and macOS (Apple silicon and Intel), packs each with the licence, the README, the icon and the examples, writes the SHA-256 of every package, `SHA256SUMS` and a build manifest, attests their provenance, checks the result as a stranger would and, only when the dry run is switched off and the owner approves, prepares a draft release; the README has a Downloads section that tells how the signed packages are checked.
- **The dependencies on the shared readers name the version that has what is used:** the G-code crate (and for ISOCAM the Excellon one) is required from 0.0.7, the version that brought the way of writing a number, the header of a program and the units given by the caller.
- **A window:** built with `--features gui`, `eh3d-camdebug` with no arguments (or with one program, or `gui [FILE]`) opens a window in the look of the other programs of the suite (amber on charcoal, the icon of the program). A G-code program is opened from the list of files of the window or dropped on it and checked at once with the same functions as the command line; its path is drawn from above (the moves made are bright, the ones to come are dim, the table and the clamps are drawn, the alerts and warnings are marked where the move ends) with zoom (wheel) and pan (drag); the moves are gone through one by one, with buttons, a slider, a table of all the moves (line, kind, length, time) or by playing them at a speed; a click on a finding or on a line of the table goes to that move; and the acceleration, the speeds, the dialect, the laser mode, the strict mode, the table, the surface, the lowest Z, the highest feed, the clamps and the notices left out can be edited. The command line works as before, and a build without the feature is only a command line.
- **The window speaks the eight languages:** English, Spanish, German, French, Italian, Japanese, Chinese and Portuguese of Portugal, chosen from the system and changeable in the window.
- **The logic of the window needs no screen:** `window` (program, settings, analysis, pieces of the path, steps, playing, status), `viewport` (zoom and pan), `browse` (the list of files) and `lang` (the words) are plain code with their own tests; `gui` only draws them. `checks::point_at` is public for the drawing of the arcs.
- **The Rust version needed is 1.88,** the one the window needs.
- **The exit codes follow the convention of the suite:** 0 when there is nothing to say, 1 for an alert or a failure (a file that cannot be read, a limit), 2 when there are only warnings and 64 for a wrong option or command; `--strict` turns the warnings into 1.
- **A missing feed is told once for each move:** the check no longer repeats what the reader says, and `--ignore feed-missing` silences it.
- **The laser mode is read:** `--laser` reads `M3` or `M4` with `S0` as the laser being off, so a cut with the laser off is told.
- **The application icon and the logos of the program:** `images/brand/` holds the icon (SVG, PNG sizes, `.ico` and `.icns`), the horizontal and vertical logos in a light and a dark version and the social preview image.
- **Shared checks updated:** the validator also compares the badge and the Version row of the README with the manifest, fails when one of its checks cannot run, and the CHANGELOG step of the workflow reads the base branch from the environment.
- **Unknown commands first:** an unknown command is refused before the file is read.
- **Limited reading:** the program is refused by its size before it is read into memory, with the message in the eight languages.
- **Waits are counted:** the dwells of `G4` are added apart from the moves in the time report, so a program with a wait at every hole no longer understates its time.
- **The workflows are hardened:** every action is pinned by its commit with its version in a comment, and the checkouts keep no credentials.
- **More tests:** the times of the moves and how they are written, the machine that is refused, the length of arcs and helices, and the findings that become alerts.
- **A picture mode for the window:** `--shot FILE.png` (built with `--features gui`) opens the window at the size of `--size WxH` (320 to 8192, 1280x780 by default), with `--theme forge|dark|light`, `--lang CODE` and the files of `--open FILE`, waits `--wait N` frames (0 to 3600, 6 by default), saves what it drew as a PNG file and closes by itself. An existing file is never replaced unless `--replace` is given, the window keeps no settings in this mode, the exit code is 0 when the file was written, 1 when it was not (also when the picture does not come back within 600 frames) and 2 for wrong arguments, and a build without the window says so. The arguments are read by plain code with its tests.

## [0.0.4] - CI hardening and shared validator

- **Faster, bounded CI:** the Cargo build is cached, a run that a newer push supersedes is cancelled, every job has a time limit, and the Rust jobs live in their own workflow that is skipped when only documents or images change.
- **A fixed Rust toolchain:** `rust-toolchain.toml` names the compiler version (with rustfmt and clippy) that the CI installs, so a new stable release cannot turn the lints red without a change here.
- **Monthly dependency updates:** Dependabot opens one grouped pull request a month for the GitHub Actions.
- **A line for every change:** a pull request that changes code must add a line under `[Unreleased]` of this file (`tools/changelog_pr_check.py`; the label `no-changelog` skips it).
- **Shared checks updated:** the validator of the suite checks the text of the banner against the manifest and reads the files that git tracks.
- **A thin clamp is no longer stepped over:** a straight move is tested against the box of each clamp exactly (segment against box, with the height at the point of entry) instead of every half millimetre, so a clamp thinner than the step is found by a long move; arcs are tested chord by chord.
- **More tests:** the checks are tried at the exact limits of the table, the lowest Z, the feed and the height of a clamp; the time of a move at the point where the ramp and the triangle meet; arcs, helices and the exit codes of every command of the command line. The findings now come in the order of the program (the notices of the reader came first), and a time just under a minute is no longer written as 60.0 s.

## [0.0.3] - Reading from bytes and notices by code
- **The CI builds against the tag of each sibling:** the sibling repositories are read from `depends_on` of the manifest (the ones they need in turn included) and checked out at the tag the repository is built from, so a build does not move when a sibling changes; a separate job, `Siblings at main`, builds against the main branch of every sibling to show a break before the sibling is tagged and does not hold a merge.

- **The program is read from its bytes:** `--max-bytes N` refuses a file over the limit (256 MiB by default), and text that is not UTF-8 is read as Latin-1 with a finding that carries the code `input-not-utf8`.
- **The findings of the reader carry their stable code** (`[unknown-g-code]`), and `--ignore CODE[,CODE]` (or `ignore_codes` in the limits) leaves out the notices of those codes; a misspelt code is an error.
- **Tests that run what the README and the scripts promise:** the sample of `run.sh`, `run.bat` and the README is the same command, and it is run on the files of `examples/`.
- **The banner of the README** shows the role and the technologies of this program (it showed the text of the shared library).

## [0.0.2] - Arcs checked along the arc, a strict mode and a report of the codes that move the origin

- **Each dependency on a sibling repository by path says the version it needs** (`version = ">=0.0.1", path = "../EH3D-..."`), so Cargo stops at once when the sibling next to this one is older than the program expects.
- **Arcs are checked along the arc:** the table limits and the clamps are tested on the points of a G2 or G3, not on its chord, so an arc that leaves the table or passes over a clamp with both ends inside is found.
- **`--strict`:** every warning counts as an alert (exit code 1), for a flow that uses the exit code as the barrier before the machine (a cut with the spindle off, for one).
- A program with G92, G53, G28, G30 or G10 is reported (the reader of G-code says it did not follow them) instead of 'no findings'.

## [0.0.1] - The time of a G-code program and the alerts that stop a bad one

- **The time:** a trapezoid of speed per move with the acceleration and the rapid speed of the machine, arcs measured along the arc, split in cutting and rapid.
- **The alerts:** rapid moves into the material, moves outside the table or below the lowest Z and heads over a clamp; **the warnings:** cuts with the spindle off or with no feed and a feed above a limit.
- **The reader's diagnostics** are alerts with their line, so a misread program is never clean.
- **The command line** `eh3d-camdebug` with `check` (exit code 1 on an alert) and `steps`, and a sample program.
- **13 tests** run on a computer, with damaged input.
