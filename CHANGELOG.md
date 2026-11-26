# Changelog

All notable changes to this project are documented here.

## [Unreleased]

- Fix the check of the table for arcs (safety): a large arc that leaves the table by less than the distance between the points sampled along it (up to about 0.1 mm on a radius of 100 mm) was not reported; the farthest points of its circle are now checked too.
- **Behaviour tests at the exact edge:** `tests/behaviour.rs` adds 67 deterministic tests without changing the behaviour, on traces built by hand so that every expected number is worked out: each rule of the checks on both sides of its limit (rapid moves into the material, cuts with the spindle off, feeds and plunges, lowest Z, the table with its tolerance, arcs that bulge out of the table, clamps and their exact height, the end of the program), the escaped text and the exact shape of the JSON report, the length of arcs and helices, the times of the steps, the zoom of the drawing, the base64 and the request of the server link, the arguments of the window and the eight languages.
- **A picture of the window in the README:** the window with the example program, written by the picture mode (`--shot`) of the published program, with its caption in the eight languages.
- **Requirements with identifiers and tests that name them:** `docs/REQUIREMENTS.md` is the register of what the program must do, one row for each capability with an identifier (`CDBG-001` on), its state (planned, prototype, implemented or validated), how far it is (done, partial, pending or blocked), its evidence and the test that shows it; a capability that is not written names the test that will show it. `tests/reqs.rs` reads the table and fails when a test does not exist, when a state does not agree with the progress or the evidence, when `docs/STATUS.md` does not say the same numbers, when a README does not link the documents or when a document holds a date or a private reference. The five documents of the family (`docs/SCOPE.md`, `docs/REQUIREMENTS.md`, `docs/STATUS.md`, `docs/NOT_YET.md` and `AGENTS.md`) are in the repository and are linked from the eight READMEs.
- The shared workflow checker (`tools/workflow_hygiene.py`) is the current one of the documentation hub: the filters under a trigger (branches, paths, types) are no longer read as events.

## [0.0.8] - Send reports to the EH3D server and a server status command

- **A server mode over the client of the suite:** `--config FILE` reads the keys `server_address`, `server_token`, `server_machine`, `server_timeout_s` (1 to 120, 15 by default), `server_retries` (0 to 5, 2 by default), `server_mdns` (`off` by default) and `server_max_answer_mib` (1 to 256, 8 by default) with `eh3d-serverclient` of EH3D-COMMON and leaves the other lines of the file alone; a value out of its range or a space in the address is a wrong use (64) that says its line; the mode is off until `server_address` is set and the local work never depends on it.
- **`eh3d-camdebug server status`:** shows the link (the token as `***`) and what the server answers at `/health`, with the refusals and the failures of the network told by their code in the language of the system (taken from `LC_ALL`, `LC_MESSAGES` or `LANG`).
- **`check --to-server` keeps the report on the server:** the JSON report of the check is kept on the EH3D server as the artifact `PROGRAM.report.json`, with the program, the dialect and the number of alerts and warnings as its parameters, whether the text or `--json` is printed; the exit code of the check does not change, the option is a wrong use (64) before any work when `--config` has no `server_address` or `server_token`, and a refusal of the server is told after the report was printed.
- **`steps --to-server` keeps the list of moves:** the lines of `steps` are kept as the artifact `PROGRAM.steps.txt` (plain text) with the number of moves as a parameter.
- **The artifact says what it is and how it was made:** its media type is JSON for the report and plain text for the moves, and its parameters carry the command, the program, the dialect and the counts.
- **The CI takes `EH3D-COMMON` at `v0.2.5`:** the sibling that is checked out is the release that has the client of the server, and the manifest lists `EH3D-COMMON` in `depends_on`.

## [0.0.7] - Checks of plunge feed and end height, numbers of the program and JSON output

- **A limit for the feed of a plunge:** `--max-plunge MM_MIN` warns, with its line, depth and feed, of a plunge (a cutting move that goes down at least as far as it goes along in the plane, so a ramp at less than 45 degrees is not one) whose feed is above the limit; a plunge with no feed is left to the check of the missing feed, and nothing is checked unless the option is given.
- **Two checks of how the program ends:** `--end-z Z` warns, on the last move, when the program ends with the tool below Z (still in the material, or too near to it) and `--require-end` warns when a program that moves does not end with `M2` or `M30`; both are off unless asked for and `--strict` makes them alerts like any other warning.
- **`check` prints a summary of the program:** the extent of everything that moved (X, Y and Z), how many rapid moves, straight cuts and arcs there are, the tools the program selects (once each, in order) and the range of the feeds of its cutting moves.
- **`check --json` prints the result as one JSON object:** the moves by kind, the lengths, the times (cutting, rapid, the waits of `G4` apart), the extent, the tools, the range of the feeds, whether the program ended and every finding with its line, its level and its message (the text is escaped; a number that is not finite is `null`); the exit code is the one of the text, and `steps --json` is a wrong use. The window does not take the new limits yet.
- **The CI and the release build take the shared libraries at their newest releases:** the pins of the sibling repositories in the workflows are raised to the releases the program now needs, so that the build and the binaries of a release use them.

## [0.0.6] - Simulation of the machine against the debugger and the maturity said in the README

- **The README says the maturity of the manifest:** the badge and the Maturity row of the eight READMEs read functional, as the manifest does, and the check of the status lights now fails when they differ.
- **Simulation of the machine against the debugger:** add `tests/machine_twin.rs`, a twin of the machine that reads the program on its own (it does not use the reader of the suite), moves with an acceleration, and notes what it hits: a rapid move into the material, a point off the table, a Z below the lowest and a cut with the spindle off. Programs from a seeded generator (some of them wrong) are run on it and the debugger must report the same lines and the same kinds of mistake, and the time it gives must be the time the twin takes to move; four mistakes and their repair show that the program put right is clean. Three tests cover them. Nothing has run on a real machine.

- **The shared validation tools are updated:** the validator also runs the workflow and lessons checks of the suite with their latest rules, including a check to run before every push.
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
