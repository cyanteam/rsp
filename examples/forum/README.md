# RSP Forum Example

A simple forum built with RSP + SQLite.

## How to Run

```bash
# Start the development server
rsp -S 0.0.0.0:8080 -t examples/forum

# Or run from the parent directory
cd rsp
rsp -S 0.0.0.0:8080 -t examples/forum
```

Then open http://localhost:8080 in your browser.

## Features

- Create posts
- View posts
- Reply to posts

## File Structure

```
forum/
├── index.rsp    # Home page - list all posts
├── new.rsp      # Create new post
└── post.rsp     # View post and add replies
```

## Database

Uses SQLite with two tables:
- `posts`: id, title, author, content, created_at
- `replies`: id, post_id, author, content, created_at

The database file `forum.db` is created automatically.
