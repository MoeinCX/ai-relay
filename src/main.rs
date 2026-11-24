//! AI Relay setup tool.
//!
//! Runs a tiny web UI on 127.0.0.1 (opened in your own browser) and talks to the Cloudflare API
//! directly from this computer. Your Cloudflare token goes only to api.cloudflare.com: there is
//! no other server, nothing is stored on disk, and the program exits when you close it.

use std::io::Read;
use std::time::{Duration, Instant};

use serde_json::{json, Value};
use tiny_http::{Header, Method, Request, Response, Server};

const UI: &str = include_str!("../ui/index.html");
const WORKER: &str = include_str!("../worker/worker.js");

/// The UI font, only in release builds (see build.rs); otherwise the page falls back to the system font.
#[cfg(has_fonts)]
fn font(path: &str) -> Option<&'static [u8]> {
    match path {
        "/fonts/Satrap-Regular.woff2" => Some(include_bytes!("../fonts/Satrap-Regular.woff2")),
        "/fonts/Satrap-Bold.woff2" => Some(include_bytes!("../fonts/Satrap-Bold.woff2")),
        "/fonts/Satrap-ExtraBold.woff2" => Some(include_bytes!("../fonts/Satrap-ExtraBold.woff2")),
        _ => None,
    }
}
#[cfg(not(has_fonts))]
fn font(_path: &str) -> Option<&'static [u8]> {
    None
}
const CF: &str = "https://api.cloudflare.com/client/v4";
const IDLE_LIMIT: Duration = Duration::from_secs(60 * 60);

fn main() {
    let server = Server::http("127.0.0.1:0").expect("cannot open a local port");
    let port = server.server_addr().to_ip().map(|a| a.port()).unwrap_or(0);
    let nonce = random_hex(16);
    let url = format!("http://127.0.0.1:{port}/#k={nonce}");

    println!("AI Relay setup v{}", env!("CARGO_PKG_VERSION"));
    println!("Opening in your browser: {url}");
    println!("If it does not open, copy the address above into your browser.");
    println!("Keep this window open while you work; close it (or press Ctrl+C) when done.");
    if std::env::var_os("AI_RELAY_NO_BROWSER").is_none() {
        let _ = webbrowser::open(&url);
    }

    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(15))
        .timeout(Duration::from_secs(60))
        .user_agent(concat!("ai-relay-setup/", env!("CARGO_PKG_VERSION")))
        .build();

    let mut last = Instant::now();
    loop {
        let req = match server.recv_timeout(Duration::from_secs(30)) {
            Ok(Some(r)) => r,
            Ok(None) => {
                if last.elapsed() > IDLE_LIMIT {
                    println!("Idle for an hour, exiting.");
                    return;
                }
                continue;
            }
            Err(_) => return,
        };
        last = Instant::now();
        if handle(req, &agent, port, &nonce) {
            println!("Closed from the browser. Bye.");
            return;
        }
    }
}

