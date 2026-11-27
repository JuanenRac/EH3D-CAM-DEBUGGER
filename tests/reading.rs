//! The file is read as bytes: a size limit, text that is not UTF-8, and the notices of the reader chosen by their code.

use eh3d_cam_debugger::checks::{add_dialect_hint, check, Level, Limits};
use eh3d_parser_gcode::{interpret, parse, parse_bytes, Dialect};
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
// ---- the server mode ----

/// A server of the computer that answers `n` requests with `status` and `body` and gives back the requests it got.
fn local_server(
    n: usize,
    status: &str,
    body: &str,
) -> (String, std::thread::JoinHandle<Vec<String>>) {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap().to_string();
    let reply = format!(
        "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
        body.len()
    );
    let handle = std::thread::spawn(move || {
        let mut heads = Vec::new();
        for _ in 0..n {
            let Ok((mut s, _)) = listener.accept() else {
                break;
            };
            let mut buf = vec![0u8; 262_144];
            let mut got = 0;
            loop {
                let k = s.read(&mut buf[got..]).unwrap_or(0);
                if k == 0 {
                    break;
                }
                got += k;
                let text = String::from_utf8_lossy(&buf[..got]).to_string();
                if let Some(split) = text.find("\r\n\r\n") {
                    let want = text[..split]
                        .lines()
                        .find_map(|l| {
                            l.to_ascii_lowercase()
                                .strip_prefix("content-length:")
                                .and_then(|v| v.trim().parse::<usize>().ok())
                        })
                        .unwrap_or(0);
                    if got >= split + 4 + want {
                        break;
                    }
                }
            }
            heads.push(String::from_utf8_lossy(&buf[..got]).to_string());
            let _ = s.write_all(reply.as_bytes());
        }
        heads
    });
    (address, handle)
}

const CREATED: &str = "{\"artifact_id\":\"a-7\",\"blob_sha256\":\"ab\",\"size\":9}";

fn link_file(dir: &std::path::Path, address: &str) -> String {
    let cfg = dir.join("link.cfg");
    fs::write(
        &cfg,
        format!("server_address = {address}\nserver_token = program-token\nserver_retries = 0\nserver_timeout_s = 5\n"),
    )
    .unwrap();
    cfg.to_str().unwrap().to_string()
}

const CLEAN: &str = "G21 G90\nM3 S12000\nG0 X0 Y0 Z5\nG1 Z-0.1 F60\nG1 X10 F200\nG0 Z5\nM5\nM2\n";

