use std::env;
use tide::Request;
use tide::convert::json;
extern crate redis;
use redis::Commands;

use data_storage_service::encrypt::{encrypt_data, decrypt_data};

#[async_std::main]
async fn main() -> tide::Result<()> {
    tide::log::start();
    let port = env::var("PORT").unwrap_or_else(|_| "8088".to_string());

    let mut app = tide::new();

    app.with(tide::log::LogMiddleware::new());
    app.at("/").post(store_data);
    app.at("/:id").post(get_data);
    app.at("/healthz").get(say_ok);

    app.listen(format!("0.0.0.0:{}", port)).await?;
    Ok(())
}

async fn store_data(mut req: Request<()>) -> tide::Result {
    let client = get_redis_client()?;
    let mut con = client.get_connection()?;

    let data = req.body_string().await?;

    if data.is_empty() {
        return Err(tide::Error::from_str(tide::http::StatusCode::BadRequest, "Missing data on POST body"));
    }

    let (hash, password, encrypted) = encrypt_data(data);

    let _ : () = con.set(hash.clone(), encrypted)?;

    let prefix_url = env::var("SERVICE_URL").unwrap_or_else(|_| "http://localhost:8088/".to_string());

    Ok(json!({"url": format!("{}{}", prefix_url, hash), "password": password}).into())
}

async fn get_data(mut req: Request<()>) -> tide::Result {
    let client = get_redis_client()?;
    let mut con = client.get_connection()?;

    let password = req.body_string().await?;

    let hash = req.param("id")?;

    let value: Option<String> = con.get(hash).ok();

    match value {
        None => Err(tide::Error::from_str(tide::http::StatusCode::Forbidden, "Forbidden")),
        Some(encrypted) => {
            let decrypted = decrypt_data(encrypted, password);
            match decrypted {
                None => Err(tide::Error::from_str(tide::http::StatusCode::Forbidden, "Forbidden")),
                Some(v) => Ok(v.into())
            }
        }
    }
}

async fn say_ok(mut _req: Request<()>) -> tide::Result {
    Ok("ok".into())
}

fn get_redis_host() -> String {
    env::var("REDIS_URL").unwrap_or_else(|_| "127.0.0.1/".to_string())
}

fn get_redis_client() -> Result<redis::Client, redis::RedisError> {
    redis::Client::open(format!("redis://{}", get_redis_host()))
}

