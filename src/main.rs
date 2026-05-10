use axum::{
    extract::{Path, Query, State},
    http::{Request, StatusCode},
    middleware::{self, Next},
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
    Form, Json, Router,
};
use axum_extra::extract::cookie::{Cookie, CookieJar};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, net::SocketAddr, sync::Arc};
use tokio::sync::RwLock;
use tower_http::{cors::CorsLayer, services::ServeDir};
use uuid::Uuid;

// ── Data types ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Pokemon {
    caught: bool,
    number: u32,
    name: String,
    type1: String,
    type2: String,
    generation: u8,
    region: String,
    notes: String,
}

#[derive(Deserialize)]
struct LoginForm {
    username: String,
    password: String,
}

// ── App state ─────────────────────────────────────────────────────────────────

struct AppStateInner {
    pokemon: RwLock<Vec<Pokemon>>,
    sessions: RwLock<HashSet<String>>,
    data_file: String,
    username: String,
    password: String,
}

type AppState = Arc<AppStateInner>;

// ── Main ──────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    // Data file: use DATA_FILE env var (set on Fly.io), default to local file
    let data_file = std::env::var("DATA_FILE").unwrap_or_else(|_| "pokemon_data.json".to_string());

    let data: Vec<Pokemon> = {
        let raw = std::fs::read_to_string(&data_file)
            .unwrap_or_else(|_| panic!("Could not read data file: {}", data_file));
        serde_json::from_str(&raw).expect("Failed to parse data file")
    };

    // Credentials from env vars; fall back to defaults for local dev
    let username = std::env::var("POKEDEX_USERNAME").unwrap_or_else(|_| "dave".to_string());
    let password = std::env::var("POKEDEX_PASSWORD").unwrap_or_else(|_| "changeme".to_string());

    let state: AppState = Arc::new(AppStateInner {
        pokemon: RwLock::new(data),
        sessions: RwLock::new(HashSet::new()),
        data_file,
        username,
        password,
    });

    let app = Router::new()
        .route("/login", get(login_page).post(login_handler))
        .route("/logout", get(logout_handler))
        .route("/api/pokemon", get(get_pokemon))
        .route("/api/pokemon/{id}/toggle", post(toggle_caught))
        .fallback_service(ServeDir::new("static"))
        .layer(middleware::from_fn_with_state(state.clone(), auth_middleware))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    println!("Pokédex tracker running at http://0.0.0.0:3000");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

// ── Auth middleware ───────────────────────────────────────────────────────────

async fn auth_middleware(
    State(state): State<AppState>,
    request: Request<axum::body::Body>,
    next: Next,
) -> Response {
    let path = request.uri().path().to_string();

    // Always allow the login page through
    if path == "/login" {
        return next.run(request).await;
    }

    // Extract session_id cookie manually from request headers
    let session_id = request
        .headers()
        .get("cookie")
        .and_then(|v| v.to_str().ok())
        .and_then(|cookies| {
            cookies
                .split(';')
                .map(|c| c.trim())
                .find(|c| c.starts_with("session_id="))
                .map(|c| c["session_id=".len()..].to_string())
        });

    let is_valid = match session_id {
        Some(id) => state.sessions.read().await.contains(&id),
        None => false,
    };

    if is_valid {
        next.run(request).await
    } else {
        Redirect::to("/login").into_response()
    }
}

// ── Login / Logout ────────────────────────────────────────────────────────────

async fn login_page(Query(params): Query<std::collections::HashMap<String, String>>) -> Html<String> {
    let show_error = params.contains_key("error");
    Html(login_html(show_error))
}

async fn login_handler(
    State(state): State<AppState>,
    jar: CookieJar,
    Form(form): Form<LoginForm>,
) -> impl IntoResponse {
    if form.username == state.username && form.password == state.password {
        let session_id = Uuid::new_v4().to_string();
        state.sessions.write().await.insert(session_id.clone());

        let mut cookie = Cookie::new("session_id", session_id);
        cookie.set_path("/");
        cookie.set_http_only(true);
        // 30-day session
        cookie.set_max_age(time::Duration::days(30));

        (jar.add(cookie), Redirect::to("/")).into_response()
    } else {
        Redirect::to("/login?error=1").into_response()
    }
}

