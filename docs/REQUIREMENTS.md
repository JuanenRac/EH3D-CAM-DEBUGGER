# EH3D-CAM-DEBUGGER: requirements

This register is the single source of the state of each capability: `STATUS.md` and the README files cite it and never state a state that it does not. A state is one of `planned`, `prototype`, `implemented` and `validated`. `validated` means that a person has run the capability on a real machine and the evidence of the manifest says `bench` or `field`; nothing here is validated, because nothing of this program has been compared with a real machine.

The register has 34 requirements: 25 implemented, 4 prototype, 5 planned and 0 validated. By progress: 25 done, 4 partial, 1 pending and 4 blocked. High coverage is not proof of correctness, so each row says how it is checked: the test that names it, and what it shows.

## How to read the register

* **ID:** `CDBG-NNN`, never reused and never renumbered; a new capability takes the next number.
* **State:** `implemented` is code that is complete and tested on a computer, `prototype` is part of the capability, `planned` is not written.
* **Progress:** `done`, `partial`, `pending` (can be written without hardware) or `blocked` (waits for hardware or for another program of the suite).
* **Evidence:** `simulator` when the test runs against a simulated machine or a server in memory, `host-test` when it checks a calculation, a reader or a screen-less piece, `none` when nothing is written yet.
* **Test:** the acceptance criterion: `file::name` is the test `name` of `tests/file.rs` (`src/file::name` is a test inside `src/file.rs`), which must exist. For a capability that is not written the cell says `planned:` and the name of the test that will show it; the day that test exists, the row must change.
* **Limits:** what is not done or not proven; a row that is `partial` or `blocked` always says it.

The check is `tests/reqs.rs`: it reads this table, finds each test and fails when a state, a test or a count does not agree.

## Time

| ID | Requirement | State | Progress | Evidence | Test | Limits |
|---|---|---|---|---|---|---|
| CDBG-001 | A move takes the time of a trapezoid of speed with the acceleration and the rapid speed of the machine. | implemented | done | host-test | `debugger::a_move_takes_the_time_of_a_trapezoid` | An upper bound: a real controller does not stop between moves. |
| CDBG-002 | The total time is split in cutting and rapid, and a cut with no feed uses a default feed. | implemented | done | host-test | `debugger::a_program_has_a_time_split_in_cutting_and_rapid` | - |
| CDBG-003 | The length of an arc is the length of the arc, a helix is longer than its flat arc and a quarter turn the long way is three quarters. | implemented | done | host-test | `debugger::the_length_of_an_arc_is_the_length_of_the_arc` | - |
| CDBG-004 | The steps add up to the total and each starts where the one before ended. | implemented | done | host-test | `debugger::the_steps_add_up_to_the_total_and_start_where_the_one_before_ended` | - |
| CDBG-005 | A machine with a value that is not more than zero is refused, naming the value. | implemented | done | host-test | `more::a_machine_with_a_value_that_is_not_more_than_zero_is_refused_naming_it` | - |
| CDBG-006 | The time is written in the unit that fits and never says sixty seconds. | implemented | done | host-test | `more::the_time_is_written_in_the_unit_that_fits_and_never_says_sixty_seconds` | - |

## Alerts and warnings

| ID | Requirement | State | Progress | Evidence | Test | Limits |
|---|---|---|---|---|---|---|
| CDBG-007 | A rapid move into the material is an alert. | implemented | done | host-test | `debugger::a_rapid_move_into_the_material_is_an_alert` | - |
| CDBG-008 | A move outside the table and a Z below the lowest one of the machine are alerts. | implemented | done | host-test | `debugger::a_move_outside_the_table_and_below_the_lowest_z_are_alerts` | - |
| CDBG-009 | A head over a clamp is an alert only below the height of the clamp, and a thin clamp between two samples of a long move is still found. | implemented | done | host-test | `debugger::a_thin_clamp_between_two_samples_of_a_long_move_is_still_found` | Clamps are boxes a person describes. |
| CDBG-010 | A cut with the spindle off, a cut with no feed and a feed above a limit are warnings. | implemented | done | host-test | `debugger::a_cut_with_the_spindle_off_and_a_cut_with_no_feed_are_warnings` | - |
| CDBG-011 | An arc is checked along the arc and not along its chord. | implemented | done | host-test | `debugger::an_arc_is_checked_along_the_arc_and_not_along_its_chord` | - |
| CDBG-012 | `--strict` makes every warning an alert. | implemented | done | host-test | `debugger::strict_makes_the_warnings_alerts` | - |
| CDBG-013 | The codes that change the origin are never reported as clean. | implemented | done | host-test | `debugger::the_codes_that_change_the_origin_are_never_reported_as_clean` | - |
| CDBG-014 | What the reader could not understand is reported with its line, so a program that was misread is never clean. | implemented | done | host-test | `debugger::what_the_reader_could_not_understand_is_reported` | - |
| CDBG-015 | The same program checked twice gives the same findings, in the order of the program. | implemented | done | host-test | `debugger::the_same_program_checked_twice_gives_the_same_findings` | - |
| CDBG-016 | A plunge faster than a limit, a program that ends below a height and a program that does not close with `M2` or `M30` are warnings, only when asked. | implemented | done | host-test | `program_checks::the_new_options_check_a_plunge_the_end_height_and_the_closing_of_the_program` | - |
| CDBG-017 | Garbage and damaged input never panic. | implemented | done | host-test | `debugger::garbage_does_not_panic` | - |