/// Returns true when the user asked to quit.
fn handle(mut req: Request, agent: &ureq::Agent, port: u16, nonce: &str) -> bool {
    // DNS-rebinding guard: only answer when addressed as our own loopback host.
    let host_ok = req
        .headers()
        .iter()
        .find(|h| h.field.equiv("Host"))
        .map(|h| h.value.as_str() == format!("127.0.0.1:{port}"))
        .unwrap_or(false);
    if !host_ok {
        let _ = req.respond(Response::from_string("forbidden").with_status_code(403));
        return false;
    }

    let path = req.url().split('?').next().unwrap_or("/").to_string();
    if *req.method() == Method::Get && (path == "/" || path == "/index.html") {
        let _ = req.respond(with_headers(Response::from_string(UI), "text/html; charset=utf-8"));
        return false;
    }
    if *req.method() == Method::Get && path == "/worker.js" {
        let _ = req.respond(with_headers(Response::from_string(WORKER), "text/javascript; charset=utf-8"));
        return false;
    }
    if *req.method() == Method::Get && path.starts_with("/fonts/") {
        let _ = match font(&path) {
            Some(b) => req.respond(with_headers(Response::from_data(b.to_vec()), "font/woff2")),
            None => req.respond(Response::from_string("not found").with_status_code(404)),
        };
        return false;
    }
    if *req.method() != Method::Post || !path.starts_with("/api/") {
        let _ = req.respond(Response::from_string("not found").with_status_code(404));
        return false;
    }

    // Every API call must carry the per-run key from the opened URL (other web pages cannot know it,
    // and the custom header forces a CORS preflight that we never approve).
    let key_ok = req
        .headers()
        .iter()
        .find(|h| h.field.equiv("X-Relay-Key"))
        .map(|h| h.value.as_str() == nonce)
        .unwrap_or(false);
    if !key_ok {
        let _ = req.respond(Response::from_string("forbidden").with_status_code(403));
        return false;
    }

    let mut raw = String::new();
    let _ = req.as_reader().take(16 * 1024).read_to_string(&mut raw);
    let body: Value = serde_json::from_str(&raw).unwrap_or(Value::Null);

    let quit = path == "/api/quit";
    let out = match path.as_str() {
        "/api/verify" => verify(agent, &body),
        "/api/deploy" => deploy(agent, &body).map(|v| {
            remember(&v, &body);
            v
        }),
        "/api/delete" => delete(agent, &body).map(|v| {
            forget_by(|r| r["account_id"] == body["account_id"] && r["name"] == body["name"]);
            v
        }),
        "/api/saved" => Ok(json!({ "ok": true, "relays": load_saved(), "file": saved_path().map(|p| p.display().to_string()) })),
        "/api/save" => save_manual(&body),
        "/api/forget" => {
            let url = body["url"].clone();
            forget_by(|r| r["url"] == url);
            Ok(json!({ "ok": true }))
        }
        "/api/quit" => Ok(json!({ "ok": true })),
        _ => Err(json!({ "error": "unknown_action" })),
    };
    let (status, v) = match out {
        Ok(v) => (200, v),
        Err(v) => (400, v),
    };
    let _ = req.respond(with_headers(Response::from_string(v.to_string()), "application/json").with_status_code(status));
    quit
}

fn with_headers<R: Read>(r: Response<R>, ctype: &str) -> Response<R> {
    r.with_header(Header::from_bytes("Content-Type", ctype).unwrap())
        .with_header(Header::from_bytes("Cache-Control", "no-store").unwrap())
        .with_header(Header::from_bytes("X-Content-Type-Options", "nosniff").unwrap())
        .with_header(Header::from_bytes("Referrer-Policy", "no-referrer").unwrap())
}

// ───────────────────────── Cloudflare API ─────────────────────────

type Res = Result<Value, Value>;

/// One Cloudflare API call. Returns (http status, json body); status 0 = network failure.
fn cf(agent: &ureq::Agent, method: &str, path: &str, token: &str, body: Option<(&str, Vec<u8>)>) -> (u16, Value) {
    let req = agent.request(method, &format!("{CF}{path}")).set("Authorization", &format!("Bearer {token}"));
    let resp = match body {
        Some((ctype, bytes)) => req.set("Content-Type", ctype).send_bytes(&bytes),
        None => req.call(),
    };
    let r = match resp {
        Ok(r) => r,
        Err(ureq::Error::Status(_, r)) => r,
        Err(e) => return (0, json!({ "success": false, "errors": [{ "code": 0, "message": e.to_string() }] })),
    };
    let status = r.status();
    let v = r.into_string().ok().and_then(|s| serde_json::from_str::<Value>(&s).ok()).unwrap_or_else(|| json!({ "success": false, "errors": [{ "code": status, "message": "bad_response" }] }));
    (status, v)
}

fn ok(v: &Value) -> bool {
    v["success"].as_bool().unwrap_or(false)
}

fn fail(step: &str, status: u16, v: &Value) -> Value {
    if status == 0 {
        return json!({ "error": "cloudflare_unreachable", "cf_message": v["errors"][0]["message"] });
    }
    let step = if status == 401 || status == 403 { if step == "token_invalid" { step } else { "no_permission" } } else { step };
    json!({ "error": step, "cf_code": v["errors"][0]["code"], "cf_message": v["errors"][0]["message"] })
}

fn token_of(b: &Value) -> Result<String, Value> {
    let t = b["token"].as_str().unwrap_or("").trim().to_string();
    if t.len() < 30 || t.len() > 120 || !t.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
        return Err(json!({ "error": "invalid_token_format" }));
    }
    Ok(t)
}
fn account_of(b: &Value) -> Result<String, Value> {
    let a = b["account_id"].as_str().unwrap_or("").trim().to_ascii_lowercase();
    if a.len() != 32 || !a.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(json!({ "error": "invalid_account_id" }));
    }
    Ok(a)
}
fn name_of(b: &Value) -> Result<String, Value> {
    let n = b["name"].as_str().unwrap_or("").trim().to_ascii_lowercase();
    let valid = n.len() >= 2
        && n.len() <= 42
        && n.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        && !n.starts_with('-')
        && !n.ends_with('-');
    if !valid {
        return Err(json!({ "error": "invalid_worker_name" }));
    }
    Ok(n)
}

