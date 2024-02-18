use std::env;

use refinery::config::{Config, ConfigDbType};

mod embedded {
    use refinery::embed_migrations;
    embed_migrations!("migrations");
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();
    let db_host = env::var("DB_HOST")?;
    let db_port = env::var("DB_PORT")?;
    let db_user = env::var("DB_USER")?;
    let db_pass = env::var("DB_PASS")?;
    let db_name = env::var("DB_NAME")?;

    let mut conn = Config::new(ConfigDbType::Postgres)
        .set_db_user(&db_user)
        .set_db_pass(&db_pass)
        .set_db_host(&db_host)
        .set_db_port(&db_port)
        .set_db_name(&db_name);
    println!("Running migrations");

    embedded::migrations::runner().run(&mut conn)?;
    Ok(())
}
