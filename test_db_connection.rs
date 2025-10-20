#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing SurrealDB connection...");
    
    // Try connection to port 8009 first
    println!("Trying port 8009...");
    let client = surrealdb::Surreal::new::<surrealdb::engine::any::Any>(("ws://127.0.0.1:8009", surrealdb::opt::Config::default())).await?;
    
    // Try signing in
    println!("Attempting sign in...");
    match client.signin(surrealdb::opt::Root {
        username: "root",
        password: "root",
    }).await {
        Ok(_) => println!("✅ Sign in successful on port 8009"),
        Err(e) => println!("❌ Sign in failed on port 8009: {}", e),
    }
    
    // Try port 8010
    println!("Trying port 8010...");
    let client2 = surrealdb::Surreal::new::<surrealdb::engine::any::Any>(("ws://127.0.0.1:8010", surrealdb::opt::Config::default())).await?;
    
    println!("Attempting sign in on port 8010...");
    match client2.signin(surrealdb::opt::Root {
        username: "root",
        password: "root",
    }).await {
        Ok(_) => println!("✅ Sign in successful on port 8010"),
        Err(e) => println!("❌ Sign in failed on port 8010: {}", e),
    }
    
    Ok(())
}
