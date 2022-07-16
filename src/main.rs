use axum::{
    extract::Path,
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get},
    Extension, Json, Router,
};
use rusqlite::{params_from_iter, Connection};
use serde::{Deserialize, Serialize};
use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
};

type Conn = Arc<Mutex<Connection>>;

#[tokio::main]
async fn main() {
    let path = "./todo.db";
    let db = Connection::open(path).unwrap();

    db.execute(
        "
CREATE TABLE IF NOT EXISTS todos (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  author TEXT NOT NULL,
  body TEXT NOT NULL,
  done INTEGER NOT NULL
);
  ",
        (),
    )
    .unwrap();

    let app = Router::new()
        .route("/todos", get(todos).post(todo_create).patch(todo_update))
        .route("/todos/:id", delete(todo_delete))
        .layer(Extension(Arc::new(Mutex::new(db))));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn todos(Extension(db): Extension<Conn>) -> impl IntoResponse {
    let db = db.lock().unwrap();
    let mut stmt = db
        .prepare("SELECT id, author, body, done from todos")
        .unwrap();

    let todos = stmt
        .query_map([], |row| {
            Ok(Todo {
                id: row.get(0)?,
                author: row.get(1)?,
                body: row.get(2)?,
                done: row.get(3)?,
            })
        })
        .unwrap()
        .map(|row| row.unwrap())
        .collect::<Vec<Todo>>();

    (StatusCode::OK, Json(todos))
}

async fn todo_create(
    Json(payload): Json<CreateTodo>,
    Extension(db): Extension<Conn>,
) -> impl IntoResponse {
    let db = db.lock().unwrap();
    db.execute(
        "INSERT INTO todos(author, body, done) values (?1, ?2, ?3)",
        (&payload.author, &payload.body, false),
    )
    .unwrap();

    StatusCode::CREATED
}

async fn todo_delete(Path(id): Path<u64>, Extension(db): Extension<Conn>) -> impl IntoResponse {
    let db = db.lock().unwrap();
    db.execute("DELETE FROM todos where id = ?", [&id]).unwrap();
    StatusCode::OK
}

async fn todo_update(
    Json(input): Json<UpdateTodo>,
    Extension(db): Extension<Conn>,
) -> impl IntoResponse {
    let mut params = Vec::new();
    let mut columns = Vec::new();

    if let Some(author) = input.author {
        columns.push("author = ?");
        params.push(author);
    }
    if let Some(body) = input.body {
        columns.push("body = ?");
        params.push(body);
    }
    if let Some(done) = input.done {
        columns.push("done = ?");
        params.push(if done == true {
            String::from("1")
        } else {
            String::from("0")
        });
    }
    if columns.len() == 0 {
        return StatusCode::BAD_REQUEST;
    }
    let mut sql = String::from("UPDATE todos SET ");
    sql += columns.join(",").as_str();
    sql += " WHERE id = ?";
    params.push(input.id.to_string());

    let db = db.lock().unwrap();
    db.execute(sql.as_str(), params_from_iter(params)).unwrap();

    StatusCode::OK
}

#[derive(Deserialize)]
struct CreateTodo {
    author: String,
    body: String,
}

#[derive(Deserialize)]
struct UpdateTodo {
    id: u64,
    author: Option<String>,
    body: Option<String>,
    done: Option<bool>,
}

#[derive(Serialize)]
struct Todo {
    id: u64,
    author: String,
    body: String,
    done: bool,
}
