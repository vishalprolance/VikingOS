// VikingOS Kernel — Hypertext Transfer Protocol (HTTP) Client & Parser
//
// Formats HTTP/1.1 GET/POST requests and parses web server responses.

#![allow(dead_code)]

use alloc::string::String;
use alloc::vec::Vec;

#[derive(Clone, Debug)]
pub struct HttpResponse {
    pub status_code: u16,
    pub status_text: String,
    pub content_type: String,
    pub content_length: usize,
    pub body: String,
}

pub struct HttpClient;

impl HttpClient {
    /// Format an HTTP/1.1 GET request string
    pub fn format_get_request(host: &str, path: &str) -> String {
        alloc::format!(
            "GET {} HTTP/1.1\r\n\
             Host: {}\r\n\
             User-Agent: VikingBrave/1.0 (VikingOS; x86_64; Bare-Metal)\r\n\
             Accept: text/html,application/xhtml+xml,application/json,*/*\r\n\
             Connection: close\r\n\
             \r\n",
            path, host
        )
    }

    /// Parse raw HTTP response bytes into an HttpResponse structure
    pub fn parse_response(data: &[u8]) -> Option<HttpResponse> {
        let text = core::str::from_utf8(data).ok()?;
        let mut lines = text.split("\r\n");

        // Parse status line: HTTP/1.1 200 OK
        let status_line = lines.next()?;
        let mut status_parts = status_line.split_whitespace();
        let _proto = status_parts.next()?;
        let code_str = status_parts.next()?;
        let status_code: u16 = code_str.parse().ok()?;
        let status_text = status_parts.collect::<Vec<&str>>().join(" ");

        let mut content_type = String::from("text/html");
        let mut content_length = 0;

        // Parse headers until empty line
        for line in &mut lines {
            if line.is_empty() {
                break;
            }
            if let Some((k, v)) = line.split_once(':') {
                let key = k.trim().to_lowercase();
                let val = v.trim();
                if key == "content-type" {
                    content_type = String::from(val);
                } else if key == "content-length" {
                    content_length = val.parse().unwrap_or(0);
                }
            }
        }

        // Remaining text is the body
        let body_parts: Vec<&str> = lines.collect();
        let body = body_parts.join("\r\n");

        Some(HttpResponse {
            status_code,
            status_text,
            content_type,
            content_length,
            body,
        })
    }
}