fn program(tag: &str) -> (std::path::PathBuf, String) {
    let dir = std::env::temp_dir().join(format!("eh3d_camdebug_{tag}_{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let file = dir.join("p.nc");
    fs::write(&file, CLEAN).unwrap();
    let f = file.to_str().unwrap().to_string();
    (dir, f)
}

#[test]
fn the_report_of_a_check_is_kept_on_the_server_as_json_whether_the_text_or_the_json_is_printed() {
    let (dir, f) = program("to_server");
    let (address, handle) = local_server(2, "201 Created", CREATED);
    let cfg = link_file(&dir, &address);
    let (code, out, err) = run(&["check", &f, "--config", &cfg, "--to-server"]);
    assert!(matches!(code, Some(0) | Some(2)), "{code:?} {err}");
    assert!(out.contains("time (upper bound)"), "{out}");
    assert!(
        out.contains("kept on the server: p.nc.report.json as a-7"),
        "{out}"
    );
    let (code, out, err) = run(&["check", &f, "--json", "--config", &cfg, "--to-server"]);
    assert!(matches!(code, Some(0) | Some(2)), "{code:?} {err}");
    assert!(
        out.contains("kept on the server: p.nc.report.json as a-7"),
        "{out}"
    );
    let heads = handle.join().unwrap();
    for head in &heads {
        assert!(
            head.starts_with("POST /api/v1/artifacts HTTP/1.1"),
            "{head}"
        );
        assert!(head.contains("Bearer program-token"));
        assert!(head.contains("\"name\":\"p.nc.report.json\""), "{head}");
        assert!(head.contains("\"format\":\"application/json\""), "{head}");
        assert!(head.contains("\"command\":\"check\""), "{head}");
        assert!(head.contains("\"program\":\"p.nc\""), "{head}");
        assert!(head.contains("eh3d-cam-debugger"), "{head}");
    }
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn the_steps_are_kept_on_the_server_as_text() {
    let (dir, f) = program("steps_server");
    let (address, handle) = local_server(1, "201 Created", CREATED);
    let cfg = link_file(&dir, &address);
    let (code, out, err) = run(&["steps", &f, "--config", &cfg, "--to-server"]);
    assert_eq!(code, Some(0), "{err}");
    assert!(out.contains("rapid") && out.contains("cut"), "{out}");
    assert!(
        out.contains("kept on the server: p.nc.steps.txt as a-7"),
        "{out}"
    );
    let heads = handle.join().unwrap();
    assert!(
        heads[0].contains("\"format\":\"text/plain\""),
        "{}",
        heads[0]
    );
    assert!(heads[0].contains("\"command\":\"steps\""), "{}", heads[0]);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn to_server_without_a_server_is_a_wrong_use_and_the_local_mode_is_untouched() {
    let (dir, f) = program("no_server");
    let (code, _, err) = run(&["check", &f, "--to-server"]);
    assert_eq!(code, Some(64));
    assert!(err.contains("--config"), "{err}");
    let cfg = dir.join("none.cfg");
    fs::write(&cfg, "# no server here\n").unwrap();
    let (code, _, err) = run(&[
        "check",
        &f,
        "--config",
        cfg.to_str().unwrap(),
        "--to-server",
    ]);
    assert_eq!(code, Some(64));
    assert!(err.contains("server mode is off"), "{err}");
    let (code, out, _) = run(&["check", &f]);
    assert!(matches!(code, Some(0) | Some(2)));
    assert!(!out.contains("kept on the server"));
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_refusal_of_the_server_is_told_after_the_report_was_printed() {
    let (dir, f) = program("refused");
    let (address, handle) = local_server(
        1,
        "422 Unprocessable Entity",
        "{\"error\":\"bad_artifact\",\"detail\":\"name too long\"}",
    );
    let cfg = link_file(&dir, &address);
    let (code, out, err) = run(&["check", &f, "--config", &cfg, "--to-server"]);
    handle.join().unwrap();
    assert_eq!(code, Some(1));
    assert!(out.contains("time (upper bound)"), "{out}");
    assert!(
        err.contains("not kept on the server") && err.contains("name too long"),
        "{err}"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn the_server_command_shows_the_link_without_the_token_and_what_the_server_answers() {
    let (dir, _) = program("status");
    let (address, handle) = local_server(
        1,
        "200 OK",
        "{\"name\":\"eh3d-server\",\"version\":\"0.1.8\",\"api_revision\":1,\"status\":\"ok\"}",
    );
    let cfg = link_file(&dir, &address);
    let (code, out, err) = run(&["server", "status", "--config", &cfg]);
    handle.join().unwrap();
    assert_eq!(code, Some(0), "{err}");
    assert!(
        out.contains("server_token = ***") && !out.contains("program-token"),
        "{out}"
    );
    assert!(out.contains("eh3d-server 0.1.8 (ok, api 1)"), "{out}");
    let (code, out, _) = run(&["server"]);
    assert_eq!(code, Some(0));
    assert!(out.contains("server mode is off"), "{out}");
    let (code, _, err) = run(&["server", "restart"]);
    assert_eq!(code, Some(64));
    assert!(err.contains("unknown"), "{err}");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn the_keys_of_the_link_are_read_from_the_config_file_with_their_ranges() {
    let (dir, f) = program("keys");
    let cfg = dir.join("link.cfg");
    let c = cfg.to_str().unwrap();
    fs::write(&cfg, "server_address = 10.0.0.5:18200\nserver_token = abc\nserver_machine = bench\nserver_timeout_s = 120\nserver_retries = 5\nserver_mdns = off\nserver_max_answer_mib = 8\nsomething_else = 1\n").unwrap();
    let (code, _, err) = run(&["check", &f, "--config", c]);
    assert!(matches!(code, Some(0) | Some(2)), "{code:?} {err}");
    for (text, word) in [
        ("server_timeout_s = 0\n", "server_timeout_s"),
        ("server_retries = 6\n", "server_retries"),
        ("server_mdns = maybe\n", "server_mdns"),
        ("server_address = a b\n", "server_address"),
    ] {
        fs::write(&cfg, text).unwrap();
        let (code, _, err) = run(&["check", &f, "--config", c]);
        assert_eq!(code, Some(64), "{text}");
        assert!(err.contains(word) && err.contains("line"), "{text}: {err}");
    }
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn what_a_controller_would_read_otherwise_is_told_with_its_code() {
    let text = "G21 G90
M3 S1000 (go!)
G93 G1 X1 F1
G94 G1 X1 X2 F100
M5
M2
";
    let f = check(&interpret(&parse(text, Dialect::Grbl)), &Limits::default());
    for (line, code, level) in [
        (2, "[realtime-character]", Level::Warning),
        (3, "[feed-mode-unsupported]", Level::Alert),
        (4, "[word-repeated]", Level::Alert),
    ] {
        assert!(
            f.iter()
                .any(|f| f.line == line && f.level == level && f.message.contains(code)),
            "{code}: {f:?}"
        );
    }
}

#[test]
fn a_program_for_a_printer_read_as_grbl_says_which_dialect_to_choose() {
    let findings = |text: &str, dialect: Dialect| {
        let program = parse(text, dialect);
        let mut f = check(&interpret(&program), &Limits::default());
        add_dialect_hint(&mut f, &program);
        f
    };
    let told = |text: &str, dialect: Dialect| {
        findings(text, dialect)
            .iter()
            .filter(|f| f.message.contains("--dialect marlin"))
            .count()
    };
    // told once, on the first finding of the reader that is about a word or a code of a printer
    let print = "M104 S200\nG21 G90\nG1 X1 E0.5 F600\nG1 X2 E1\nM2\n";
    assert_eq!(told(print, Dialect::Grbl), 1);
    assert_eq!(told(print, Dialect::Marlin), 0);
    let alerts = findings(print, Dialect::Grbl);
    assert!(alerts.iter().any(|f| f.level == Level::Alert), "{alerts:?}");
    // a mistake that says nothing about a printer gets no such advice
    assert_eq!(told(PROGRAM, Dialect::Grbl), 0);
    assert_eq!(told("G21 G90\nM77\nM2\n", Dialect::Grbl), 0);
}
