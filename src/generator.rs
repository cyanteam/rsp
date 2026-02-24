use crate::parser::{ParsedTemplate, Token};

#[derive(Debug, Clone, Default)]
pub struct GeneratedCode {
    pub source: String,
    pub needs_cargo: bool,
    pub dependencies: Vec<String>,
}

pub struct Generator;

impl Generator {
    pub fn new() -> Self {
        Generator
    }

    pub fn generate_full_source(&self, parsed: &ParsedTemplate) -> GeneratedCode {
        let mut imports = String::new();
        let mut static_code = String::new();
        let mut render_code = String::new();
        let mut needs_cargo = false;
        let mut dependencies = Vec::new();
        let mut has_lazy = false;
        let mut has_request = false;
        let mut has_escape_html = false;
        let mut has_response_control = false;

        for dec in &parsed.declarations {
            if dec.contains("Lazy<") || dec.contains("once_cell") {
                has_lazy = true;
            }
            if dec.contains("escape_html") {
                has_escape_html = true;
            }
            static_code.push_str(&format!("{}\n", dec));
        }

        for token in &parsed.tokens {
            match token {
                Token::Text(text) => {
                    let escaped = escape_string(text);
                    render_code.push_str(&format!("    output.push_str(\"{}\");\n", escaped));
                }
                Token::Expression(expr) => {
                    if expr.contains("req()") || expr.contains("req.") {
                        has_request = true;
                    }
                    if expr.contains("escape_html") {
                        has_escape_html = true;
                    }
                    if expr.contains("header(")
                        || expr.contains("header_url(")
                        || expr.contains("SetCookie(")
                        || expr.contains("CleanCookie(")
                    {
                        has_response_control = true;
                    }
                    render_code.push_str(&format!(
                        "    output.push_str(&format!(\"{{}}\", {}));\n",
                        expr
                    ));
                }
                Token::Code(code_block) => {
                    if code_block.contains("req()") || code_block.contains("req.") {
                        has_request = true;
                    }
                    if code_block.contains("escape_html") {
                        has_escape_html = true;
                    }
                    if code_block.contains("header(")
                        || code_block.contains("header_url(")
                        || code_block.contains("SetCookie(")
                        || code_block.contains("CleanCookie(")
                    {
                        has_response_control = true;
                    }
                    for line in code_block.lines() {
                        render_code.push_str(&format!("    {}\n", line));
                    }
                }
                Token::Directive(directive) => {
                    let directive = directive.trim();

                    if directive.starts_with("use ") {
                        let use_stmt = directive.trim_start_matches("use ").trim();
                        if !use_stmt.ends_with(';') {
                            imports.push_str(&format!("use {};\n", use_stmt));
                        } else {
                            imports.push_str(&format!("{}\n", directive));
                        }
                    } else if directive.starts_with("dep ") {
                        needs_cargo = true;
                        let dep = directive.trim_start_matches("dep ").trim();
                        dependencies.push(dep.to_string());
                    } else if directive.starts_with("once_cell") {
                        has_lazy = true;
                        needs_cargo = true;
                        dependencies.push("once_cell = \"1\"".to_string());
                    } else if directive.starts_with("rusqlite") {
                        needs_cargo = true;
                        dependencies.push(
                            "rusqlite = { version = \"0.32\", features = [\"bundled\"] }"
                                .to_string(),
                        );
                    }
                }
                Token::Declaration(_) => {}
            }
        }

        if has_request || has_response_control {
            imports.insert_str(
                0,
                "use rsp_runtime::{Request, Params, Cookies, Headers, escape_html};\n",
            );
            needs_cargo = true;
        } else if has_escape_html {
            imports.insert_str(0, "use rsp_runtime::escape_html;\n");
            needs_cargo = true;
        }

        if has_lazy && !imports.contains("use once_cell") {
            imports.insert_str(0, "use once_cell::sync::Lazy;\n");
        }

        let request_init = if has_request || has_response_control {
            "    let req = Request::new();\n    let _ = &req;\n"
        } else {
            ""
        };

        let source = format!(
            r#"#![allow(unused)]
use std::os::raw::c_char;
use std::ffi::CString;
use std::cell::RefCell;

thread_local! {{
    static STATUS_CODE: RefCell<u16> = RefCell::new(200);
    static REDIRECT: RefCell<Option<String>> = RefCell::new(None);
    static COOKIES: RefCell<Vec<(String, String, i64)>> = RefCell::new(Vec::new());
    static HEADERS: RefCell<Vec<(String, String)>> = RefCell::new(Vec::new());
}}

fn header(code: u16) {{
    STATUS_CODE.with(|c| *c.borrow_mut() = code);
}}

fn header_url(url: &str) {{
    REDIRECT.with(|r| *r.borrow_mut() = Some(url.to_string()));
    STATUS_CODE.with(|c| *c.borrow_mut() = 302);
}}

fn SetCookie(name: &str, value: &str, max_age: i64) {{
    COOKIES.with(|c| c.borrow_mut().push((name.to_string(), value.to_string(), max_age)));
}}

fn CleanCookie(name: &str) {{
    COOKIES.with(|c| c.borrow_mut().push((name.to_string(), "".to_string(), -1)));
}}

{}
{}

#[no_mangle]
pub extern "C" fn render() -> *mut c_char {{
    let mut output = String::new();
{}    {}
    let c_string = CString::new(output).unwrap();
    c_string.into_raw()
}}

#[no_mangle]
pub extern "C" fn free_string(s: *mut c_char) {{
    if s.is_null() {{ return; }}
    unsafe {{ drop(CString::from_raw(s)); }}
}}

#[no_mangle]
pub extern "C" fn get_status_code() -> u16 {{
    STATUS_CODE.with(|c| *c.borrow())
}}

#[no_mangle]
pub extern "C" fn get_redirect() -> *mut c_char {{
    let redirect = REDIRECT.with(|r| r.borrow().clone());
    match redirect {{
        Some(url) => {{
            let c_string = CString::new(url).unwrap();
            c_string.into_raw()
        }}
        None => std::ptr::null_mut(),
    }}
}}

#[no_mangle]
pub extern "C" fn get_cookies() -> *mut c_char {{
    let cookies: String = COOKIES.with(|c| {{
        c.borrow().iter()
            .map(|(name, value, max_age)| format!("{{}}\t{{}}\t{{}}", name, value, max_age))
            .collect::<Vec<_>>()
            .join("\n")
    }});
    let c_string = CString::new(cookies).unwrap();
    c_string.into_raw()
}}

#[no_mangle]
pub extern "C" fn get_headers() -> *mut c_char {{
    let headers: String = HEADERS.with(|h| {{
        h.borrow().iter()
            .map(|(name, value)| format!("{{}}:{{}}", name, value))
            .collect::<Vec<_>>()
            .join("\n")
    }});
    let c_string = CString::new(headers).unwrap();
    c_string.into_raw()
}}
"#,
            imports, static_code, request_init, render_code
        );

        GeneratedCode {
            source,
            needs_cargo,
            dependencies,
        }
    }
}

impl Default for Generator {
    fn default() -> Self {
        Self::new()
    }
}

fn escape_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}
