use axum::{
    body::Body,
    extract::Request as AxumRequest,
    http::{header, Response, StatusCode},
    response::IntoResponse,
    Router,
};
use clap::Parser;
use rsp::engine::RenderResult;
use rsp::RspEngine;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tower::ServiceExt;
use tower_http::services::ServeDir;
use walkdir::WalkDir;

#[derive(Parser)]
#[command(name = "rsp")]
#[command(version = "0.1.0")]
#[command(about = "Rust Server Pages - A PHP-like template engine for Rust", long_about = None)]
struct Cli {
    #[arg(value_name = "FILE")]
    file: Option<PathBuf>,

    #[arg(short = 'S', long = "server", value_name = "ADDR:PORT")]
    server: Option<String>,

    #[arg(short = 't', long = "docroot", value_name = "DIR", default_value = ".")]
    docroot: PathBuf,

    #[arg(short = 'i', long = "index", value_name = "FILE", default_value = "index.rsp")]
    index: String,

    #[arg(long = "precompile")]
    precompile: bool,

    #[arg(long = "cache-dir", value_name = "DIR")]
    cache_dir: Option<PathBuf>,
}

fn main() {
    let cli = Cli::parse();

    let docroot = cli.docroot.canonicalize().unwrap_or_else(|_| cli.docroot.clone());
    let cache_dir = cli.cache_dir.clone().unwrap_or_else(|| docroot.join(".rspcache"));

    let engine = Arc::new(
        RspEngine::new(cache_dir.clone()).expect("Failed to initialize engine"),
    );
    
    engine.set_docroot(docroot.clone());

    rsp::engine::register_cleanup(engine.clone());

    let runtime = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");

    if let Some(addr) = cli.server {
        runtime.block_on(run_server(engine.clone(), &docroot, &addr, &cli.index));
    } else if let Some(file) = cli.file {
        run_file(&engine, &file);
    } else {
        if cli.precompile {
            precompile_all(&engine, &docroot);
        } else {
            print_usage();
        }
    }

    engine.unload_all();
}

