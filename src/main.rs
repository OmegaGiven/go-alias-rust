use ogdevdesk_service::server::{ServerConfig, run_server};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    run_server(ServerConfig::from_env()).await
}
