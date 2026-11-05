//! The program is a command line first: without the `gui` feature it never opens a window, and the `gui` word explains why. (With the feature, no arguments open the window, so these
//! tests are for the build the CI makes.)
#![cfg(not(feature = "gui"))]

use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_eh3d-camdebug");

#[test]
fn no_arguments_print_the_help_when_the_build_has_no_window() {
    let out = Command::new(BIN).output().expect("the program runs");
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("checks a G-code program before it reaches a machine"));
    assert!(text.contains("[gui]"));
}

#[test]
fn the_gui_word_says_that_this_build_has_no_window() {
    let out = Command::new(BIN)
        .arg("gui")
        .output()
        .expect("the program runs");
    assert!(!out.status.success());
    let text = String::from_utf8_lossy(&out.stderr);
    assert!(text.contains("--features gui"), "{text}");
}

#[test]
fn a_single_program_without_a_command_is_an_unknown_command_in_this_build() {
    let program = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/sample.nc");
    let out = Command::new(BIN)
        .arg(program)
        .output()
        .expect("the program runs");
    assert!(!out.status.success());
}