fn run_file(engine: &Arc<RspEngine>, file: &PathBuf) {
    match engine.render_file(file) {
        Ok(result) => {
            if let Some(redirect) = &result.redirect {
                println!("Redirect: {}", redirect);
            }
            println!("{}", result.content);
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

async fn run_server(engine: Arc<RspEngine>, docroot: &Path, addr: &str, index: &str) {
    let addr: SocketAddr = addr.parse().unwrap_or_else(|_| {
        eprintln!("Invalid address format, using 127.0.0.1:8080");
        "127.0.0.1:8080".parse().unwrap()
    });

    println!("RSP development server started");
    println!("Document root: {}", docroot.display());
    println!("Index file: {}", index);
    println!("Listening on http://{}", addr);
    println!("Press Ctrl+C to stop");

    let docroot_clone = docroot.to_path_buf();
    let serve_dir = ServeDir::new(docroot_clone.clone());
    let index_clone = index.to_string();

    let app = Router::new()
        .fallback(move |req| {
            let engine = engine.clone();
            let docroot = docroot_clone.clone();
            let serve_dir = serve_dir.clone();
            let index = index_clone.clone();
            async move {
                handle_request(req, engine, docroot, serve_dir, &index).await
            }
        });

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    if let Err(e) = axum::serve(listener, app).await {
        eprintln!("Server error: {}", e);
    }
}

async fn handle_request(
    axum_req: AxumRequest,
    engine: Arc<RspEngine>,
    docroot: PathBuf,
    serve_dir: ServeDir,
    default_index: &str,
) -> impl IntoResponse {
    let uri = axum_req.uri().clone();
    let method = axum_req.method().to_string();
    let mut path = uri.path().trim_start_matches('/').to_string();
    let query = uri.query().unwrap_or("").to_string();
    
    // Extract HTTP headers
    let headers = axum_req.headers().clone();
    
    let body_bytes = axum::body::to_bytes(axum_req.into_body(), 1024 * 1024 * 10).await;
    let body = body_bytes.map(|b| String::from_utf8_lossy(&b).to_string()).unwrap_or_default();
    
    // Handle directory request - redirect to index
    if path.is_empty() || path.ends_with('/') {
        path = if path.is_empty() {
            default_index.to_string()
        } else {
            format!("{}{}", path, default_index)
        };
    }
    
    // Check if it's an RSP file
    if path.ends_with(".rsp") {
        let file_path = docroot.join(&path);
        
        if file_path.exists() {
            // Set up environment variables for request
            std::env::set_var("REQUEST_METHOD", &method);
            std::env::set_var("QUERY_STRING", &query);
            std::env::set_var("REQUEST_URI", &path);
            std::env::set_var("RSP_BODY", &body);
            
            // Set HTTP headers as environment variables (HTTP_* format)
            for (name, value) in headers.iter() {
                let env_key = format!("HTTP_{}", name.as_str().replace('-', "_").to_uppercase());
                if let Ok(v) = value.to_str() {
                    std::env::set_var(&env_key, v);
                }
            }
            
            match engine.render_file_with_body(&file_path, &body) {
                Ok(result) => {
                    return build_response(result);
                }
                Err(e) => {
                    return Response::builder()
                        .status(StatusCode::INTERNAL_SERVER_ERROR)
                        .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
                        .body(Body::from(format!("Error: {}", e)))
                        .unwrap();
                }
            }
        }
    }
    
    // Serve static files
    let req = AxumRequest::builder()
        .method(method.as_str())
        .uri(uri)
        .body(Body::from(body))
        .unwrap();
    
    match serve_dir.oneshot(req).await {
        Ok(res) => res.map(Body::new),
        Err(_) => Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Body::from("Internal Server Error"))
            .unwrap(),
    }
}

fn build_response(result: RenderResult) -> Response<Body> {
    let status = StatusCode::from_u16(result.status_code).unwrap_or(StatusCode::OK);
    
    let mut builder = Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "text/html; charset=utf-8");
    
    // Handle redirect
    if let Some(redirect) = &result.redirect {
        builder = builder.header(header::LOCATION, redirect);
    }
    
    // Set cookies
    for (name, value, max_age) in &result.cookies {
        let cookie_str = if *max_age < 0 {
            format!("{}=; Path=/; Max-Age=0; HttpOnly", name)
        } else {
            format!("{}={}; Path=/; Max-Age={}{}; HttpOnly", 
                name, value, max_age,
                if *max_age > 0 { "" } else { "" }
            )
        };
        builder = builder.header(header::SET_COOKIE, cookie_str);
    }
    
    // Set custom headers
    for (name, value) in &result.headers {
        builder = builder.header(name.as_str(), value.as_str());
    }
    
    builder.body(Body::from(result.content)).unwrap()
}

fn precompile_all(engine: &Arc<RspEngine>, docroot: &Path) {
    println!("Precompiling all .rsp files in {}...", docroot.display());

    let mut count = 0;
    let mut errors = 0;

    for entry in WalkDir::new(docroot)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.extension().map_or(false, |ext| ext == "rsp") {
            print!("Compiling {}... ", path.display());
            match engine.render_file(path) {
                Ok(_) => {
                    println!("OK");
                    count += 1;
                }
                Err(e) => {
                    println!("FAILED");
                    eprintln!("  Error: {}", e);
                    errors += 1;
                }
            }
        }
    }

    println!();
    println!("Precompiled {} files, {} errors", count, errors);
}

fn print_usage() {
    println!(r#"RSP - Rust Server Pages

Usage:
  rsp <file.rsp>                  Run an rsp file
  rsp -S <addr:port> [options]    Start development server

Options:
  -S, --server <ADDR:PORT>        Start built-in web server
  -t, --docroot <DIR>             Document root directory (default: .)
  -i, --index <FILE>              Default index file (default: index.rsp)
      --precompile                Precompile all .rsp files
      --cache-dir <DIR>           Cache directory (default: .rspcache)

Examples:
  rsp hello.rsp                   Run hello.rsp and print output
  rsp -S 0.0.0.0:8080             Start server on port 8080
  rsp -S 127.0.0.1:3000 -t ./www  Serve from ./www directory
  rsp --precompile                Precompile all rsp files

Template syntax:
  <% code %>                      Execute Rust code
  <%= expression %>               Output expression value
  <%! static ... %>               Static declarations (run once)
  <%@ use ... %>                  Import module
  <%@ dep ... %>                  Add dependency
  <%@ once_cell %>                Enable lazy static initialization

Request API:
  req.get["key"]                  GET parameter (returns &str)
  req.post["key"]                 POST parameter
  req.cookie["key"]               Cookie value
  req.ua["user-agent"]            HTTP header
  req.method()                    Request method
  req.path()                      Request path
  req.is_post() / req.is_get()    Check method

Response API:
  header(302)                     Set status code (100-599)
  header_url("/login")            Redirect to URL (302)
  SetCookie("name", "value", 3600)  Set cookie (max_age in seconds)
  CleanCookie("name")             Delete cookie

Database:
  <%@ dep rusqlite = {{ version = "0.32", features = ["bundled"] }} %>
  <%@ use rusqlite::Connection %>
"#);
}
