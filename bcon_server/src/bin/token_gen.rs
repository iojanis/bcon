use anyhow::Result;
use bcon_server::{AuthService, ClientRole};
use clap::{Arg, Command};

fn main() -> Result<()> {
    let matches = Command::new("token_gen")
        .version("1.0.0")
        .about("Generate JWT tokens for Bcon server")
        .arg(Arg::new("type")
            .short('t')
            .long("type")
            .value_name("TYPE")
            .help("Token type: adapter or client")
            .required(true))
        .arg(Arg::new("adapter-secret")
            .long("adapter-secret")
            .value_name("SECRET")
            .help("Adapter secret (32+ chars)")
            .default_value("adapter_test_secret_32_chars_minimum_length_required_here"))
        .arg(Arg::new("client-secret")
            .long("client-secret")
            .value_name("SECRET")
            .help("Client secret (32+ chars)")
            .default_value("client_test_secret_32_chars_minimum_length_required_here"))
        .arg(Arg::new("server-id")
            .long("server-id")
            .value_name("ID")
            .help("Server ID (for adapter tokens)")
            .default_value("test_server"))
        .arg(Arg::new("server-name")
            .long("server-name")
            .value_name("NAME")
            .help("Server name (for adapter tokens)")
            .default_value("Test Server"))
        .arg(Arg::new("user-id")
            .long("user-id")
            .value_name("ID")
            .help("User ID (for client tokens)"))
        .arg(Arg::new("username")
            .long("username")
            .value_name("NAME")
            .help("Username (for client tokens)")
            .default_value("TestUser"))
        .arg(Arg::new("role")
            .long("role")
            .value_name("ROLE")
            .help("User role: guest, player, admin, system")
            .default_value("player"))
        .arg(Arg::new("expires-days")
            .long("expires-days")
            .value_name("DAYS")
            .help("Token expiration in days")
            .default_value("30"))
        .get_matches();

    let token_type = matches.get_one::<String>("type").unwrap();
    let adapter_secret = matches.get_one::<String>("adapter-secret").unwrap().to_string();
    let client_secret = matches.get_one::<String>("client-secret").unwrap().to_string();
    let expires_days: i64 = matches.get_one::<String>("expires-days")
        .unwrap()
        .parse()
        .unwrap_or(30);

    match token_type.as_str() {
        "adapter" => {
            let auth_service = AuthService::new(adapter_secret, client_secret)?;
            let server_id = matches.get_one::<String>("server-id").unwrap().to_string();
            let server_name = matches.get_one::<String>("server-name").map(|s| s.to_string());
            
            let token = auth_service.create_adapter_token(server_id.clone(), server_name, expires_days)?;
            
            println!("Adapter Token Generated:");
            println!("Server ID: {}", server_id);
            println!("Expires: {} days", expires_days);
            println!("Token: {}", token);
            println!();
            println!("Use this token in your Minecraft server plugin/mod:");
            println!("Authorization: Bearer {}", token);
        }
        "client" => {
            let auth_service = AuthService::new(adapter_secret, client_secret)?;
            let user_id = matches.get_one::<String>("user-id").map(|s| s.to_string());
            let username = matches.get_one::<String>("username").map(|s| s.to_string());
            let role_str = matches.get_one::<String>("role").unwrap();
            let role = ClientRole::from_str(role_str);
            
            // Convert days to hours for client tokens
            let expires_hours = expires_days * 24;
            let token = auth_service.create_client_token(user_id, username.clone(), role, expires_hours)?;
            
            println!("Client Token Generated:");
            println!("Username: {}", username.unwrap_or_else(|| "None".to_string()));
            println!("Role: {}", role_str);
            println!("Expires: {} hours ({} days)", expires_hours, expires_days);
            println!("Token: {}", token);
            println!();
            println!("Use this token in your web client:");
            println!("{{ \"eventType\": \"auth\", \"data\": {{ \"token\": \"{}\" }} }}", token);
        }
        _ => {
            eprintln!("Invalid token type. Use 'adapter' or 'client'.");
            std::process::exit(1);
        }
    }

    Ok(())
}