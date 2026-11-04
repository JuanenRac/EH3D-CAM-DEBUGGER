//! The file is read as bytes: a size limit, text that is not UTF-8, and the notices of the reader chosen by their code.

use eh3d_cam_debugger::checks::{check, Limits};
use eh3d_parser_gcode::{interpret, parse_bytes, Dialect};
use std::fs;
use std::process::Command;

const PROGRAM: &str = "G21 G90\nG123\nG1 X5 F100\nM2\n";

fn latin1() -> Vec<u8> {
    // a comment with an o with an acute accent written as one byte (Latin-1), which is not UTF-8
    let mut b = b"(fabricaci".to_vec();
    b.push(0xF3);
    b.extend_from_slice(b"n)\n");
    b.extend_from_slice(PROGRAM.as_bytes());
    b
}

fn run(args: &[&str]) -> (Option<i32>, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_eh3d-camdebug"))
        .args(args)
        .output()
        .expect("the program runs");
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
fn text_that_is_not_utf8_is_a_finding_with_its_code() {
    let trace = interpret(&parse_bytes(&latin1(), Dialect::Grbl).unwrap());
    let f = check(&trace, &Limits::default());
    assert!(
        f.iter().any(|f| f.message.contains("[input-not-utf8]")),
        "{f:?}"
    );
}

#[test]
fn a_finding_of_the_reader_is_left_out_by_its_code() {
    let trace = interpret(&parse_bytes(PROGRAM.as_bytes(), Dialect::Grbl).unwrap());
    let all = check(&trace, &Limits::default());
    assert!(
        all.iter().any(|f| f.message.contains("[unknown-g-code]")),
        "{all:?}"
    );
    let some = check(
        &trace,
        &Limits {
            ignore_codes: vec!["unknown-g-code".to_string()],
            ..Limits::default()
        },
    );
    assert!(!some.iter().any(|f| f.message.contains("unknown-g-code")));
    assert!(some.len() < all.len());
}

#[test]
fn the_command_line_applies_the_limit_the_encoding_and_the_filter() {
    let dir = std::env::temp_dir().join(format!("eh3d_camdebug_read_{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let file = dir.join("p.nc");
    fs::write(&file, latin1()).unwrap();
    let f = file.to_str().unwrap();
    let (code, out, _) = run(&["check", f]);
    assert_eq!(code, Some(2), "only warnings: {out}");
    assert!(out.contains("[input-not-utf8]"), "{out}");
    assert!(out.contains("[unknown-g-code]"), "{out}");
    let (_, out, _) = run(&["check", f, "--ignore", "input-not-utf8,unknown-g-code"]);
    assert!(out.contains("no findings"), "{out}");
    let (code, _, err) = run(&["check", f, "--max-bytes", "10"]);
    assert_eq!(code, Some(1));
    assert!(err.contains("above the limit"), "{err}");
    let (code, _, err) = run(&["check", f, "--ignore", "no-such-code"]);
    assert_eq!(code, Some(64));
    assert!(err.contains("not a code"), "{err}");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn the_refusal_of_a_program_over_the_limit_is_written_in_the_language_of_the_system() {
    let dir = std::env::temp_dir().join(format!("eh3d_camdebug_lang_{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let file = dir.join("p.nc");
    fs::write(&file, PROGRAM).unwrap();
    let f = file.to_str().unwrap();
    for (lang, word) in [
        ("en", "above the limit"),
        ("es", "supera el límite"),
        ("de", "überschreitet die Grenze"),
        ("fr", "dépasse la limite"),
        ("it", "supera il limite"),
        ("ja", "上限を超えています"),
        ("zh", "文件超过上限"),
        ("pt", "excede o limite"),
    ] {
        let out = Command::new(env!("CARGO_BIN_EXE_eh3d-camdebug"))
            .args(["check", f, "--max-bytes", "10"])
            .env("LC_ALL", lang)
            .output()
            .unwrap();
        let err = String::from_utf8_lossy(&out.stderr).into_owned();
        assert_eq!(out.status.code(), Some(1), "{lang}: {err}");
        assert!(err.contains(word), "{lang}: {err}");
    }
    let _ = fs::remove_dir_all(&dir);
}
