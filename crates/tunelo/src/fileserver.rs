//! Built-in file server with embedded React frontend.
//!
//! Routes:
//!   /_api/ls?path=/          → JSON directory listing
//!   /_api/raw?path=/foo.txt  → raw file content (with Range support)
//!   /*                       → SPA frontend (embedded at compile time)
//!
//! Uses hyper for proper HTTP/1.1: keep-alive, full header parsing,
//! correct content-length, streaming responses.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Result;
use futures_util::StreamExt;
use http_body_util::{BodyExt, Full, StreamBody};
use hyper::body::{Bytes, Frame};
use hyper::header::{
    ACCEPT_RANGES, ACCESS_CONTROL_ALLOW_ORIGIN, CACHE_CONTROL, CONTENT_LENGTH, CONTENT_RANGE,
    CONTENT_TYPE, RANGE,
};
use hyper::{Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use include_dir::{include_dir, Dir};
use serde::Serialize;
use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;
use tokio_util::io::ReaderStream;
use tracing::{debug, info_span, Instrument};

type BoxBody = http_body_util::combinators::BoxBody<Bytes, std::io::Error>;

/// Embedded frontend build output.
static FRONTEND: Dir = include_dir!("$CARGO_MANIFEST_DIR/../../web/dist");

/// Start file server on a fixed port (for local-only mode).
pub async fn start_on_port(root: PathBuf, port: u16) -> Result<u16> {
    let listener = TcpListener::bind(format!("127.0.0.1:{port}")).await?;
    let port = listener.local_addr()?.port();
    spawn_server(listener, root.canonicalize()?);
    Ok(port)
}

/// Start file server on a random port (for tunnel mode).
pub async fn start_background(root: PathBuf) -> Result<u16> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let port = listener.local_addr()?.port();
    spawn_server(listener, root.canonicalize()?);
    Ok(port)
}

fn spawn_server(listener: TcpListener, root: PathBuf) {
    let root = Arc::new(root);
    tokio::spawn(async move {
        loop {
            let (stream, remote) = match listener.accept().await {
                Ok(s) => s,
                Err(_) => return,
            };
            let root = root.clone();
            tokio::spawn(
                async move {
                    let io = TokioIo::new(stream);
                    let service = hyper::service::service_fn(move |req| {
                        let root = root.clone();
                        async move { handle(req, &root).await }
                    });
                    // HTTP/1.1 with keep-alive
                    if let Err(e) = hyper_util::server::conn::auto::Builder::new(
                        hyper_util::rt::TokioExecutor::new(),
                    )
                    .http1()
                    .keep_alive(true)
                    .serve_connection(io, service)
                    .await
                    {
                        debug!(error = %e, "connection error");
                    }
                }
                .instrument(info_span!("file", %remote)),
            );
        }
    });
}

// ─── Request routing ─────────────────────────────────────────────────────────

async fn handle(req: Request<hyper::body::Incoming>, root: &Path) -> Result<Response<BoxBody>, std::io::Error> {
    let path = req.uri().path();
    debug!(method = %req.method(), path, "request");

    if path.starts_with("/_api/ls") {
        Ok(handle_api_ls(root, req.uri()).await)
    } else if path.starts_with("/_api/raw") {
        Ok(handle_api_raw(root, &req).await)
    } else {
        Ok(handle_spa(path))
    }
}

// ─── API: directory listing ──────────────────────────────────────────────────

#[derive(Serialize)]
struct FileEntry {
    name: String,
    is_dir: bool,
    size: u64,
}

async fn handle_api_ls(root: &Path, uri: &hyper::Uri) -> Response<BoxBody> {
    let query_path = extract_query_param(uri, "path").unwrap_or_else(|| "/".into());
    let clean = sanitize_path(&query_path);
    let fs_path = root.join(clean.trim_start_matches('/'));

    let resolved = match fs_path.canonicalize() {
        Ok(p) if p.starts_with(root) && p.is_dir() => p,
        _ => return json_response(StatusCode::NOT_FOUND, b"[]"),
    };

    let mut entries = Vec::new();
    let mut dir = match tokio::fs::read_dir(&resolved).await {
        Ok(d) => d,
        Err(_) => return json_response(StatusCode::INTERNAL_SERVER_ERROR, b"[]"),
    };

    while let Ok(Some(entry)) = dir.next_entry().await {
        let name = entry.file_name().to_string_lossy().to_string();
        if let Ok(meta) = entry.metadata().await {
            entries.push(FileEntry {
                name,
                is_dir: meta.is_dir(),
                size: meta.len(),
            });
        }
    }

    // Sort: dirs first, then alphabetical
    entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
    });

    let json = serde_json::to_vec(&entries).unwrap_or_default();
    json_response(StatusCode::OK, &json)
}

