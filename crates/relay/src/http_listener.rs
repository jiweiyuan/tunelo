//! Public HTTP listener — accepts browser connections and routes to tunnels.
//!
//! Design principles:
//! 1. Parse ONLY what's needed — Host header + auth info
//! 2. Never buffer the full request on the relay path
//! 3. Use copy_bidirectional for zero-copy relay
//!
//! Private tunnel auth flow:
//!   1. URL with `?pwd=<password>` → validate → Set-Cookie → redirect to clean URL
//!   2. Cookie `__tunelo_password` → validate → relay
//!   3. No auth → serve a password input page
//!   4. POST /__tunelo_verify → validate form body → Set-Cookie → redirect

use std::sync::Arc;

use anyhow::Result;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tracing::{debug, info, info_span, warn, Instrument};

use crate::auth::constant_time_eq;
use crate::proxy;
use crate::router::Router;

const PEEK_BUF_SIZE: usize = 8192;

pub async fn run_http_listener(addr: String, router: Arc<Router>) -> Result<()> {
    let listener = TcpListener::bind(&addr).await?;
    info!(addr = %addr, "HTTP listener started");

    loop {
        let (stream, remote) = listener.accept().await?;
        let router = router.clone();

        tokio::spawn(
            async move {
                if let Err(e) = handle_connection(stream, &router).await {
                    debug!(error = %e, "connection error");
                }
            }
            .instrument(info_span!("http", %remote)),
        );
    }
}

async fn handle_connection(mut stream: TcpStream, router: &Router) -> Result<()> {
    stream.set_nodelay(true)?;

    let mut peek_buf = [0u8; PEEK_BUF_SIZE];
    let n = stream.peek(&mut peek_buf).await?;
    if n == 0 {
        return Ok(());
    }
    let raw = &peek_buf[..n];

    let host = match extract_host(raw) {
        Some(h) => h,
        None => {
            send_error(&mut stream, 400, "Missing Host header").await;
            return Ok(());
        }
    };

    let subdomain = match extract_subdomain(&host) {
        Some(s) => s,
        None => {
            send_error(&mut stream, 400, "Invalid hostname").await;
            return Ok(());
        }
    };

    let session = match router.get(&subdomain) {
        Some(s) => s,
        None => {
            send_error(&mut stream, 502, &format!("No tunnel: {host}")).await;
            return Ok(());
        }
    };

    // ── Public tunnel: relay directly ──────────────────────────────────
    let expected = match &session.password {
        Some(pw) => pw,
        None => {
            debug!(subdomain = %subdomain, "routing (public)");
            if let Err(e) = proxy::relay_connection(&session, stream).await {
                warn!(error = %e, subdomain = %subdomain, "relay error");
            }
            return Ok(());
        }
    };

    // ── Private tunnel: check auth ────────────────────────────────────

    // 1. Cookie — subsequent visits (most common, check first)
    if let Some(ref c) = extract_cookie(raw, "__tunelo_password") {
        if constant_time_eq(c.as_bytes(), expected.as_bytes()) {
            debug!(subdomain = %subdomain, "routing (cookie auth)");
            if let Err(e) = proxy::relay_connection(&session, stream).await {
                warn!(error = %e, subdomain = %subdomain, "relay error");
            }
            return Ok(());
        }
    }

    // 2. ?pwd= in URL — first click from shared link
    if let Some(ref pwd) = extract_query_param(raw, "pwd") {
        if constant_time_eq(pwd.as_bytes(), expected.as_bytes()) {
            // Consume the request (don't relay), set cookie, redirect to /
            consume_request(&mut stream).await;
            send_auth_redirect(&mut stream, expected).await;
            debug!(subdomain = %subdomain, "pwd accepted, set cookie");
            return Ok(());
        }
    }

    // 3. POST /__tunelo_verify — form submission from password page
    let method = extract_method(raw);
    let path = extract_path(raw);
    if method == Some("POST") && path == Some("/__tunelo_verify") {
        let body = consume_and_read_body(&mut stream).await;
        if let Some(pw) = extract_form_field(&body, "password") {
            if constant_time_eq(pw.as_bytes(), expected.as_bytes()) {
                send_auth_redirect(&mut stream, expected).await;
                debug!(subdomain = %subdomain, "form auth accepted");
                return Ok(());
            }
        }
        // Wrong password — show auth page again with error
        consume_request(&mut stream).await;
        send_auth_page(&mut stream, true).await;
        return Ok(());
    }

    // 4. No valid auth — serve the password input page
    consume_request(&mut stream).await;
    send_auth_page(&mut stream, false).await;
    Ok(())
}

// ─── Auth page HTML ──────────────────────────────────────────────────────────

