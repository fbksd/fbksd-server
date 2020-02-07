use std::io;

use actix_web::{error, middleware, web, App, Error, HttpRequest, HttpResponse, HttpServer};
use bytes::BytesMut;
use futures::StreamExt;
use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize)]
struct WebhookPayload {
    created_at: String,
    updated_at: String,
    event_name: String,
    name: String,
    owner_email: String,
    owner_name: String,
    path: String,
    path_with_namespace: String,
    project_id: i32,
    project_visibility: String,
}

// async fn index(item: web::Json<MyObj>) -> HttpResponse {
//     println!("model: {:?}", &item);

//     HttpResponse::Ok().finish()
// }

/// This handler manually load request payload and parse json object
async fn index_manual(mut body: web::Payload, req: HttpRequest) -> Result<HttpResponse, Error> {
    const MAX_SIZE: usize = 262_144; // max payload size is 256k

    let headers = req.headers();
    let event_type = match headers.get("X-Gitlab-Event") {
        Some(header) => String::from(header.to_str().unwrap()),
        None => String::from("")
    };

    println!("Event type = {}", &event_type);

    // payload is a stream of Bytes objects
    let mut bytes = BytesMut::new();
    while let Some(chunk) = body.next().await {
        let chunk = chunk?;
        // limit max size of in-memory payload
        if (bytes.len() + chunk.len()) > MAX_SIZE {
            return Err(error::ErrorBadRequest("overflow"));
        }
        bytes.extend_from_slice(&chunk);
    }

    // body is loaded, now we can deserialize serde-json
    // let obj = serde_json::from_slice::<WebhookPayload>(&bytes)?;
    let obj: Value = serde_json::from_slice(&bytes)?;
    println!("{:?}", obj);
    Ok(HttpResponse::Ok().finish())
}

#[actix_rt::main]
async fn main() -> io::Result<()> {
    std::env::set_var("RUST_LOG", "actix_web=debug");
    env_logger::init();

    println!("Started http server: 0.0.0.0:8190");

    // load ssl keys
    let mut builder = SslAcceptor::mozilla_intermediate(SslMethod::tls()).unwrap();
    builder
        .set_private_key_file("key.pem", SslFiletype::PEM)
        .unwrap();
    builder.set_certificate_chain_file("cert.pem").unwrap();

    HttpServer::new(|| {
        App::new()
            // enable logger
            .wrap(middleware::Logger::default())
            // register simple handler, handle all methods
            .service(
                web::resource("/")
                    .route(web::post().to(index_manual))
                    .route(web::get().to(|| HttpResponse::Ok())),
            )
    })
    .bind_openssl("0.0.0.0:8190", builder)?
    .run()
    .await
}
