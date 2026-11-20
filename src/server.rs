//! The server mode: the result of this program can be kept on the EH3D server as an artifact, so the other programs of the suite (and the people with a login)
//! find it there. The link itself is the one client of the suite (`eh3d-serverclient` of EH3D-COMMON): the same seven keys with the same ranges, retries with
//! growing pauses (a `POST` is never sent twice), the state of the link and the refusals of the server by their code in the language of the person.
//!
//! The mode is off until `server_address` is set in the settings file; nothing is sent without it, and the local work never depends on it. The keys are
//! `server_address`, `server_token`, `server_machine`, `server_timeout_s` (1 to 120, 15), `server_retries` (0 to 5, 2), `server_mdns` (`off`) and
//! `server_max_answer_mib` (1 to 256, 8). The token is write-only: it is never printed.

use eh3d_serverclient as sc;
use sc::json::{quote, Json};
use sc::{Client, ClientError, Lang, Settings};

/// The name this program gives the server (the application of its logins and the producer of its artifacts).
pub const APP: &str = "cam-debugger";
/// The most bytes of one artifact that this program sends in a single request (larger files go by an upload of the server, which this program does not use).
pub const MAX_SEND_BYTES: usize = 8 * 1024 * 1024;

/// Whether a key of a settings file belongs to the link: a program that reads its own keys leaves these to [`settings_from_text`].
pub fn is_server_key(key: &str) -> bool {
    sc::settings::is_server_key(&key.trim().to_ascii_lowercase())
}

/// Checks one `key = value` line of the link against its range, for a program that reads its own settings file and meets a key of the link in it: the
/// error names the key and says what the value must be.
pub fn check_key(key: &str, value: &str) -> Result<(), String> {
    Settings::default()
        .set(key, value)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// The settings of the link in the text of a settings file; the keys of the program itself are left alone, and a value out of its range is an error that
/// names the key and the line.
pub fn settings_from_text(text: &str) -> Result<Settings, String> {
    Settings::from_text(text)
}

/// The settings of the link in a settings file.
pub fn settings_from_file(path: &str) -> Result<Settings, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
    settings_from_text(&text).map_err(|e| format!("{path}: {e}"))
}

/// The language of the person for the refusals of the server, from the environment (`LC_ALL`, `LC_MESSAGES`, `LANG`); English when it is none of the eight.
pub fn language() -> Lang {
    ["LC_ALL", "LC_MESSAGES", "LANG"]
        .iter()
        .filter_map(|k| std::env::var(k).ok())
        .find_map(|v| Lang::from_tag(&v))
        .unwrap_or(Lang::En)
}

/// What a refusal or a failure says to the person: the text of its code, followed by the detail the server gave when the text does not hold it.
pub fn explain(e: &ClientError, lang: Lang) -> String {
    let text = e.text(lang);
    match e {
        ClientError::Server { detail, .. } if !detail.is_empty() && !text.contains(detail) => {
            format!("{text}: {detail}")
        }
        _ => text,
    }
}

/// Checks that there is a server to talk to and a token to send, and says which is missing.
pub fn require(settings: &Settings) -> Result<(), String> {
    if !settings.is_configured() {
        return Err("server mode is off: set server_address in the settings file".into());
    }
    if settings.server_token.is_empty() {
        return Err("server_token is empty: the server needs the token of a program".into());
    }
    Ok(())
}

/// A client over plain HTTP for these settings. `server_mdns` is read and kept, but this program does not look for the server on the network: the address
/// is the one in the file.
pub fn connect(settings: Settings) -> Client {
    Client::connect(settings)
}

/// What the server answers at `/health`, in one line, and the state of the link.
pub fn status_line(client: &mut Client, lang: Lang) -> Result<String, String> {
    let h = client.health().map_err(|e| explain(&e, lang))?;
    Ok(format!(
        "{} {} ({}, api {})",
        h.name, h.version, h.status, h.api_revision
    ))
}

/// Keeps a file on the server in one call: the settings of the link, the name, the media type, the bytes and the values it was made with. Gives the line to print.
pub fn keep(
    settings: Settings,
    name: &str,
    format: &str,
    data: &[u8],
    params: &[(&str, &str)],
) -> Result<String, String> {
    require(&settings)?;
    let mut client = connect(settings);
    let sent = send_artifact(&mut client, name, format, data, params, language())?;
    Ok(sent_line(&artifact_name(name)?, &sent))
}