async fn logout_handler(State(state): State<AppState>, jar: CookieJar) -> impl IntoResponse {
    if let Some(cookie) = jar.get("session_id") {
        state.sessions.write().await.remove(cookie.value());
    }
    let removal = Cookie::build(Cookie::from("session_id")).path("/").build();
    (jar.remove(removal), Redirect::to("/login")).into_response()
}

// ── API ───────────────────────────────────────────────────────────────────────

async fn get_pokemon(State(state): State<AppState>) -> Json<Vec<Pokemon>> {
    Json(state.pokemon.read().await.clone())
}

async fn toggle_caught(
    Path(id): Path<u32>,
    State(state): State<AppState>,
) -> Result<Json<Pokemon>, StatusCode> {
    let mut data = state.pokemon.write().await;

    let pokemon = data
        .iter_mut()
        .find(|p| p.number == id)
        .ok_or(StatusCode::NOT_FOUND)?;

    pokemon.caught = !pokemon.caught;
    let updated = pokemon.clone();

    let json = serde_json::to_string_pretty(&*data).unwrap();
    let path = state.data_file.clone();
    drop(data);
    std::fs::write(&path, json).ok();

    Ok(Json(updated))
}

// ── Login HTML ────────────────────────────────────────────────────────────────

fn login_html(show_error: bool) -> String {
    let error_block = if show_error {
        r#"<div class="error">Incorrect username or password.</div>"#
    } else {
        ""
    };

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <title>Pokédex Tracker — Login</title>
  <style>
    *, *::before, *::after {{ box-sizing: border-box; margin: 0; padding: 0; }}
    body {{
      font-family: system-ui, -apple-system, sans-serif;
      background: #f5f5f5;
      min-height: 100vh;
      display: flex;
      flex-direction: column;
      align-items: center;
      justify-content: center;
    }}
    .card {{
      background: #fff;
      border-radius: 12px;
      box-shadow: 0 4px 20px rgba(0,0,0,.12);
      padding: 2.5rem 2rem;
      width: 100%;
      max-width: 360px;
    }}
    .logo {{
      text-align: center;
      font-size: 1.5rem;
      font-weight: 700;
      color: #cc0000;
      margin-bottom: 1.75rem;
    }}
    label {{
      display: block;
      font-size: .875rem;
      font-weight: 600;
      color: #444;
      margin-bottom: .3rem;
    }}
    input {{
      width: 100%;
      padding: .6rem .8rem;
      border: 1px solid #ddd;
      border-radius: 8px;
      font-size: 1rem;
      margin-bottom: 1rem;
      transition: border-color .15s;
    }}
    input:focus {{ outline: none; border-color: #cc0000; }}
    button {{
      width: 100%;
      padding: .7rem;
      background: #cc0000;
      color: #fff;
      border: none;
      border-radius: 8px;
      font-size: 1rem;
      font-weight: 600;
      cursor: pointer;
      transition: background .15s;
    }}
    button:hover {{ background: #a00000; }}
    .error {{
      background: #fdecea;
      color: #c62828;
      border: 1px solid #ef9a9a;
      border-radius: 8px;
      padding: .6rem .9rem;
      font-size: .875rem;
      margin-bottom: 1rem;
    }}
  </style>
</head>
<body>
  <div class="card">
    <div class="logo">⚡ Pokédex Tracker</div>
    {error_block}
    <form method="POST" action="/login">
      <label for="username">Username</label>
      <input id="username" name="username" type="text" autocomplete="username" required autofocus />
      <label for="password">Password</label>
      <input id="password" name="password" type="password" autocomplete="current-password" required />
      <button type="submit">Sign in</button>
    </form>
  </div>
</body>
</html>"#,
        error_block = error_block
    )
}