fn verify(agent: &ureq::Agent, b: &Value) -> Res {
    let token = token_of(b)?;
    let (s, v) = cf(agent, "GET", "/user/tokens/verify", &token, None);
    let (s2, accts) = cf(agent, "GET", "/accounts?per_page=50", &token, None);
    if !ok(&v) && !ok(&accts) {
        // Account-owned tokens are verified per account: needs the Account ID from the user.
        let acc = b["account_id"].as_str().unwrap_or("").trim().to_ascii_lowercase();
        if acc.len() == 32 && acc.chars().all(|c| c.is_ascii_hexdigit()) {
            let (s3, v3) = cf(agent, "GET", &format!("/accounts/{acc}/tokens/verify"), &token, None);
            if ok(&v3) {
                return Ok(json!({ "ok": true, "accounts": [], "needAccountId": true, "accountToken": true }));
            }
            return Err(fail("token_invalid", if s3 == 0 { 0 } else { 401 }, &v3));
        }
        return Err(fail("token_invalid", if s == 0 { 0 } else { s.max(s2) }, &v));
    }
    let list: Vec<Value> = accts["result"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|x| Some(json!({ "id": x["id"].as_str()?, "name": x["name"].as_str().unwrap_or("") })))
                .collect()
        })
        .unwrap_or_default();
    let need = list.is_empty();
    Ok(json!({ "ok": true, "accounts": list, "needAccountId": need }))
}

fn deploy(agent: &ureq::Agent, b: &Value) -> Res {
    let token = token_of(b)?;
    let acc = account_of(b)?;
    let name = name_of(b)?;
    let secret = b["secret"].as_str().unwrap_or("").to_string();
    if secret.len() < 16 || secret.len() > 80 || !secret.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        return Err(json!({ "error": "invalid_secret" }));
    }

    // 1) The account's workers.dev subdomain (new accounts have none yet: create one).
    let (s, v) = cf(agent, "GET", &format!("/accounts/{acc}/workers/subdomain"), &token, None);
    if s == 0 {
        return Err(fail("cloudflare_unreachable", 0, &v));
    }
    let mut sub = v["result"]["subdomain"].as_str().unwrap_or("").to_string();
    let mut created = false;
    if sub.is_empty() {
        if s == 401 || s == 403 {
            return Err(fail("no_permission", s, &v));
        }
        let mut last = (s, v);
        for _ in 0..4 {
            let try_sub = format!("ai-{}", random_hex(4));
            let body = json!({ "subdomain": try_sub }).to_string().into_bytes();
            let (s2, v2) = cf(agent, "PUT", &format!("/accounts/{acc}/workers/subdomain"), &token, Some(("application/json", body)));
            if ok(&v2) {
                sub = v2["result"]["subdomain"].as_str().unwrap_or(&try_sub).to_string();
                created = true;
                break;
            }
            if s2 == 401 || s2 == 403 {
                return Err(fail("no_permission", s2, &v2));
            }
            last = (s2, v2);
        }
        if sub.is_empty() {
            return Err(fail("subdomain_failed", last.0, &last.1));
        }
    }

    // 2) Upload the worker (ES module) with SECRET as an encrypted secret_text binding.
    let meta = json!({
        "main_module": "worker.js",
        "compatibility_date": "2025-01-01",
        "bindings": [{ "type": "secret_text", "name": "SECRET", "text": secret }],
    });
    let boundary = format!("----airelay{}", random_hex(12));
    let mut mp = Vec::new();
    push_part(&mut mp, &boundary, "metadata", "metadata.json", "application/json", meta.to_string().as_bytes());
    push_part(&mut mp, &boundary, "worker.js", "worker.js", "application/javascript+module", WORKER.as_bytes());
    mp.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
    let ctype = format!("multipart/form-data; boundary={boundary}");
    let (s, v) = cf(agent, "PUT", &format!("/accounts/{acc}/workers/scripts/{name}"), &token, Some((&ctype, mp)));
    if !ok(&v) {
        return Err(fail("upload_failed", s, &v));
    }

    // 3) Turn on the workers.dev address for this worker.
    let body = json!({ "enabled": true, "previews_enabled": false }).to_string().into_bytes();
    let (s, v) = cf(agent, "POST", &format!("/accounts/{acc}/workers/scripts/{name}/subdomain"), &token, Some(("application/json", body)));
    if !ok(&v) {
        return Err(fail("enable_failed", s, &v));
    }

    let host = format!("{name}.{sub}.workers.dev");
    Ok(json!({ "ok": true, "host": host, "url": format!("https://{host}"), "subdomainCreated": created }))
}

