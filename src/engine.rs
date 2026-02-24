use crate::compiler::{CompileError, CompileOptions, Compiler};
use crate::generator::Generator;
use crate::loader::{LoadError, Loader};
use crate::parser::{ParseError, Parser};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct RenderResult {
    pub content: String,
    pub status_code: u16,
    pub redirect: Option<String>,
    pub cookies: Vec<(String, String, i64)>,
    pub headers: Vec<(String, String)>,
}

impl Default for RenderResult {
    fn default() -> Self {
        Self {
            content: String::new(),
            status_code: 200,
            redirect: None,
            cookies: Vec::new(),
            headers: Vec::new(),
        }
    }
}

pub struct RspEngine {
    parser: Parser,
    generator: Generator,
    compiler: Compiler,
    loader: std::sync::Mutex<Loader>,
    cache_dir: PathBuf,
    docroot: std::sync::Mutex<PathBuf>,
}

#[derive(Debug)]
pub enum RspError {
    Parse(ParseError),
    Compile(CompileError),
    Load(LoadError),
    Io(std::io::Error),
}

impl std::fmt::Display for RspError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RspError::Parse(e) => write!(f, "Parse error: {}", e),
            RspError::Compile(e) => write!(f, "{}", e),
            RspError::Load(e) => write!(f, "Load error: {}", e),
            RspError::Io(e) => write!(f, "IO error: {}", e),
        }
    }
}

impl std::error::Error for RspError {}

impl From<ParseError> for RspError {
    fn from(e: ParseError) -> Self {
        RspError::Parse(e)
    }
}

impl From<CompileError> for RspError {
    fn from(e: CompileError) -> Self {
        RspError::Compile(e)
    }
}

impl From<LoadError> for RspError {
    fn from(e: LoadError) -> Self {
        RspError::Load(e)
    }
}

impl From<std::io::Error> for RspError {
    fn from(e: std::io::Error) -> Self {
        RspError::Io(e)
    }
}

impl RspEngine {
    pub fn new(cache_dir: PathBuf) -> Result<Self, RspError> {
        std::fs::create_dir_all(&cache_dir)?;

        Ok(RspEngine {
            parser: Parser::new(),
            generator: Generator::new(),
            compiler: Compiler::new(cache_dir.clone()),
            loader: std::sync::Mutex::new(Loader::new()),
            cache_dir,
            docroot: std::sync::Mutex::new(PathBuf::from(".")),
        })
    }

    pub fn set_docroot(&self, path: PathBuf) {
        if let Ok(mut d) = self.docroot.lock() {
            *d = path;
        }
    }

    pub fn render(&self, rsp_content: &str) -> Result<RenderResult, RspError> {
        let parsed = self.parser.parse(rsp_content)?;
        let generated = self.generator.generate_full_source(&parsed);

        let mut hasher = Sha256::new();
        hasher.update(rsp_content.as_bytes());
        let hash = format!("{:x}", hasher.finalize());

        let lib_path = if generated.needs_cargo {
            let options = CompileOptions {
                dependencies: generated.dependencies,
            };
            self.compiler
                .compile_with_options(&generated.source, &hash, options)?
        } else {
            self.compiler.compile(&generated.source, &hash)?
        };

        let mut loader = self.loader.lock().unwrap();
        let (content, status_code, redirect, cookies, headers) =
            loader.render_with_response(&lib_path)?;

        Ok(RenderResult {
            content,
            status_code,
            redirect,
            cookies,
            headers,
        })
    }

    pub fn render_file(&self, path: &Path) -> Result<RenderResult, RspError> {
        let content = std::fs::read_to_string(path)?;
        self.render(&content)
    }

    pub fn render_file_with_body(&self, path: &Path, body: &str) -> Result<RenderResult, RspError> {
        std::env::set_var("RSP_BODY", body);
        self.render_file(path)
    }

    pub fn include(&self, relative_path: &str) -> Result<String, RspError> {
        let docroot = self.docroot.lock().unwrap().clone();
        let full_path = docroot.join(relative_path);

        if !full_path.exists() {
            return Ok(format!(
                "<!-- Include error: {} not found -->",
                relative_path
            ));
        }

        let result = self.render_file(&full_path)?;
        Ok(result.content)
    }

    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }

    pub fn unload_all(&self) {
        if let Ok(mut loader) = self.loader.lock() {
            loader.unload_all();
        }
    }
}

static CLEANUP_REGISTERED: AtomicBool = AtomicBool::new(false);
static mut ENGINE_TO_CLEANUP: Option<Arc<RspEngine>> = None;

pub fn register_cleanup(engine: Arc<RspEngine>) {
    if CLEANUP_REGISTERED
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_ok()
    {
        unsafe {
            ENGINE_TO_CLEANUP = Some(engine.clone());
        }

        ctrlc::set_handler(move || {
            unsafe {
                if let Some(ref engine) = ENGINE_TO_CLEANUP {
                    let _ = engine.loader.lock().map(|mut l| l.unload_all());
                }
            }
            std::process::exit(0);
        })
        .ok();
    }
}