// ─── API: raw file content (with Range support) ─────────────────────────────

async fn handle_api_raw(root: &Path, req: &Request<hyper::body::Incoming>) -> Response<BoxBody> {
    let query_path = extract_query_param(req.uri(), "path").unwrap_or_else(|| "/".into());
    let clean = sanitize_path(&query_path);
    let fs_path = root.join(clean.trim_start_matches('/'));

    let resolved = match fs_path.canonicalize() {
        Ok(p) if p.starts_with(root) && p.is_file() => p,
        _ => return text_response(StatusCode::NOT_FOUND, "Not Found"),
    };

    let mime = guess_mime(&resolved);
    let file_size = match tokio::fs::metadata(&resolved).await {
        Ok(m) => m.len(),
        Err(_) => return text_response(StatusCode::INTERNAL_SERVER_ERROR, "Read error"),
    };

    // Parse Range header
    let range = req
        .headers()
        .get(RANGE)
        .and_then(|v| v.to_str().ok())
        .and_then(parse_range);

    match range {
        Some((start, end_opt)) => {
            let end = end_opt.unwrap_or(file_size - 1).min(file_size - 1);
            if start >= file_size || start > end {
                return Response::builder()
                    .status(StatusCode::RANGE_NOT_SATISFIABLE)
                    .header(CONTENT_RANGE, format!("bytes */{file_size}"))
                    .body(empty_body())
                    .unwrap();
            }

            let length = end - start + 1;

            let file = match tokio::fs::File::open(&resolved).await {
                Ok(f) => f,
                Err(_) => return text_response(StatusCode::INTERNAL_SERVER_ERROR, "Read error"),
            };

            use tokio::io::AsyncSeekExt;
            let mut file = file;
            if let Err(_) = file.seek(std::io::SeekFrom::Start(start)).await {
                return text_response(StatusCode::INTERNAL_SERVER_ERROR, "Seek error");
            }

            let limited = file.take(length);
            let stream = ReaderStream::new(limited);
            let body = StreamBody::new(
                stream.map(|r: Result<Bytes, std::io::Error>| r.map(Frame::data)),
            );

            Response::builder()
                .status(StatusCode::PARTIAL_CONTENT)
                .header(CONTENT_TYPE, mime)
                .header(CONTENT_RANGE, format!("bytes {start}-{end}/{file_size}"))
                .header(CONTENT_LENGTH, length)
                .header(ACCEPT_RANGES, "bytes")
                .header(ACCESS_CONTROL_ALLOW_ORIGIN, "*")
                .body(BoxBody::new(body))
                .unwrap()
        }
        None => {
            let file = match tokio::fs::File::open(&resolved).await {
                Ok(f) => f,
                Err(_) => return text_response(StatusCode::INTERNAL_SERVER_ERROR, "Read error"),
            };

            let stream = ReaderStream::new(file);
            let body = StreamBody::new(
                stream.map(|r: Result<Bytes, std::io::Error>| r.map(Frame::data)),
            );

            Response::builder()
                .status(StatusCode::OK)
                .header(CONTENT_TYPE, mime)
                .header(CONTENT_LENGTH, file_size)
                .header(ACCEPT_RANGES, "bytes")
                .header(ACCESS_CONTROL_ALLOW_ORIGIN, "*")
                .body(BoxBody::new(body))
                .unwrap()
        }
    }
}

// ─── SPA: serve embedded frontend ───────────────────────────────────────────

