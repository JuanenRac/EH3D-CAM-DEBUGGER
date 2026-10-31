# Changelog

All notable changes to this project are documented here.

## [Unreleased]

## [0.0.1] - The time of a G-code program and the alerts that stop a bad one

- **The time:** a trapezoid of speed per move with the acceleration and the rapid speed of the machine, arcs measured along the arc, split in cutting and rapid.
- **The alerts:** rapid moves into the material, moves outside the table or below the lowest Z and heads over a clamp; **the warnings:** cuts with the spindle off or with no feed and a feed above a limit.
- **The reader's diagnostics** are alerts with their line, so a misread program is never clean.
- **The command line** `eh3d-camdebug` with `check` (exit code 1 on an alert) and `steps`, and a sample program.
- **13 tests** run on a computer, with damaged input.
