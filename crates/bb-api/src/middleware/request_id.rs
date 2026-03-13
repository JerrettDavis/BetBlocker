use axum::http::{HeaderValue, Request};
use tower_http::request_id::{MakeRequestId, RequestId};
use uuid::Uuid;

/// Generates UUID v7 request IDs for each incoming request.
#[derive(Clone, Default)]
pub struct UuidV7RequestId;

impl MakeRequestId for UuidV7RequestId {
    fn make_request_id<B>(&mut self, _request: &Request<B>) -> Option<RequestId> {
        let id = Uuid::now_v7().to_string();
        HeaderValue::from_str(&id)
            .ok()
            .map(RequestId::new)
    }
}