## Reading, output and server

| ID | Requirement | State | Progress | Evidence | Test | Limits |
|---|---|---|---|---|---|---|
| CDBG-018 | Text that is not UTF-8 is a finding with its code, and a file over the limit is refused. | implemented | done | host-test | `reading::text_that_is_not_utf8_is_a_finding_with_its_code` | - |
| CDBG-019 | `--ignore` leaves out only the notices of the codes it names. | implemented | done | host-test | `debugger::an_ignored_code_hides_only_the_notices_of_that_code` | - |
| CDBG-020 | `check --json` prints one object with the numbers, the extent and every finding, with the same exit code. | implemented | done | host-test | `program_checks::check_json_is_one_object_with_the_same_exit_code_and_steps_does_not_take_it` | - |
| CDBG-021 | The report is kept on the server as JSON or text, and nothing is sent without an address. | prototype | partial | simulator | `reading::the_report_of_a_check_is_kept_on_the_server_as_json_whether_the_text_or_the_json_is_printed` | Tried against a server in memory, not against a real server. |
| CDBG-022 | The token of the server is never shown. | implemented | done | host-test | `reading::the_server_command_shows_the_link_without_the_token_and_what_the_server_answers` | - |
| CDBG-023 | A twin of the machine hits what the debugger reports, and a program put right is clean. | prototype | partial | simulator | `machine_twin::what_the_twin_hits_is_what_the_debugger_reports` | The twin is a model made for the tests, not a real machine. |

## Window

| ID | Requirement | State | Progress | Evidence | Test | Limits |
|---|---|---|---|---|---|---|
| CDBG-024 | The window finds what the command line finds when it opens a program. | prototype | partial | host-test | `src/window::the_window_finds_what_the_command_line_finds` | The logic is tested without a screen; no person has looked at the drawing. |
| CDBG-025 | Stepping goes through the moves and stops at both ends, and playing stops at the end. | prototype | partial | host-test | `src/window::stepping_goes_through_the_moves_and_stops_at_both_ends` | The logic is tested without a screen; no person has looked at the drawing. |
| CDBG-026 | Every sentence of the window has a text in each of the eight languages. | implemented | done | host-test | `src/lang::every_sentence_has_a_text_in_each_of_the_eight_languages` | - |
| CDBG-027 | A setting that changes after the analysis makes the result stale. | implemented | done | host-test | `src/window::a_setting_that_changes_after_the_analysis_makes_it_stale` | - |

## Not written yet

| ID | Requirement | State | Progress | Evidence | Test | Limits |
|---|---|---|---|---|---|---|
| CDBG-028 | The removal of material is simulated in 3D and the result is compared with the part. | planned | blocked | none | `planned:removal::the_stock_left_matches_the_part` | Needs the 3D view of the window of the suite. |
| CDBG-029 | The findings are told in the language of the window. | planned | pending | none | `planned:findings_text::a_finding_is_told_in_each_of_the_eight_languages` | The texts of the findings are those of the library, in English. |
| CDBG-030 | The clamps are taken from the project of the job. | planned | blocked | none | `planned:project_clamps::the_clamps_of_the_project_are_the_clamps_checked` | Needs the project format of the suite to say where the clamps are. |
| CDBG-031 | A program is run on a real machine and the time and the findings are compared with what happened. | planned | blocked | none | `planned:real_machine::the_time_and_the_findings_match_a_real_run` | Needs a machine. |
| CDBG-032 | The window is looked at on a screen and matches the picture the program takes of itself. | planned | blocked | none | `planned:window_screen::the_window_matches_the_reference_picture` | Needs a person at a screen. |

## Dialects

| ID | Requirement | State | Progress | Evidence | Test | Limits |
|---|---|---|---|---|---|---|
| CDBG-033 | A program for a Marlin printer read as GRBL names the dialect to choose, without changing the levels or the exit code. | implemented | done | host-test | `reading::a_program_for_a_printer_read_as_grbl_says_which_dialect_to_choose` | Recognised by an `E` word or the codes M104, M109, M140 and M190; it applies when the program is read as GRBL. |
| CDBG-034 | With no dialect chosen, a program that is surely for a 3D printer is read as Marlin and that is said in the output and in the window; a chosen dialect always rules. | implemented | done | host-test | `reading::only_a_program_that_is_surely_for_a_printer_is_read_as_marlin_by_itself` | A temperature code with its value, a move with an `E` word and no M3, M4 or M6; not tried with real printer files. |