async fn send_auth_page(stream: &mut TcpStream, show_error: bool) {
    let error_html = if show_error {
        r#"<p style="color:#e53e3e;margin:0 0 16px;font-size:14px">Incorrect password. Try again.</p>"#
    } else {
        ""
    };

    let body = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>Access Required</title>
<style>
  * {{ margin:0; padding:0; box-sizing:border-box }}
  body {{ font-family:-apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif;
         background:#f7f7f8; display:flex; align-items:center; justify-content:center;
         min-height:100vh; color:#1a1a1a }}
  .card {{ background:#fff; border-radius:12px; padding:40px; width:100%;
           max-width:380px; box-shadow:0 2px 12px rgba(0,0,0,0.08) }}
  .icon {{ width:48px; height:48px; background:#0061FE; border-radius:10px;
           display:flex; align-items:center; justify-content:center; margin-bottom:20px }}
  .icon svg {{ color:white }}
  h1 {{ font-size:20px; font-weight:600; margin-bottom:6px }}
  .sub {{ color:#637282; font-size:14px; margin-bottom:24px }}
  input {{ width:100%; padding:10px 14px; border:1.5px solid #d4dce5;
           border-radius:8px; font-size:15px; outline:none;
           transition:border-color 0.15s }}
  input:focus {{ border-color:#0061FE }}
  button {{ width:100%; padding:10px; background:#0061FE; color:#fff;
            border:none; border-radius:8px; font-size:15px; font-weight:500;
            cursor:pointer; margin-top:12px; transition:background 0.15s }}
  button:hover {{ background:#0050D4 }}
  button:active {{ transform:scale(0.98) }}
</style>
</head>
<body>
<div class="card">
  <div class="icon">
    <svg width="24" height="24" fill="none" stroke="white" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" viewBox="0 0 24 24">
      <rect x="3" y="11" width="18" height="11" rx="2" ry="2"/><path d="M7 11V7a5 5 0 0 1 10 0v4"/>
    </svg>
  </div>
  <h1>Password Required</h1>
  <p class="sub">This tunnel is protected. Enter the password to continue.</p>
  {error_html}
  <form method="POST" action="/__tunelo_verify">
    <input type="password" name="password" placeholder="Password" autofocus autocomplete="off" required>
    <button type="submit">Continue</button>
  </form>
</div>
</body>
</html>"#
    );

    let header = format!(
        "HTTP/1.1 200 OK\r\n\
         Content-Type: text/html; charset=utf-8\r\n\
         Content-Length: {}\r\n\
         Cache-Control: no-store\r\n\
         Connection: close\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(header.as_bytes()).await;
    let _ = stream.write_all(body.as_bytes()).await;
}

async fn send_auth_redirect(stream: &mut TcpStream, password: &str) {
    let resp = format!(
        "HTTP/1.1 302 Found\r\n\
         Location: /\r\n\
         Set-Cookie: __tunelo_password={password}; Path=/; HttpOnly; SameSite=Strict; Max-Age=86400\r\n\
         Content-Length: 0\r\n\
         Connection: close\r\n\r\n"
    );
    let _ = stream.write_all(resp.as_bytes()).await;
}

// ─── HTTP parsing helpers ────────────────────────────────────────────────────

fn extract_host(raw: &[u8]) -> Option<String> {
    let mut headers = [httparse::EMPTY_HEADER; 32];
    let mut req = httparse::Request::new(&mut headers);
    let _ = req.parse(raw);

    for h in req.headers.iter() {
        if h.name.eq_ignore_ascii_case("host") {
            let val = std::str::from_utf8(h.value).ok()?;
            return Some(val.split(':').next().unwrap_or(val).to_string());
        }
    }
    None
}

fn extract_subdomain(host: &str) -> Option<String> {
    let dot = host.find('.')?;
    if dot == 0 { return None; }
    Some(host[..dot].to_lowercase())
}

fn extract_method(raw: &[u8]) -> Option<&str> {
    let mut headers = [httparse::EMPTY_HEADER; 32];
    let mut req = httparse::Request::new(&mut headers);
    let _ = req.parse(raw);
    req.method
}

fn extract_path(raw: &[u8]) -> Option<&str> {
    let mut headers = [httparse::EMPTY_HEADER; 32];
    let mut req = httparse::Request::new(&mut headers);
    let _ = req.parse(raw);
    req.path.map(|p| p.split('?').next().unwrap_or(p))
}

fn extract_query_param(raw: &[u8], key: &str) -> Option<String> {
    let mut headers = [httparse::EMPTY_HEADER; 32];
    let mut req = httparse::Request::new(&mut headers);
    let _ = req.parse(raw);

    let path = req.path?;
    let query = path.split('?').nth(1)?;
    for pair in query.split('&') {
        let mut kv = pair.splitn(2, '=');
        if kv.next()? == key {
            return Some(kv.next().unwrap_or("").to_string());
        }
    }
    None
}

fn extract_cookie(raw: &[u8], name: &str) -> Option<String> {
    let mut headers = [httparse::EMPTY_HEADER; 32];
    let mut req = httparse::Request::new(&mut headers);
    let _ = req.parse(raw);

    let prefix = format!("{name}=");
    for h in req.headers.iter() {
        if h.name.eq_ignore_ascii_case("cookie") {
            if let Ok(cookies) = std::str::from_utf8(h.value) {
                for pair in cookies.split(';') {
                    let pair = pair.trim();
                    if let Some(val) = pair.strip_prefix(&prefix) {
                        return Some(val.to_string());
                    }
                }
            }
        }
    }
    None
}

fn extract_form_field(body: &str, key: &str) -> Option<String> {
    let prefix = format!("{key}=");
    for pair in body.split('&') {
        if let Some(val) = pair.strip_prefix(&prefix) {
            return Some(urldecode(val));
        }
    }
    None
}

fn urldecode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut bytes = s.bytes();
    while let Some(b) = bytes.next() {
        match b {
            b'%' => {
                let hi = bytes.next().and_then(hex_val);
                let lo = bytes.next().and_then(hex_val);
                if let (Some(h), Some(l)) = (hi, lo) {
                    out.push((h << 4 | l) as char);
                }
            }
            b'+' => out.push(' '),
            _ => out.push(b as char),
        }
    }
    out
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// Consume the peeked request from the socket (so we can write a response).
async fn consume_request(stream: &mut TcpStream) {
    let mut buf = [0u8; PEEK_BUF_SIZE];
    let _ = stream.read(&mut buf).await;
}

/// Consume the request and return the body as a string.
async fn consume_and_read_body(stream: &mut TcpStream) -> String {
    let mut buf = vec![0u8; PEEK_BUF_SIZE];
    let n = stream.read(&mut buf).await.unwrap_or(0);
    let raw = std::str::from_utf8(&buf[..n]).unwrap_or("");
    // Body starts after \r\n\r\n
    raw.split("\r\n\r\n").nth(1).unwrap_or("").to_string()
}

async fn send_error(stream: &mut TcpStream, status: u16, msg: &str) {
    let reason = match status {
        400 => "Bad Request",
        502 => "Bad Relay",
        _ => "Error",
    };
    let body = format!("{status} {reason}: {msg}\n");
    let resp = format!(
        "HTTP/1.1 {status} {reason}\r\n\
         Content-Type: text/plain\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\r\n\
         {body}",
        body.len()
    );
    let _ = stream.write_all(resp.as_bytes()).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_host() {
        let raw = b"GET /path HTTP/1.1\r\nHost: abc.tunelo.net\r\nAccept: */*\r\n\r\n";
        assert_eq!(extract_host(raw).unwrap(), "abc.tunelo.net");
    }

    #[test]
    fn test_extract_host_with_port() {
        let raw = b"GET / HTTP/1.1\r\nHost: abc.tunelo.net:8080\r\n\r\n";
        assert_eq!(extract_host(raw).unwrap(), "abc.tunelo.net");
    }

    #[test]
    fn test_extract_subdomain() {
        assert_eq!(extract_subdomain("abc.tunelo.net"), Some("abc".into()));
        assert_eq!(extract_subdomain("myapp.localhost"), Some("myapp".into()));
        assert_eq!(extract_subdomain("localhost"), None);
        assert_eq!(extract_subdomain(".bad"), None);
    }

    #[test]
    fn test_extract_query_param() {
        let raw = b"GET /?pwd=hello123&foo=bar HTTP/1.1\r\nHost: x.tunelo.net\r\n\r\n";
        assert_eq!(extract_query_param(raw, "pwd"), Some("hello123".into()));
        assert_eq!(extract_query_param(raw, "foo"), Some("bar".into()));
        assert_eq!(extract_query_param(raw, "nope"), None);
    }

    #[test]
    fn test_extract_cookie() {
        let raw = b"GET / HTTP/1.1\r\nHost: x.y\r\nCookie: foo=bar; __tunelo_password=secret123\r\n\r\n";
        assert_eq!(extract_cookie(raw, "__tunelo_password"), Some("secret123".into()));
        assert_eq!(extract_cookie(raw, "foo"), Some("bar".into()));
        assert_eq!(extract_cookie(raw, "nope"), None);
    }

    #[test]
    fn test_extract_form_field() {
        assert_eq!(extract_form_field("password=hello&x=1", "password"), Some("hello".into()));
        assert_eq!(extract_form_field("password=hello+world", "password"), Some("hello world".into()));
        assert_eq!(extract_form_field("password=a%20b", "password"), Some("a b".into()));
        assert_eq!(extract_form_field("x=1", "password"), None);
    }

    #[test]
    fn test_extract_method_and_path() {
        let raw = b"POST /__tunelo_verify HTTP/1.1\r\nHost: x.y\r\n\r\npassword=abc";
        assert_eq!(extract_method(raw), Some("POST"));
        assert_eq!(extract_path(raw), Some("/__tunelo_verify"));

        let raw = b"GET /foo?bar=1 HTTP/1.1\r\nHost: x.y\r\n\r\n";
        assert_eq!(extract_method(raw), Some("GET"));
        assert_eq!(extract_path(raw), Some("/foo"));
    }
}
