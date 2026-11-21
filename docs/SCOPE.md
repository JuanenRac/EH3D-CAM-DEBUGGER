# EH3D-CAM-DEBUGGER: scope

Status: **host-test**. EH3D-CAM-DEBUGGER is a library, a command line and a window that read a G-code program, work out how long it takes and say what in it would hurt the machine, the part or the table. It has been tried on a computer only: it has not been compared with a collection of real programs or with a machine that ran them. This page says what it is for, what it will not do and where it sits among the other programs of the suite.

## Purpose

The program of the suite that checks a G-code program before it reaches a machine: it reads it, times every move with the acceleration and the speeds of the machine, and reports alerts and warnings with their line (a rapid move into the material, a move outside the table, a head that passes over a clamp, a cut with the spindle off, a feed above a limit, and what the reader could not understand). The window plays the program move by move over the table.

## What it does

* Reads a program from its bytes (a size limit, text that is not UTF-8 read as Latin-1 and reported) and gives every finding of the reader its stable code, which `--ignore` can leave out.
* Times every move as a trapezoid of speed with the acceleration and the rapid speed of the machine, arcs included, and splits the total in cutting and rapid.
* Reports alerts (exit code 1) and warnings with their line; `--strict` makes every warning an alert.
* Tests the table, the lowest Z and the clamps along the arc of a G2 or G3 and exactly along a straight move, so a clamp thinner than any step is found.
* Checks, when asked, a plunge feed limit, the height at which the program ends and the closing `M2` or `M30`; `check --json` prints the summary and every finding as one object.
* Plays the program in a window (built with `--features gui`) in eight languages, with steps, a rate, the findings marked where the move ends, and keeps the report on the EH3D server when asked.

## What it will not do

* No decision about the limits of a machine, its interlocks or its emergency stop (see below).
* No removal of material and no 3D view: the path is shown from above (see `NOT_YET.md`).
* No guarantee that a program with no findings is safe: the clamps are boxes that a person describes by hand.
* No change to the program it reads: it only reports.
* No claim that anything was proven on a machine: the evidence of every capability is `simulator` or `host-test`.

## Where it sits

* **It reads:** a G-code program, its machine description and its clamps as plain files, its settings file and, when asked, the server of the suite.
* **It writes:** a report (text or JSON) and the steps as text; when asked, the same artifacts on the server. It never writes to the program it reads.
* **It is read by:** the other programs of the suite through those files or through the server; programs exchange work through files or through the server, never by depending on each other.
* **It depends on:** `EH3D-COMMON` (the client of the server) and `EH3D-CORE-LIBS` (the G-code reader), built next to it as sibling folders.

## What it never decides

The limits of a machine, its interlocks and its emergency stop belong to the machine and to its controller, not to this program. A program without findings is a program in which this check found nothing, not a safe one: the time is an upper bound, the clamps are only the boxes that were described, and what the reader could not understand is told as an alert. Nothing here replaces the emergency stop of a machine or the judgement of the person who stands at it.

The requirements of the program, with the state of each and the test that shows it, are in `REQUIREMENTS.md`; the state in a paragraph is in `STATUS.md`.
