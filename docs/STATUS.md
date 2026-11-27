# EH3D-CAM-DEBUGGER: status

## Current state

EH3D-CAM-DEBUGGER is a working program on a computer: a library (`eh3d_cam_debugger`), a command line (`eh3d-camdebug`) and a window (built with `--features gui`). The window plays the program over the table, and its logic, its viewport and its eight languages are tested without a screen. A twin of the machine in the tests hits what the debugger reports. The suite of tests runs on Windows and Linux in the CI. The window has not been looked at on a screen, and no real program or machine has been used.

## Capabilities

The official state of each capability is in `REQUIREMENTS.md`. The register has 34 requirements: 25 implemented, 4 prototype, 5 planned and 0 validated. By progress: 25 done, 4 partial, 1 pending and 4 blocked. This page never states a state that `REQUIREMENTS.md` does not.

## Open questions

* Whether the removal of material is simulated here or in the window of the suite that shows the part in 3D.
* Whether the clamps should come from the project of the job instead of being described by hand.
* Which controller dialects beyond the ones the reader knows should be told how they wait and how they time a move.

## What it needs from other repositories

* `EH3D-COMMON`: the client of the server, as a sibling folder at the version named in the workflows.
* `EH3D-CORE-LIBS`: the G-code reader.
* A 3D view of the part, in the window of the suite, for the removal of material.

## Known technical debt and omitted tests

* The times are an upper bound that were never compared with the time a real controller takes.
* The drawing of the window is not covered by tests; only its logic is, and no person has looked at it on a screen.
* The texts of the findings come from the library and are in English.
* Tests that need a machine or a screen are not written: the register names each as `planned:` with the test that will show it.
