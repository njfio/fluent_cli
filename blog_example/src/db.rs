use sqlx::{sqlite::SqlitePool, Result};
use crate::models::{CreatePost, Post, UpdatePost};

pub async fn init_db() -> Result<SqlitePool> {
    let pool = SqlitePool::connect("sqlite:blog.db").await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS posts (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            content TEXT NOT NULL,
            author TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        )
        "#,
    )
    .execute(&pool)
    .await?;

    Ok(pool)
}

pub async fn get_all_posts(pool: &SqlitePool) -> Result<Vec<Post>> {
    sqlx::query_as::<_, Post>("SELECT * FROM posts ORDER BY created_at DESC")
        .fetch_all(pool)
        .await
}

pub async fn get_post(pool: &SqlitePool, id: i64) -> Result<Option<Post>> {
    sqlx::query_as::<_, Post>("SELECT * FROM posts WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
}

pub async fn create_post(pool: &SqlitePool, post: CreatePost) -> Result<i64> {
    let result = sqlx::query(
        "INSERT INTO posts (title, content, author) VALUES (?, ?, ?)"
    )
    .bind(&post.title)
    .bind(&post.content)
    .bind(&post.author)
    .execute(pool)
    .await?;

    Ok(result.last_insert_rowid())
}

pub async fn update_post(pool: &SqlitePool, id: i64, post: UpdatePost) -> Result<bool> {
    let existing = get_post(pool, id).await?;

    if existing.is_none() {
        return Ok(false);
    }

    let existing = existing.unwrap();
    let title = post.title.unwrap_or(existing.title);
    let content = post.content.unwrap_or(existing.content);

    sqlx::query(
        "UPDATE posts SET title = ?, content = ?, updated_at = datetime('now') WHERE id = ?"
    )
    .bind(title)
    .bind(content)
    .bind(id)
    .execute(pool)
    .await?;

    Ok(true)
}

pub async fn delete_post(pool: &SqlitePool, id: i64) -> Result<bool> {
    let result = sqlx::query("DELETE FROM posts WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;

    Ok(result.rows_affected() > 0)
}