/// What the `server status` command prints: the settings of the link (the token as `***`) and, when there is a server, what it answers at `/health`.
pub fn status_report(settings: Settings) -> Result<String, String> {
    let mut text = settings.to_text(false);
    if !settings.is_configured() {
        text.push_str("server mode is off\n");
        return Ok(text);
    }
    let mut client = connect(settings);
    let line = status_line(&mut client, language())?;
    text.push_str(&line);
    text.push('\n');
    Ok(text)
}

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Standard base64 with padding, which is how the server takes the bytes of an artifact.
pub fn base64(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let n = (u32::from(chunk[0]) << 16)
            | (u32::from(*chunk.get(1).unwrap_or(&0)) << 8)
            | u32::from(*chunk.get(2).unwrap_or(&0));
        out.push(B64[(n >> 18) as usize & 63] as char);
        out.push(B64[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            B64[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            B64[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

/// The name an artifact may have: 1 to 120 characters, no control character and no slash. A path is cut down to its last part.
pub fn artifact_name(path_or_name: &str) -> Result<String, String> {
    let last = path_or_name.rsplit(['/', '\\']).next().unwrap_or("").trim();
    if last.is_empty() || last.chars().count() > 120 || last.chars().any(char::is_control) {
        return Err(format!(
            "'{path_or_name}' cannot be the name of an artifact (1 to 120 characters, no control character)"
        ));
    }
    Ok(last.to_string())
}

/// The media type (in lower case, as the server wants it) of a file by its extension: G-code, an HP-GL plot, a DXF drawing, a drill file, a Gerber layer, a picture
/// or a table; anything else is plain bytes.
pub fn media_type(file: &str) -> &'static str {
    let ext = file
        .rsplit(['/', '\\'])
        .next()
        .and_then(|name| name.rsplit_once('.'))
        .map(|(_, e)| e.to_ascii_lowercase())
        .unwrap_or_default();
    match ext.as_str() {
        "nc" | "ngc" | "gcode" | "gco" | "tap" | "cnc" => "text/x-gcode",
        "plt" | "hpgl" | "hpg" => "application/vnd.hp-hpgl",
        "dxf" => "image/vnd.dxf",
        "ncd" | "drl" | "xln" | "exc" | "drd" => "application/x-excellon",
        "gbr" | "gtl" | "gbl" | "gts" | "gbs" | "gto" | "gbo" | "gtp" | "gbp" | "gko" | "gm1" => {
            "application/x-gerber"
        }
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "json" => "application/json",
        "csv" => "text/csv",
        "txt" | "cfg" | "log" | "md" => "text/plain",
        _ => "application/octet-stream",
    }
}

/// The body of `POST /api/v1/artifacts`: the name, the media type, the bytes, who made it and the values it was made with.
pub fn artifact_request(name: &str, format: &str, data: &[u8], params: &[(&str, &str)]) -> String {
    let producer = format!(
        "{{\"name\":{},\"version\":{}}}",
        quote(&format!("eh3d-{APP}")),
        quote(option_env!("CARGO_PKG_VERSION").unwrap_or("0"))
    );
    let fields: Vec<String> = params
        .iter()
        .map(|(k, v)| format!("{}:{}", quote(k), quote(v)))
        .collect();
    format!(
        "{{\"name\":{},\"format\":{},\"data_base64\":{},\"producer\":{producer},\"params\":{{{}}}}}",
        quote(name),
        quote(format),
        quote(&base64(data)),
        fields.join(",")
    )
}

/// What the server says of a file it kept.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sent {
    pub artifact_id: String,
    pub blob_sha256: String,
    pub size: u64,
}

/// Keeps `data` on the server as an artifact. Nothing is sent when there is no server or the file is larger than [`MAX_SEND_BYTES`]; the request is a `POST`, so
/// it is sent once and never repeated by the client.
pub fn send_artifact(
    client: &mut Client,
    name: &str,
    format: &str,
    data: &[u8],
    params: &[(&str, &str)],
    lang: Lang,
) -> Result<Sent, String> {
    require(client.settings())?;
    if data.len() > MAX_SEND_BYTES {
        return Err(format!(
            "{name}: {} bytes is more than the {MAX_SEND_BYTES} that go in one request",
            data.len()
        ));
    }
    let body = artifact_request(&artifact_name(name)?, format, data, params);
    let reply = client
        .post("/api/v1/artifacts", Some(&body))
        .map_err(|e| explain(&e, lang))?;
    let json = reply
        .json()
        .ok_or_else(|| "the answer of the server is not JSON".to_string())?;
    let text = |k: &str| {
        json.get(k)
            .and_then(Json::as_str)
            .map(str::to_string)
            .ok_or_else(|| format!("the answer of the server has no {k}"))
    };
    Ok(Sent {
        artifact_id: text("artifact_id")?,
        blob_sha256: text("blob_sha256")?,
        size: json.get("size").and_then(Json::as_u64).unwrap_or(0),
    })
}

/// The line the command prints after a file was kept.
pub fn sent_line(name: &str, sent: &Sent) -> String {
    format!(
        "kept on the server: {name} as {} ({} bytes, sha256 {})",
        sent.artifact_id, sent.size, sent.blob_sha256
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use sc::sim::SimServer;
    use sc::{Reply, Request, Transport, TransportError};
    use std::sync::{Arc, Mutex};

    /// A server that keeps what it is sent and answers like the artifact route; the logins go to the server in memory of the suite.
    type Seen = Vec<(String, String, Option<String>)>;

    #[derive(Clone, Default)]
    struct Fake {
        seen: Arc<Mutex<Seen>>,
        status: Arc<Mutex<u16>>,
    }

    impl Transport for Fake {
        fn send(&self, r: &Request) -> Result<Reply, TransportError> {
            self.seen
                .lock()
                .unwrap()
                .push((r.method.clone(), r.path.clone(), r.body.clone()));
            let status = *self.status.lock().unwrap();
            if status != 0 && status != 201 {
                return Ok(Reply {
                    status,
                    body: "{\"error\":\"bad_artifact\",\"detail\":\"name too long\"}".into(),
                });
            }
            Ok(Reply {
                status: 201,
                body: "{\"artifact_id\":\"a-1\",\"blob_sha256\":\"ab12\",\"size\":5}".into(),
            })
        }
    }

    fn settings(token: &str) -> Settings {
        let mut s = Settings::default();
        s.set("server_address", "127.0.0.1:18200").unwrap();
        s.set("server_token", token).unwrap();
        s
    }

    #[test]
    fn base64_follows_the_standard_with_padding() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
        assert_eq!(base64(&[0xff, 0xfe, 0xfd]), "//79");
    }

    #[test]
    fn the_keys_of_the_link_are_read_with_their_ranges_and_the_rest_is_left_alone() {
        let s = settings_from_text(
            "my_key = 3\nserver_address = 10.0.0.5:18200\nserver_timeout_s = 30\nserver_retries = 4\nserver_mdns = off\nserver_machine = bench\n",
        )
        .unwrap();
        assert_eq!(s.server_address, "10.0.0.5:18200");
        assert_eq!((s.server_timeout_s, s.server_retries), (30, 4));
        assert!(!s.server_mdns);
        assert!(is_server_key(" SERVER_TOKEN ") && !is_server_key("my_key"));
        let e = settings_from_text("server_timeout_s = 0\n").unwrap_err();
        assert!(
            e.contains("server_timeout_s") && e.contains("line 1"),
            "{e}"
        );
        let e = settings_from_text("server_retries = 6\n").unwrap_err();
        assert!(e.contains("server_retries"), "{e}");
        // the defaults are the conservative ones
        let d = settings_from_text("").unwrap();
        assert_eq!(
            (d.server_timeout_s, d.server_retries, d.server_mdns),
            (15, 2, false)
        );
        assert!(!d.is_configured());
    }

    #[test]
    fn nothing_is_sent_without_a_server_or_a_token() {
        assert!(require(&Settings::default()).unwrap_err().contains("off"));
        let mut s = Settings::default();
        s.set("server_address", "127.0.0.1:1").unwrap();
        assert!(require(&s).unwrap_err().contains("server_token"));
        let fake = Fake::default();
        let mut c = Client::new(s, Box::new(fake.clone()));
        let e = send_artifact(&mut c, "a.nc", "text/x-gcode", b"G0", &[], Lang::En).unwrap_err();
        assert!(e.contains("server_token"), "{e}");
        assert!(fake.seen.lock().unwrap().is_empty());
    }

    #[test]
    fn an_artifact_is_one_post_with_the_bytes_the_producer_and_the_values() {
        let fake = Fake::default();
        let mut c = Client::new(settings("program-token"), Box::new(fake.clone()));
        let sent = send_artifact(
            &mut c,
            "C:\\work\\board.nc",
            "text/x-gcode",
            b"G0 X1\n",
            &[("feed", "600"), ("note", "a \"quoted\" word")],
            Lang::En,
        )
        .unwrap();
        assert_eq!(sent.artifact_id, "a-1");
        assert_eq!(sent.size, 5);
        assert!(sent_line("board.nc", &sent).contains("a-1"));
        let seen = fake.seen.lock().unwrap();
        assert_eq!(seen.len(), 1);
        let (method, path, body) = &seen[0];
        assert_eq!(
            (method.as_str(), path.as_str()),
            ("POST", "/api/v1/artifacts")
        );
        let j = Json::parse(body.as_deref().unwrap()).unwrap();
        // the path is cut down to the name of the file, and the bytes go in base64
        assert_eq!(j.get("name").and_then(Json::as_str), Some("board.nc"));
        assert_eq!(j.get("format").and_then(Json::as_str), Some("text/x-gcode"));
        assert_eq!(
            j.get("data_base64").and_then(Json::as_str),
            Some("RzAgWDEK")
        );
        let producer = j.get("producer").unwrap();
        assert_eq!(
            producer.get("name").and_then(Json::as_str),
            Some(format!("eh3d-{APP}").as_str())
        );
        let params = j.get("params").unwrap();
        assert_eq!(params.get("feed").and_then(Json::as_str), Some("600"));
        assert_eq!(
            params.get("note").and_then(Json::as_str),
            Some("a \"quoted\" word")
        );
    }

    #[test]
    fn a_refusal_comes_back_in_the_language_of_the_person_with_its_detail() {
        let fake = Fake::default();
        *fake.status.lock().unwrap() = 422;
        let mut c = Client::new(settings("program-token"), Box::new(fake.clone()));
        let e = send_artifact(&mut c, "a.nc", "text/x-gcode", b"G0", &[], Lang::Es).unwrap_err();
        assert!(e.contains("name too long"), "{e}");
        let en = send_artifact(&mut c, "a.nc", "text/x-gcode", b"G0", &[], Lang::En).unwrap_err();
        assert_ne!(e, en);
        // a POST is not sent twice
        assert_eq!(fake.seen.lock().unwrap().len(), 2);
    }

    #[test]
    fn names_and_sizes_that_the_server_would_refuse_never_leave() {
        assert_eq!(artifact_name("dir/sub\\file.nc").unwrap(), "file.nc");
        assert!(artifact_name("").is_err());
        assert!(artifact_name("dir/").is_err());
        assert!(artifact_name("a\u{7}b").is_err());
        assert!(artifact_name(&"x".repeat(121)).is_err());
        assert!(artifact_name(&"x".repeat(120)).is_ok());
        let fake = Fake::default();
        let mut c = Client::new(settings("t"), Box::new(fake.clone()));
        let big = vec![0u8; MAX_SEND_BYTES + 1];
        let e = send_artifact(&mut c, "big.nc", "text/x-gcode", &big, &[], Lang::En).unwrap_err();
        assert!(e.contains("one request"), "{e}");
        assert!(fake.seen.lock().unwrap().is_empty());
    }

    #[test]
    fn the_login_of_the_suite_server_in_memory_works_with_this_mode() {
        let sim = SimServer::new();
        let mut c = Client::new(settings(""), Box::new(sim.clone()))
            .with_sleeper(Box::new(sim.sleeper()))
            .with_clock(Box::new(sim.clock()));
        let session = c.login("admin", "secret-password", APP).unwrap();
        assert_eq!(session.app, APP);
        assert!(status_line(&mut c, Lang::En)
            .unwrap()
            .contains("eh3d-server"));
        // a program token of the server in memory is taken from the settings
        let mut c = Client::new(settings("program-token"), Box::new(sim.clone()))
            .with_sleeper(Box::new(sim.sleeper()));
        assert!(c.health().is_ok());
        assert!(c.get("/api/v1/echo").is_ok());
    }

    #[test]
    fn a_server_that_is_not_there_is_told_in_words_after_the_retries() {
        let sim = SimServer::new();
        sim.script(vec![sc::sim::Fault::NoConnection; 3]);
        let mut c = Client::new(settings("program-token"), Box::new(sim.clone()))
            .with_sleeper(Box::new(sim.sleeper()));
        let e = send_artifact(&mut c, "a.nc", "text/x-gcode", b"G0", &[], Lang::En).unwrap_err();
        assert!(!e.is_empty());
        assert_eq!(sim.pauses().len(), 2);
    }

    /// A server of the computer that answers `n` requests with `status` and `body`; it gives back the requests it got.
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
                let mut buf = vec![0u8; 65536];
                let mut got = 0;
                // read until the whole request (head and body) is there
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

    #[test]
    fn a_file_is_kept_over_the_network_with_the_token_of_the_program() {
        let (address, handle) = local_server(
            1,
            "201 Created",
            "{\"artifact_id\":\"a-9\",\"blob_sha256\":\"ff\",\"size\":3}",
        );
        let text = format!(
            "server_address = {address}\nserver_token = program-token\nserver_retries = 0\n"
        );
        let line = keep(
            settings_from_text(&text).unwrap(),
            "out/x.nc",
            "text/x-gcode",
            b"G0\n",
            &[],
        )
        .unwrap();
        assert!(line.contains("x.nc") && line.contains("a-9"), "{line}");
        let heads = handle.join().unwrap();
        assert!(
            heads[0].starts_with("POST /api/v1/artifacts HTTP/1.1"),
            "{}",
            heads[0]
        );
        assert!(heads[0].contains("Bearer program-token"));
        assert!(
            heads[0].contains("\"data_base64\":\"RzAK\""),
            "{}",
            heads[0]
        );
    }

    #[test]
    fn the_status_report_hides_the_token_and_tells_when_the_mode_is_off() {
        let off = status_report(Settings::default()).unwrap();
        assert!(off.contains("server mode is off") && off.contains("server_timeout_s = 15"));
        let (address, handle) = local_server(
            1,
            "200 OK",
            "{\"name\":\"eh3d-server\",\"version\":\"0.1.8\",\"api_revision\":1,\"status\":\"ok\"}",
        );
        let text =
            format!("server_address = {address}\nserver_token = very-secret\nserver_retries = 0\n");
        let report = status_report(settings_from_text(&text).unwrap()).unwrap();
        handle.join().unwrap();
        assert!(
            report.contains("server_token = ***") && !report.contains("very-secret"),
            "{report}"
        );
        assert!(report.contains("eh3d-server 0.1.8 (ok, api 1)"), "{report}");
    }

    #[test]
    fn one_line_of_the_link_is_checked_on_its_own() {
        assert!(check_key("server_address", "127.0.0.1:18200").is_ok());
        assert!(check_key("server_retries", "5").is_ok());
        assert!(check_key(" Server_Timeout_S ", "120").is_ok());
        for (key, value) in [
            ("server_retries", "6"),
            ("server_timeout_s", "121"),
            ("server_mdns", "maybe"),
            ("server_address", "a b"),
            ("server_max_answer_mib", "0"),
        ] {
            let e = check_key(key, value).unwrap_err();
            assert!(e.contains(key), "{e}");
        }
    }

    #[test]
    fn the_media_type_follows_the_extension_in_any_case_and_any_folder() {
        assert_eq!(media_type("out/board.nc"), "text/x-gcode");
        assert_eq!(media_type("C:\\work\\BOARD.NGC"), "text/x-gcode");
        assert_eq!(media_type("a.PLT"), "application/vnd.hp-hpgl");
        assert_eq!(media_type("a.dxf"), "image/vnd.dxf");
        assert_eq!(media_type("holes.ncd"), "application/x-excellon");
        assert_eq!(media_type("top.GTL"), "application/x-gerber");
        assert_eq!(media_type("a.json"), "application/json");
        assert_eq!(media_type("tools.csv"), "text/csv");
        assert_eq!(media_type("noext"), "application/octet-stream");
        assert_eq!(media_type("dir.d/noext"), "application/octet-stream");
        assert_eq!(media_type(""), "application/octet-stream");
        // every type is in lower case and has a slash, as the server wants it
        for f in [
            "a.nc", "a.plt", "a.dxf", "a.ncd", "a.gtl", "a.svg", "a.png", "a.json", "a.csv",
            "a.txt", "a.zzz",
        ] {
            let m = media_type(f);
            assert!(m.contains('/') && m == m.to_ascii_lowercase(), "{m}");
        }
    }
}
