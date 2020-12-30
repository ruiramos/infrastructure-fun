use std::env;
use tide::Request;
use tide::convert::json;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use data_storage_service::encrypt::{encrypt_data, decrypt_data};

#[derive(Clone, Default, Debug)]
struct State {
    map: Arc<Mutex<HashMap<String, String>>>
}
impl State {
    fn new() -> Self {
        State {
            map: Arc::new(Mutex::new(HashMap::new()))
        }
    }
}

#[async_std::main]
async fn main() -> tide::Result<()> {
    let port = env::var("PORT").unwrap_or_else(|_| "8088".to_string());

    let mut app = tide::with_state(State::new());

    app.at("/").post(store_data);
    app.at("/:id").post(get_data);
    app.at("/healthz").get(say_ok);

    println!("* Starting the server running on port {}", port);

    app.listen(format!("0.0.0.0:{}", port)).await?;
    Ok(())
}

async fn store_data(mut req: Request<State>) -> tide::Result {
    let data = req.body_string().await?;
    let state: &State = req.state();

    if data.is_empty() {
        return Err(tide::Error::from_str(tide::http::StatusCode::BadRequest, "Missing data on POST body"));
    }

    let (hash, password, encrypted) = encrypt_data(data);

    let mut map = state.map.lock().unwrap();
    map.insert(hash.clone(), encrypted);

    let prefix_url = env::var("SERVICE_URL").unwrap_or_else(|_| "http://localhost:8088/".to_string());

    Ok(json!({"url": format!("{}{}", prefix_url, hash), "password": password}).into())
}

async fn get_data(mut req: Request<State>) -> tide::Result {
    let password = req.body_string().await?;
    let hash = req.param("id")?;
    let state: &State = req.state();

    let map = state.map.lock().unwrap();
    let value = map.get(hash);

    match value {
        None => Err(tide::Error::from_str(tide::http::StatusCode::Forbidden, "Forbidden")),
        Some(encrypted) => {
            let decrypted = decrypt_data(encrypted.to_string(), password);
            match decrypted {
                None => Err(tide::Error::from_str(tide::http::StatusCode::Forbidden, "Forbidden")),
                Some(v) => Ok(v.into())
            }
        }
    }
}

async fn say_ok(mut _req: Request<State>) -> tide::Result {
    Ok("ok".into())
}
