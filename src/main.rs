use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::{
    net::SocketAddr,
    path::PathBuf,
    sync::Arc,
};
use tokio::sync::RwLock;
use tower_http::{cors::CorsLayer, services::ServeDir};

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

type AppState = Arc<RwLock<Vec<Pokemon>>>;

const DATA_FILE: &str = "pokemon_data.json";

#[tokio::main]
async fn main() {
    let data_path = PathBuf::from(DATA_FILE);
    let data: Vec<Pokemon> = if data_path.exists() {
        let raw = std::fs::read_to_string(&data_path).expect("Failed to read data file");
        serde_json::from_str(&raw).expect("Failed to parse data file")
    } else {
        eprintln!("Error: {} not found.", DATA_FILE);
        std::process::exit(1);
    };

    let state: AppState = Arc::new(RwLock::new(data));

    let app = Router::new()
        .route("/api/pokemon", get(get_pokemon))
        .route("/api/pokemon/{id}/toggle", post(toggle_caught))
        .fallback_service(ServeDir::new("static"))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    println!("Pokédex tracker running at http://0.0.0.0:3000");
    println!("Access from other devices using this machine's IP address on port 3000.");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn get_pokemon(State(state): State<AppState>) -> Json<Vec<Pokemon>> {
    let data = state.read().await;
    Json(data.clone())
}

async fn toggle_caught(
    Path(id): Path<u32>,
    State(state): State<AppState>,
) -> Result<Json<Pokemon>, StatusCode> {
    let mut data = state.write().await;

    let pokemon = data
        .iter_mut()
        .find(|p| p.number == id)
        .ok_or(StatusCode::NOT_FOUND)?;

    pokemon.caught = !pokemon.caught;
    let updated = pokemon.clone();

    let json = serde_json::to_string_pretty(&*data).unwrap();
    drop(data);
    std::fs::write(DATA_FILE, json).ok();

    Ok(Json(updated))
}
