use std::path::{Path, PathBuf};
use std::process::Command;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CompileError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Compilation failed:\n{0}")]
    Compile(String),
}

#[derive(Debug, Clone, Default)]
pub struct CompileOptions {
    pub dependencies: Vec<String>,
}

pub struct Compiler {
    cache_dir: PathBuf,
    global_target: PathBuf,
}

impl Compiler {
    pub fn new(cache_dir: PathBuf) -> Self {
        let global_target = std::env::var("RSP_TARGET_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                std::env::var("HOME")
                    .map(|h| PathBuf::from(h).join(".rsp").join("target"))
                    .unwrap_or_else(|_| cache_dir.join("target"))
            });

        Compiler {
            cache_dir,
            global_target,
        }
    }

    pub fn compile(&self, source: &str, hash: &str) -> Result<PathBuf, CompileError> {
        std::fs::create_dir_all(&self.cache_dir)?;

        let output_path = self.get_lib_path(hash);

        if output_path.exists() {
            return Ok(output_path);
        }

        let source_path = self.cache_dir.join(format!("{}.rs", hash));
        std::fs::write(&source_path, source)?;

        let mut cmd = Command::new("rustc");
        cmd.arg(&source_path)
            .arg("--crate-type=cdylib")
            .arg("-o")
            .arg(&output_path)
            .arg("-C")
            .arg("opt-level=2")
            .arg("-C")
            .arg("debuginfo=0");

        let output = cmd.output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(CompileError::Compile(stderr.to_string()));
        }

        let _ = std::fs::remove_file(&source_path);

        Ok(output_path)
    }

    pub fn compile_with_options(
        &self,
        source: &str,
        hash: &str,
        options: CompileOptions,
    ) -> Result<PathBuf, CompileError> {
        let output_path = self.get_lib_path(hash);

        if output_path.exists() {
            return Ok(output_path);
        }

        std::fs::create_dir_all(&self.global_target)?;

        let project_dir = self.cache_dir.join("cargo").join(hash);
        std::fs::create_dir_all(project_dir.join("src"))?;

        let cargo_toml = self.generate_cargo_toml(hash, &options);
        std::fs::write(project_dir.join("Cargo.toml"), cargo_toml)?;

        std::fs::write(project_dir.join("src/lib.rs"), source)?;

        let mut cmd = Command::new("cargo");
        cmd.arg("build")
            .arg("--release")
            .current_dir(&project_dir)
            .env("CARGO_TARGET_DIR", &self.global_target);

        let output = cmd.output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(CompileError::Compile(stderr.to_string()));
        }

        let pkg_name = format!("rsp_{}", hash);
        let built_lib = self.global_target.join("release").join({
            #[cfg(target_os = "linux")]
            {
                format!("lib{}.so", pkg_name)
            }
            #[cfg(target_os = "macos")]
            {
                format!("lib{}.dylib", pkg_name)
            }
            #[cfg(target_os = "windows")]
            {
                format!("{}.dll", pkg_name)
            }
        });

        if built_lib.exists() {
            std::fs::copy(&built_lib, &output_path)?;
        }

        Ok(output_path)
    }

    fn generate_cargo_toml(&self, name: &str, options: &CompileOptions) -> String {
        let runtime_path = std::env::var("RSP_RUNTIME_PATH").unwrap_or_else(|_| {
            let exe_path = std::env::current_exe().unwrap_or_default();
            let exe_dir = exe_path.parent().unwrap_or(std::path::Path::new("."));
            exe_dir.join("runtime").to_string_lossy().to_string()
        });

        let mut deps: Vec<String> = options.dependencies.clone();

        let has_runtime = deps.iter().any(|d| d.contains("rsp-runtime"));
        if !has_runtime {
            deps.push(format!("rsp-runtime = {{ path = \"{}\" }}", runtime_path));
        }

        let deps_str = deps.join("\n");

        let pkg_name = format!("rsp_{}", name);

        format!(
            r#"[package]
name = "{}"
version = "0.0.1"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
{}

[profile.release]
opt-level = 2
lto = false
codegen-units = 16

[workspace]
"#,
            pkg_name, deps_str
        )
    }

    fn get_lib_path(&self, name: &str) -> PathBuf {
        #[cfg(target_os = "linux")]
        let lib_name = format!("lib{}.so", name);

        #[cfg(target_os = "macos")]
        let lib_name = format!("lib{}.dylib", name);

        #[cfg(target_os = "windows")]
        let lib_name = format!("{}.dll", name);

        self.cache_dir.join(lib_name)
    }

    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }
}
