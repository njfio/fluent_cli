# Simple Blog Application

A lightweight blogging application built with Rust, Axum, and SQLite.

## Features

- ✍️ Create, read, update, and delete blog posts
- 🎨 Clean, responsive UI
- 💾 SQLite database for persistence
- 🚀 Fast and lightweight

## Prerequisites

- Rust 1.70 or later
- Cargo

## Getting Started

### Installation

1. Navigate to the blog_example directory:
```bash
cd blog_example
```

2. Build the project:
```bash
cargo build --release
```

### Running the Application

Start the server:
```bash
cargo run --release
```

The application will be available at `http://127.0.0.1:3000`

## Usage

### Home Page
Visit `http://127.0.0.1:3000` to see all blog posts in reverse chronological order.

### Create a Post
1. Click "New Post" in the navigation
2. Fill in the title, author, and content
3. Click "Create Post"

### View a Post
Click on any post title from the home page to view the full post.

### Edit a Post
1. Navigate to a post
2. Click the "Edit" button
3. Modify the title and/or content
4. Click "Update Post"

### Delete a Post
1. Navigate to a post
2. Click the "Delete" button
3. Confirm the deletion

## Project Structure

```
blog_example/
├── src/
│   ├── main.rs          # Application entry point and routing
│   ├── handlers.rs      # HTTP request handlers
│   ├── models.rs        # Data models
│   └── db.rs           # Database operations
├── templates/           # HTML templates (Askama)
│   ├── base.html
│   ├── index.html
│   ├── post.html
│   ├── new.html
│   └── edit.html
├── static/
│   └── style.css       # CSS styles
├── Cargo.toml          # Project dependencies
└── blog.db            # SQLite database (created on first run)
```

## Technology Stack

- **Web Framework**: [Axum](https://github.com/tokio-rs/axum) - Fast, ergonomic web framework
- **Database**: [SQLx](https://github.com/launchbadge/sqlx) with SQLite - Async SQL toolkit
- **Templates**: [Askama](https://github.com/djc/askama) - Type-safe, compiled templates
- **Runtime**: [Tokio](https://tokio.rs) - Async runtime

## API Routes

- `GET /` - List all posts
- `GET /posts/new` - New post form
- `POST /posts` - Create a new post
- `GET /posts/:id` - View a specific post
- `GET /posts/:id/edit` - Edit post form
- `POST /posts/:id/update` - Update a post
- `POST /posts/:id/delete` - Delete a post

## Development

### Run in development mode:
```bash
cargo run
```

### Run tests (if added):
```bash
cargo test
```

### Format code:
```bash
cargo fmt
```

### Lint code:
```bash
cargo clippy
```

## License

MIT
