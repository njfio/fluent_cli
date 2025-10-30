use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Post {
    pub id: i64,
    pub title: String,
    pub content: String,
    pub author: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreatePost {
    pub title: String,
    pub content: String,
    pub author: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePost {
    pub title: Option<String>,
    pub content: Option<String>,
}
