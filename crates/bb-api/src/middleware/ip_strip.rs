use axum::http::Request;
use tower::{Layer, Service};

/// Tower layer that strips IP-identifying headers from incoming requests.
///
/// Removes `X-Forwarded-For`, `X-Real-Ip`, and `Forwarded` headers so that
/// federated-report endpoints cannot be used to fingerprint reporters.
#[derive(Clone, Default)]
pub struct StripSourceIpLayer;

impl<S> Layer<S> for StripSourceIpLayer {
    type Service = StripSourceIp<S>;

    fn layer(&self, inner: S) -> Self::Service {
        StripSourceIp { inner }
    }
}

/// Service produced by [`StripSourceIpLayer`].
#[derive(Clone)]
pub struct StripSourceIp<S> {
    inner: S,
}

impl<S, B> Service<Request<B>> for StripSourceIp<S>
where
    S: Service<Request<B>>,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request<B>) -> Self::Future {
        let headers = req.headers_mut();
        headers.remove("x-forwarded-for");
        headers.remove("x-real-ip");
        headers.remove("forwarded");
        self.inner.call(req)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{HeaderValue, Request, StatusCode};
    use tower::{ServiceBuilder, ServiceExt};

    #[tokio::test]
    async fn strips_ip_headers() {
        // A trivial inner service that echoes back which headers remain.
        let svc = ServiceBuilder::new()
            .layer(StripSourceIpLayer)
            .service(tower::service_fn(|req: Request<Body>| async move {
                let has_xff = req.headers().contains_key("x-forwarded-for");
                let has_xri = req.headers().contains_key("x-real-ip");
                let has_fwd = req.headers().contains_key("forwarded");
                // Return 200 only if all three are absent.
                let status = if has_xff || has_xri || has_fwd {
                    StatusCode::BAD_REQUEST
                } else {
                    StatusCode::OK
                };
                Ok::<_, std::convert::Infallible>(axum::response::Response::builder()
                    .status(status)
                    .body(Body::empty())
                    .unwrap())
            }));

        let req = Request::builder()
            .uri("/test")
            .header("x-forwarded-for", HeaderValue::from_static("1.2.3.4"))
            .header("x-real-ip", HeaderValue::from_static("1.2.3.4"))
            .header("forwarded", HeaderValue::from_static("for=1.2.3.4"))
            .header("content-type", HeaderValue::from_static("application/json"))
            .body(Body::empty())
            .unwrap();

        let response = svc.oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK, "IP headers should have been stripped");
    }

    #[tokio::test]
    async fn passes_other_headers_through() {
        let svc = ServiceBuilder::new()
            .layer(StripSourceIpLayer)
            .service(tower::service_fn(|req: Request<Body>| async move {
                let has_ct = req.headers().contains_key("content-type");
                let status = if has_ct {
                    StatusCode::OK
                } else {
                    StatusCode::BAD_REQUEST
                };
                Ok::<_, std::convert::Infallible>(axum::response::Response::builder()
                    .status(status)
                    .body(Body::empty())
                    .unwrap())
            }));

        let req = Request::builder()
            .uri("/test")
            .header("content-type", HeaderValue::from_static("application/json"))
            .body(Body::empty())
            .unwrap();

        let response = svc.oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK, "non-IP headers should be preserved");
    }
}