fn handle_spa(req_path: &str) -> Response<BoxBody> {
    let path = req_path.trim_start_matches('/');
    let file = FRONTEND
        .get_file(path)
        .or_else(|| FRONTEND.get_file("index.html"));

    match file {
        Some(f) => {
            let mime = guess_mime_str(if path.is_empty() { "index.html" } else { path });
            let body = f.contents();

            let mut builder = Response::builder()
                .status(StatusCode::OK)
                .header(CONTENT_TYPE, mime)
                .header(CONTENT_LENGTH, body.len());

            // Cache static assets (hashed filenames) aggressively
            if path.starts_with("assets/") {
                builder = builder.header(CACHE_CONTROL, "public, max-age=31536000, immutable");
            }

            builder.body(full_body(Bytes::from_static(body))).unwrap()
        }
        None => text_response(StatusCode::NOT_FOUND, "Not Found"),
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn full_body(data: Bytes) -> BoxBody {
    BoxBody::new(Full::new(data).map_err(|_| std::io::Error::other("infallible")))
}

fn empty_body() -> BoxBody {
    full_body(Bytes::new())
}

fn json_response(status: StatusCode, body: &[u8]) -> Response<BoxBody> {
    Response::builder()
        .status(status)
        .header(CONTENT_TYPE, "application/json; charset=utf-8")
        .header(CONTENT_LENGTH, body.len())
        .header(ACCESS_CONTROL_ALLOW_ORIGIN, "*")
        .body(full_body(Bytes::copy_from_slice(body)))
        .unwrap()
}

fn text_response(status: StatusCode, msg: &str) -> Response<BoxBody> {
    Response::builder()
        .status(status)
        .header(CONTENT_TYPE, "text/plain; charset=utf-8")
        .header(CONTENT_LENGTH, msg.len())
        .body(full_body(Bytes::copy_from_slice(msg.as_bytes())))
        .unwrap()
}

/// Parse `bytes=START-END` range value.
fn parse_range(val: &str) -> Option<(u64, Option<u64>)> {
    let range = val.strip_prefix("bytes=")?;
    let mut parts = range.splitn(2, '-');
    let start_str = parts.next()?;
    let end_str = parts.next()?;

    if start_str.is_empty() {
        // bytes=-500 → last 500 bytes (not supported yet, return None)
        return None;
    }

    let start: u64 = start_str.parse().ok()?;
    let end: Option<u64> = if end_str.is_empty() {
        None
    } else {
        Some(end_str.parse().ok()?)
    };
    Some((start, end))
}

fn extract_query_param(uri: &hyper::Uri, key: &str) -> Option<String> {
    let query = uri.query()?;
    for pair in query.split('&') {
        let mut kv = pair.splitn(2, '=');
        if kv.next()? == key {
            return Some(urldecode(kv.next().unwrap_or("")));
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

fn sanitize_path(path: &str) -> String {
    let decoded = urldecode(path);
    let mut parts: Vec<&str> = Vec::new();
    for seg in decoded.split('/') {
        match seg {
            "" | "." => continue,
            ".." => { parts.pop(); }
            s => parts.push(s),
        }
    }
    if parts.is_empty() { "/".into() } else { format!("/{}", parts.join("/")) }
}

fn guess_mime(path: &Path) -> &'static str {
    guess_mime_str(path.extension().and_then(|e| e.to_str()).unwrap_or(""))
}

fn guess_mime_str(name: &str) -> &'static str {
    let ext = name.rsplit('.').next().unwrap_or("").to_lowercase();
    match ext.as_str() {
        "html" | "htm" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" | "mjs" => "application/javascript; charset=utf-8",
        "json" => "application/json; charset=utf-8",
        "txt" | "log" | "md" | "csv" | "tsv" => "text/plain; charset=utf-8",
        "xml" => "application/xml; charset=utf-8",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "avif" => "image/avif",
        "ico" => "image/x-icon",
        "woff2" => "font/woff2",
        "woff" => "font/woff",
        "ttf" => "font/ttf",
        "pdf" => "application/pdf",
        "zip" => "application/zip",
        "gz" => "application/gzip",
        "wasm" => "application/wasm",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "mov" => "video/quicktime",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "ogg" => "audio/ogg",
        "flac" => "audio/flac",
        "m4a" => "audio/mp4",
        "xlsx" | "xls" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_path() {
        assert_eq!(sanitize_path("/"), "/");
        assert_eq!(sanitize_path("/foo/bar"), "/foo/bar");
        assert_eq!(sanitize_path("/../../../etc/passwd"), "/etc/passwd");
        assert_eq!(sanitize_path("/foo/../bar"), "/bar");
        assert_eq!(sanitize_path(""), "/");
        assert_eq!(sanitize_path("/../../.."), "/");
    }

    #[test]
    fn test_urldecode() {
        assert_eq!(urldecode("/foo%20bar"), "/foo bar");
        assert_eq!(urldecode("/hello%2Fworld"), "/hello/world");
        assert_eq!(urldecode("hello+world"), "hello world");
    }

    #[test]
    fn test_parse_range() {
        assert_eq!(parse_range("bytes=0-1023"), Some((0, Some(1023))));
        assert_eq!(parse_range("bytes=1048576-"), Some((1048576, None)));
        assert_eq!(parse_range("bytes=-500"), None); // suffix range not supported
        assert_eq!(parse_range("invalid"), None);
    }
}
