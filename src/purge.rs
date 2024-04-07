use dotenv::dotenv;

mod db;

#[tokio::main]
async fn main() {
    dotenv().ok();
    let pool = db::connect_db().await;
    let _ = sqlx::query!("DELETE FROM messages").execute(&pool).await;
    let _ = sqlx::query!("DELETE FROM chat_user").execute(&pool).await;
    let _ = sqlx::query!("DELETE FROM chats").execute(&pool).await;
}
