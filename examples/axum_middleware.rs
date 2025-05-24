//! Example of using `bucketboss` with Axum middleware.
//!
//! This example shows how to create a simple rate-limited API server using Axum and the TokenBucket rate limiter.
//! It includes a custom middleware that applies rate limiting to all routes.

use axum::{
    body::Bytes,
    error_handling::HandleError,
    extract::State,
    handler::Handler,
    http::{Request, StatusCode},
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
async fn rate_limiter_middleware<B>(
    State(state): State<AppState>,
    request: Request<B>,
    next: Next<B>,
) -> Result<Response, AppError> {
    // Try to acquire a token from the rate limiter
    let mut limiter = state.rate_limiter.lock().await;
    if limiter.try_acquire(1).is_err() {
        return Err(AppError::RateLimitExceeded);
    }

    // If we have a token, proceed with the request
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
    let rate_limiter = TokenBucket::new(5, 10.0);

    // Create the application state
    let state = AppState {
        rate_limiter: Arc::new(Mutex::new(rate_limiter)),
    };

    // Build our application with a route
    let app = Router::new()
        .route("/", get(hello_world))
        .route("/status", get(status))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            |state: State<AppState>, req, next| async move {
                rate_limiter_middleware(state, req, next).await
            },
        ))
        .with_state(state);

    // Run the server
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Server running on http://{}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;
    use axum::http::Request;
    use axum::routing::get;
    use axum::Router;
    use std::net::TcpListener;
    use tokio::sync::Mutex;

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

        // Create a test client
        let client = axum_test::TestClient::new(app);

        // First request should succeed
        let response = client.get("/").send().await;
        assert_eq!(response.status(), StatusCode::OK);

        // Second request should be rate limited
        let response = client.get("/").send().await;
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    }
}
