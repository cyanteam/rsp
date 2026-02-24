use libloading::{Library, Symbol};
use std::collections::HashMap;
use std::ffi::CStr;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use thiserror::Error;

pub struct Loader {
    libraries: HashMap<PathBuf, LoadedLib>,
}

struct LoadedLib {
    library: Library,
    modified: SystemTime,
}

#[derive(Error, Debug)]
pub enum LoadError {
    #[error("Failed to load library: {0}")]
    Load(#[from] libloading::Error),
    #[error("Failed to get symbol: {0}")]
    Symbol(#[from] std::ffi::NulError),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

impl Loader {
    pub fn new() -> Self {
        Loader {
            libraries: HashMap::new(),
        }
    }

    pub fn render(&mut self, lib_path: &Path) -> Result<String, LoadError> {
        let (content, _, _, _, _) = self.render_with_response(lib_path)?;
        Ok(content)
    }

    pub fn render_with_response(
        &mut self,
        lib_path: &Path,
    ) -> Result<
        (
            String,
            u16,
            Option<String>,
            Vec<(String, String, i64)>,
            Vec<(String, String)>,
        ),
        LoadError,
    > {
        let modified = std::fs::metadata(lib_path)?.modified()?;

        let needs_reload = match self.libraries.get(lib_path) {
            Some(loaded) => loaded.modified != modified,
            None => true,
        };

        if needs_reload {
            if let Some(old) = self.libraries.remove(lib_path) {
                drop(old);
            }
            let library = unsafe { Library::new(lib_path) }?;
            self.libraries
                .insert(lib_path.to_path_buf(), LoadedLib { library, modified });
        }

        let loaded = self.libraries.get(lib_path).unwrap();

        let render_fn: Symbol<unsafe extern "C" fn() -> *mut std::os::raw::c_char> =
            unsafe { loaded.library.get(b"render") }?;

        let free_fn: Symbol<unsafe extern "C" fn(*mut std::os::raw::c_char)> =
            unsafe { loaded.library.get(b"free_string") }?;

        let get_status_fn: Symbol<unsafe extern "C" fn() -> u16> =
            unsafe { loaded.library.get(b"get_status_code") }?;

        let get_redirect_fn: Symbol<unsafe extern "C" fn() -> *mut std::os::raw::c_char> =
            unsafe { loaded.library.get(b"get_redirect") }?;

        let get_cookies_fn: Symbol<unsafe extern "C" fn() -> *mut std::os::raw::c_char> =
            unsafe { loaded.library.get(b"get_cookies") }?;

        let get_headers_fn: Symbol<unsafe extern "C" fn() -> *mut std::os::raw::c_char> =
            unsafe { loaded.library.get(b"get_headers") }?;

        let ptr = unsafe { render_fn() };
        let content = unsafe {
            let c_str = CStr::from_ptr(ptr);
            let s = c_str.to_string_lossy().into_owned();
            free_fn(ptr);
            s
        };

        let status_code = unsafe { get_status_fn() };

        let redirect = unsafe {
            let ptr = get_redirect_fn();
            if ptr.is_null() {
                None
            } else {
                let c_str = CStr::from_ptr(ptr);
                let s = c_str.to_string_lossy().into_owned();
                free_fn(ptr);
                if s.is_empty() {
                    None
                } else {
                    Some(s)
                }
            }
        };

        let cookies = unsafe {
            let ptr = get_cookies_fn();
            let s = if ptr.is_null() {
                String::new()
            } else {
                let c_str = CStr::from_ptr(ptr);
                let s = c_str.to_string_lossy().into_owned();
                free_fn(ptr);
                s
            };
            parse_cookies(&s)
        };

        let headers = unsafe {
            let ptr = get_headers_fn();
            let s = if ptr.is_null() {
                String::new()
            } else {
                let c_str = CStr::from_ptr(ptr);
                let s = c_str.to_string_lossy().into_owned();
                free_fn(ptr);
                s
            };
            parse_headers(&s)
        };

        Ok((content, status_code, redirect, cookies, headers))
    }

    pub fn unload_all(&mut self) {
        self.libraries.clear();
    }
}

fn parse_cookies(s: &str) -> Vec<(String, String, i64)> {
    if s.is_empty() {
        return Vec::new();
    }
    s.split('\n')
        .filter_map(|line| {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 3 {
                Some((
                    parts[0].to_string(),
                    parts[1].to_string(),
                    parts[2].parse().unwrap_or(0),
                ))
            } else {
                None
            }
        })
        .collect()
}

fn parse_headers(s: &str) -> Vec<(String, String)> {
    if s.is_empty() {
        return Vec::new();
    }
    s.split('\n')
        .filter_map(|line| {
            let parts: Vec<&str> = line.splitn(2, ':').collect();
            if parts.len() == 2 {
                Some((parts[0].to_string(), parts[1].to_string()))
            } else {
                None
            }
        })
        .collect()
}

impl Default for Loader {
    fn default() -> Self {
        Self::new()
    }
}
