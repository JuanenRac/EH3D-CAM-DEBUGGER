# Changelog

All notable changes to this project are documented here.

## [Unreleased]
- **The CI builds against the tag of each sibling:** the sibling repositories are read from `depends_on` of the manifest (the ones they need in turn included) and checked out at the tag the repository is built from, so a build does not move when a sibling changes; a separate job, `Siblings at main`, builds against the main branch of every sibling to show a break before the sibling is tagged and does not hold a merge.

- **The program is read from its bytes:** `--max-bytes N` refuses a file over the limit (256 MiB by default), and text that is not UTF-8 is read as Latin-1 with a finding that carries the code `input-not-utf8`.
- **The findings of the reader carry their stable code** (`[unknown-g-code]`), and `--ignore CODE[,CODE]` (or `ignore_codes` in the limits) leaves out the notices of those codes; a misspelt code is an error.
- **Tests that run what the README and the scripts promise:** the sample of `run.sh`, `run.bat` and the README is the same command, and it is run on the files of `examples/`.

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