fn delete(agent: &ureq::Agent, b: &Value) -> Res {
    let token = token_of(b)?;
    let acc = account_of(b)?;
    let name = name_of(b)?;
    let (s, v) = cf(agent, "DELETE", &format!("/accounts/{acc}/workers/scripts/{name}"), &token, None);
    if !ok(&v) {
        return Err(fail("delete_failed", s, &v));
    }
    Ok(json!({ "ok": true }))
}

// ───────────────────────── Saved relays (this computer only) ─────────────────────────
//
// After a successful build the relay's address and secret are kept in a small JSON file in the
// user's own profile folder, so running the tool again shows every service's address at once,
// with no token and no setup steps. Nothing else is stored (never the Cloudflare token).

fn saved_path() -> Option<std::path::PathBuf> {
    let base = std::env::var_os("APPDATA")
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("XDG_CONFIG_HOME").map(std::path::PathBuf::from))
        .or_else(|| std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".config")))?;
    Some(base.join("ai-relay").join("relays.json"))
}

fn load_saved() -> Vec<Value> {
    saved_path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str::<Value>(&s).ok())
        .and_then(|v| v["relays"].as_array().cloned())
        .unwrap_or_default()
}

fn write_saved(list: &[Value]) {
    if let Some(p) = saved_path() {
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(&p, serde_json::to_string_pretty(&json!({ "version": 1, "relays": list })).unwrap_or_default());
    }
}

fn now_secs() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// Keeps the newest relay first; one entry per worker (same account + name ⇒ replaced, e.g. after a new secret).
fn upsert(entry: Value) {
    let mut list: Vec<Value> = load_saved()
        .into_iter()
        .filter(|r| r["url"] != entry["url"] && !(r["name"] == entry["name"] && r["account_id"] == entry["account_id"] && !entry["account_id"].is_null()))
        .collect();
    list.insert(0, entry);
    list.truncate(10);
    write_saved(&list);
}

fn remember(result: &Value, body: &Value) {
    upsert(json!({
        "url": result["url"],
        "secret": body["secret"],
        "name": body["name"],
        "account_id": body["account_id"],
        "created": now_secs(),
    }));
}

fn forget_by(pred: impl Fn(&Value) -> bool) {
    let list: Vec<Value> = load_saved().into_iter().filter(|r| !pred(r)).collect();
    write_saved(&list);
}

/// Saves a relay the user pasted (an address from earlier, or one built by hand).
fn save_manual(b: &Value) -> Res {
    let url = b["url"].as_str().unwrap_or("");
    let secret = b["secret"].as_str().unwrap_or("");
    let url_ok = url.starts_with("https://") && url.len() < 200 && !url[8..].contains('/');
    let secret_ok = (16..=80).contains(&secret.len()) && secret.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if !url_ok || !secret_ok {
        return Err(json!({ "error": "invalid_address" }));
    }
    upsert(json!({ "url": url, "secret": secret, "name": Value::Null, "account_id": Value::Null, "created": now_secs() }));
    Ok(json!({ "ok": true }))
}

fn push_part(out: &mut Vec<u8>, boundary: &str, name: &str, file: &str, ctype: &str, data: &[u8]) {
    out.extend_from_slice(
        format!("--{boundary}\r\nContent-Disposition: form-data; name=\"{name}\"; filename=\"{file}\"\r\nContent-Type: {ctype}\r\n\r\n")
            .as_bytes(),
    );
    out.extend_from_slice(data);
    out.extend_from_slice(b"\r\n");
}

fn random_hex(bytes: usize) -> String {
    let mut b = vec![0u8; bytes];
    getrandom::getrandom(&mut b).expect("no randomness");
    b.iter().map(|x| format!("{x:02x}")).collect()
}
