use std::collections::HashMap;
use std::ops::Index;

#[derive(Debug, Clone, Default)]
pub struct Params(HashMap<String, String>);

impl Params {
    pub fn get(&self, key: &str) -> Option<&String> {
        self.0.get(key)
    }

    pub fn str(&self, key: &str) -> &str {
        self.0.get(key).map(|s| s.as_str()).unwrap_or("")
    }

    pub fn or(&self, key: &str, default: &str) -> String {
        self.0
            .get(key)
            .cloned()
            .unwrap_or_else(|| default.to_string())
    }
}

impl Index<&str> for Params {
    type Output = str;
    fn index(&self, key: &str) -> &Self::Output {
        self.0.get(key).map(|s| s.as_str()).unwrap_or("")
    }
}

#[derive(Debug, Clone, Default)]
pub struct Headers(HashMap<String, String>);

impl Headers {
    pub fn get(&self, key: &str) -> Option<&String> {
        self.0.get(&key.to_lowercase())
    }

    pub fn str(&self, key: &str) -> &str {
        self.0
            .get(&key.to_lowercase())
            .map(|s| s.as_str())
            .unwrap_or("")
    }

    pub fn or(&self, key: &str, default: &str) -> String {
        self.0
            .get(&key.to_lowercase())
            .cloned()
            .unwrap_or_else(|| default.to_string())
    }
}

impl Index<&str> for Headers {
    type Output = str;
    fn index(&self, key: &str) -> &Self::Output {
        self.0
            .get(&key.to_lowercase())
            .map(|s| s.as_str())
            .unwrap_or("")
    }
}

#[derive(Debug, Clone, Default)]
pub struct Cookies(HashMap<String, String>);

impl Cookies {
    pub fn get(&self, key: &str) -> Option<&String> {
        self.0.get(key)
    }

    pub fn str(&self, key: &str) -> &str {
        self.0.get(key).map(|s| s.as_str()).unwrap_or("")
    }

    pub fn or(&self, key: &str, default: &str) -> String {
        self.0
            .get(key)
            .cloned()
            .unwrap_or_else(|| default.to_string())
    }
}

impl Index<&str> for Cookies {
    type Output = str;
    fn index(&self, key: &str) -> &Self::Output {
        self.0.get(key).map(|s| s.as_str()).unwrap_or("")
    }
}

#[derive(Debug, Clone, Default)]
pub struct Request {
    pub get: Params,
    pub post: Params,
    pub cookie: Cookies,
    pub ua: Headers,
    method: String,
    path: String,
    body: String,
}

impl Request {
    pub fn new() -> Self {
        let get: HashMap<String, String> = std::env::var("QUERY_STRING")
            .unwrap_or_default()
            .split('&')
            .filter_map(|p| {
                let mut parts = p.splitn(2, '=');
                let key = parts.next()?.to_string();
                if key.is_empty() {
                    return None;
                }
                let value = urldecode(parts.next().unwrap_or(""));
                Some((key, value))
            })
            .collect();

        let body = std::env::var("RSP_BODY").unwrap_or_default();

        let post: HashMap<String, String> = body
            .split('&')
            .filter_map(|p| {
                let mut parts = p.splitn(2, '=');
                let key = parts.next()?.to_string();
                if key.is_empty() {
                    return None;
                }
                let value = urldecode(parts.next().unwrap_or(""));
                Some((key, value))
            })
            .collect();

        let cookie: HashMap<String, String> = std::env::var("HTTP_COOKIE")
            .unwrap_or_default()
            .split(';')
            .filter_map(|c| {
                let c = c.trim();
                let mut parts = c.splitn(2, '=');
                let key = parts.next()?.to_string();
                if key.is_empty() {
                    return None;
                }
                let value = urldecode(parts.next().unwrap_or(""));
                Some((key, value))
            })
            .collect();

        let mut headers = HashMap::new();
        for (key, value) in std::env::vars() {
            if key.starts_with("HTTP_") {
                let header_name = key[5..].replace('_', "-").to_lowercase();
                headers.insert(header_name, value);
            }
        }

        if let Ok(ct) = std::env::var("CONTENT_TYPE") {
            headers.insert("content-type".to_string(), ct);
        }
        if let Ok(cl) = std::env::var("CONTENT_LENGTH") {
            headers.insert("content-length".to_string(), cl);
        }

        Request {
            get: Params(get),
            post: Params(post),
            cookie: Cookies(cookie),
            ua: Headers(headers),
            method: std::env::var("REQUEST_METHOD").unwrap_or("GET".to_string()),
            path: std::env::var("REQUEST_URI").unwrap_or("/".to_string()),
            body,
        }
    }

    pub fn get_i64(&self, key: &str) -> Option<i64> {
        self.get.0.get(key).and_then(|v| v.parse().ok())
    }

    pub fn post_i64(&self, key: &str) -> Option<i64> {
        self.post.0.get(key).and_then(|v| v.parse().ok())
    }

    pub fn method(&self) -> &str {
        &self.method
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn is_post(&self) -> bool {
        self.method == "POST"
    }

    pub fn is_get(&self) -> bool {
        self.method == "GET"
    }

    pub fn body(&self) -> &str {
        &self.body
    }

    pub fn ip(&self) -> &str {
        self.ua
            .str("x-forwarded-for")
            .split(',')
            .next()
            .map(|s| s.trim())
            .unwrap_or_else(|| self.ua.str("x-real-ip"))
    }
}

fn urldecode(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '+' {
            result.push(' ');
        } else if c == '%' {
            let hex: String = chars.by_ref().take(2).collect();
            if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                result.push(byte as char);
            }
        } else {
            result.push(c);
        }
    }
    result
}

pub fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
