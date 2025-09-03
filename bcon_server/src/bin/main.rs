use anyhow::Result;
use bcon_server::{BconServer, BconConfig};
use tracing::{info, error};

#[cfg(not(target_arch = "wasm32"))]
#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    info!("Starting Bcon WebSocket Communication Server");

    // Parse CLI arguments and load configuration
    let config = match bcon_server::config::native_config::parse_cli_args()? {
        Some(config) => config,
        None => {
            // Config was generated, exit
            return Ok(());
        }
    };

    // Print configuration summary
    config.print_summary();

    // Create and start the server
    let server = BconServer::new(config)?;
    
    info!("Bcon server initialized successfully");
    
    // Start the server (this blocks until shutdown)
    match server.start().await {
        Ok(()) => {
            info!("Bcon server shutdown gracefully");
            Ok(())
        }
        Err(e) => {
            error!("Bcon server error: {}", e);
            Err(e)
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn main() {
    console_error_panic_hook::set_once();
    wasm_logger::init(wasm_logger::Config::default());
    
    wasm_bindgen_futures::spawn_local(async {
        if let Err(e) = run_wasm().await {
            web_sys::console::error_1(&format!("WASM error: {}", e).into());
        }
    });
}

#[cfg(target_arch = "wasm32")]
async fn run_wasm() -> Result<()> {
    use wasm_bindgen::prelude::*;
    
    // WASM version would be initialized differently
    // This is just a placeholder showing the structure
    web_sys::console::log_1(&"Bcon WASM module loaded".into());
    Ok(())
}