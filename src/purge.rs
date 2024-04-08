use dotenv::dotenv;

mod db;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();
    let pool = db::connect_db().await;
    sqlx::query!("DELETE FROM messages").execute(&pool).await?;
    sqlx::query!("DELETE FROM chat_user").execute(&pool).await?;
    sqlx::query!("DELETE FROM chats").execute(&pool).await?;

    Ok(())
}
