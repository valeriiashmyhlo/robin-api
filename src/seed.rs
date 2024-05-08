use dotenv::dotenv;

mod db;

async fn seed_db(pool: &sqlx::PgPool) -> anyhow::Result<()> {
    sqlx::query!("INSERT INTO users (id, username, token, password) VALUES ('cc36a1f5-eb49-4552-b159-ce3040c519e0', 'Test1', 'ab36a1f5-eb49-4552-b159-ce3040c519e1', 'pass')").execute(pool).await?;
    sqlx::query!("INSERT INTO users (id, username, token, password) VALUES ('ac36a1f5-eb49-4552-b159-ce3040c519e0', 'Test2', 'cb36a1f5-eb49-4552-b159-ce3040c519e1', 'pass')").execute(pool).await?;
    sqlx::query!("INSERT INTO chats (id) VALUES ('d58535ec-fe54-4d30-9808-94af7d6dc1bf')")
        .execute(pool)
        .await?;
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();
    let pool = db::connect_db().await;
    seed_db(&pool).await?;
    Ok(())
}
