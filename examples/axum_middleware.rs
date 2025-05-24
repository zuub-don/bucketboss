//! Example of using `bucketboss` with Axum middleware.
//!
//! This example shows how to create a simple rate-limited API server using Axum and the TokenBucket rate limiter.
//! It includes a custom middleware that applies rate limiting to all routes.

use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use bucketboss::{RateLimiter, TokenBucket};
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::Mutex;
use tower::ServiceBuilder;

// A simple state that holds our rate limiter
#[derive(Clone)]
struct AppState {
    rate_limiter: Arc<Mutex<TokenBucket>>,
}

// Custom error type for our application
#[derive(Debug)]
enum AppError {
    RateLimitExceeded,
}

// Implement `IntoResponse` for our error type
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            AppError::RateLimitExceeded => {
                (StatusCode::TOO_MANY_REQUESTS, "Rate limit exceeded").into_response()
            }
        }
    }
}

// Middleware function that applies rate limiting
async fn rate_limiter_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let limiter = state.rate_limiter.lock().await;
    
    if limiter.try_acquire(1).is_err() {
        return Err(AppError::RateLimitExceeded);
    }
    
    let response = next.run(request).await;
    Ok(response)
}

// A simple handler that returns "Hello, World!"
async fn hello_world() -> &'static str {
    "Hello, World!"
}

// A handler that returns the current rate limit status
async fn status(State(state): State<AppState>) -> String {
    let limiter = state.rate_limiter.lock().await;
    format!(
        "Available tokens: {}/{}",
        limiter.available_tokens(),
        limiter.capacity()
    )
}

#[tokio::main]
async fn main() {
    // Create a rate limiter that allows 10 requests per second with a burst of 5
    let rate_limiter = Arc::new(Mutex::new(TokenBucket::new(5, 10.0)));
    
    // Create the application state
    let state = AppState { rate_limiter };
    
    // Build our application with some routes
    let app = Router::new()
        .route("/", get(hello_world))
        .route("/status", get(status))
        .layer(
            ServiceBuilder::new()
                .layer(middleware::from_fn_with_state(
                    state.clone(),
                    |state: axum::extract::State<AppState>, req: Request, next: Next| async move {
                        rate_limiter_middleware(state, req, next).await
                    },
                ))
        )
        .with_state(state);
    
    // Run the server
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Listening on {}", addr);
    
    axum::serve(
        tokio::net::TcpListener::bind(addr).await.unwrap(),
        app
    )
    .await
    .unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        routing::get,
        Router,
    };
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use tokio::net::TcpListener;
    use std::sync::mpsc;

    #[tokio::test]
    async fn test_rate_limiter_middleware() {
        // Create a rate limiter that allows 2 requests per second with a burst of 1
        let rate_limiter = TokenBucket::new(1, 2.0);
        let state = AppState {
            rate_limiter: Arc::new(Mutex::new(rate_limiter)),
        };

        // Build our test application
        let app = Router::new()
            .route("/", get(hello_world))
            .layer(middleware::from_fn_with_state(
                state.clone(),
                |state: State<AppState>, req, next| async move {
                    rate_limiter_middleware(state, req, next).await
                },
            ))
            .with_state(state);

        // Create a channel to communicate the server address
        let (tx, rx) = mpsc::sync_channel(1);
        
        // Clone the app for the server task
        let server_app = app.clone();
        
        // Start the server in a separate thread
        let _server_handle = std::thread::spawn(move || {
            // Create a new runtime for the server thread
            let rt = tokio::runtime::Runtime::new().unwrap();
            
            rt.block_on(async {
                // Bind to a random port
                let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
                let addr = listener.local_addr().unwrap();
                
                // Send the address back to the test
                tx.send(addr).unwrap();
                
                // Start the server
                axum::serve(listener, server_app.into_make_service())
                    .with_graceful_shutdown(async {
                        // This future will resolve when the test is done
                        futures::future::pending::<()>().await
                    })
                    .await
                    .unwrap();
            });
        });
        
        // Get the server address
        let addr = rx.recv().unwrap();
        
        // Create a client
        let client = reqwest::Client::new();
        
        // First request should succeed
        let url = format!("http://{}", addr);
        let response = client.get(&url).send().await.unwrap();
        assert_eq!(response.status(), 200); // 200 OK
        
        // Second request should be rate limited
        let response = client.get(&url).send().await.unwrap();
        assert_eq!(response.status(), 429); // 429 Too Many Requests
        
        // The server will be cleaned up when the test thread is dropped
    }
}
