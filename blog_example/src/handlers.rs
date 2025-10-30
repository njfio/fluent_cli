use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
    Form,
};
use askama::Template;
use sqlx::SqlitePool;

use crate::db;
use crate::models::{CreatePost, Post, UpdatePost};

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    posts: Vec<Post>,
}

#[derive(Template)]
#[template(path = "post.html")]
struct PostTemplate {
    post: Post,
}

#[derive(Template)]
#[template(path = "new.html")]
struct NewPostTemplate;

#[derive(Template)]
#[template(path = "edit.html")]
struct EditPostTemplate {
    post: Post,
}

pub async fn index(State(pool): State<SqlitePool>) -> Response {
    match db::get_all_posts(&pool).await {
        Ok(posts) => {
            let template = IndexTemplate { posts };
            Html(template.render().unwrap()).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Database error: {}", e),
        )
            .into_response(),
    }
}

pub async fn show_post(State(pool): State<SqlitePool>, Path(id): Path<i64>) -> Response {
    match db::get_post(&pool, id).await {
        Ok(Some(post)) => {
            let template = PostTemplate { post };
            Html(template.render().unwrap()).into_response()
        }
        Ok(None) => (StatusCode::NOT_FOUND, "Post not found").into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Database error: {}", e),
        )
            .into_response(),
    }
}

pub async fn new_post_form() -> Html<String> {
    let template = NewPostTemplate;
    Html(template.render().unwrap())
}

pub async fn create_post(
    State(pool): State<SqlitePool>,
    Form(post): Form<CreatePost>,
) -> Response {
    match db::create_post(&pool, post).await {
        Ok(id) => Redirect::to(&format!("/posts/{}", id)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to create post: {}", e),
        )
            .into_response(),
    }
}

pub async fn edit_post_form(State(pool): State<SqlitePool>, Path(id): Path<i64>) -> Response {
    match db::get_post(&pool, id).await {
        Ok(Some(post)) => {
            let template = EditPostTemplate { post };
            Html(template.render().unwrap()).into_response()
        }
        Ok(None) => (StatusCode::NOT_FOUND, "Post not found").into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Database error: {}", e),
        )
            .into_response(),
    }
}

pub async fn update_post(
    State(pool): State<SqlitePool>,
    Path(id): Path<i64>,
    Form(post): Form<UpdatePost>,
) -> Response {
    match db::update_post(&pool, id, post).await {
        Ok(true) => Redirect::to(&format!("/posts/{}", id)).into_response(),
        Ok(false) => (StatusCode::NOT_FOUND, "Post not found").into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to update post: {}", e),
        )
            .into_response(),
    }
}

pub async fn delete_post(State(pool): State<SqlitePool>, Path(id): Path<i64>) -> Response {
    match db::delete_post(&pool, id).await {
        Ok(true) => Redirect::to("/").into_response(),
        Ok(false) => (StatusCode::NOT_FOUND, "Post not found").into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to delete post: {}", e),
        )
            .into_response(),
    }
}
