use axum::body::Body;
use axum::extract::Json as AxumJson;
use axum::http::{Request, StatusCode};
use axum::routing::post;
use axum::{Json, Router as AxumRouter};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use srvcs_exponent::{api::Deps, health, router, telemetry};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tower::ServiceExt;

const DEAD_URL: &str = "http://127.0.0.1:1";

// --- Computing mocks for every srvcs dependency kind. ---
//
// srvcs-exponent only depends on srvcs-floatpower, but per the srvcs test
// convention each dependency *kind* gets a genuinely computing mock so the
// composition is exercised against real answers rather than canned values.

/// `srvcs-floatadd`: `{"a", "b"}` -> `{"result": a + b}` (f64).
async fn spawn_floatadd() -> String {
    let app = AxumRouter::new().route(
        "/",
        post(|AxumJson(body): AxumJson<Value>| async move {
            let a = body.get("a").and_then(Value::as_f64).unwrap_or(0.0);
            let b = body.get("b").and_then(Value::as_f64).unwrap_or(0.0);
            Json(json!({ "result": a + b }))
        }),
    );
    serve(app).await
}

/// `srvcs-floatmultiply`: `{"a", "b"}` -> `{"result": a * b}` (f64).
async fn spawn_floatmultiply() -> String {
    let app = AxumRouter::new().route(
        "/",
        post(|AxumJson(body): AxumJson<Value>| async move {
            let a = body.get("a").and_then(Value::as_f64).unwrap_or(0.0);
            let b = body.get("b").and_then(Value::as_f64).unwrap_or(0.0);
            Json(json!({ "result": a * b }))
        }),
    );
    serve(app).await
}

/// `srvcs-floatdivide`: `{"a", "b"}` -> `{"result": a / b}` (f64).
async fn spawn_floatdivide() -> String {
    let app = AxumRouter::new().route(
        "/",
        post(|AxumJson(body): AxumJson<Value>| async move {
            let a = body.get("a").and_then(Value::as_f64).unwrap_or(0.0);
            let b = body.get("b").and_then(Value::as_f64).unwrap_or(1.0);
            Json(json!({ "result": a / b }))
        }),
    );
    serve(app).await
}

/// `srvcs-floatsubtract`: `{"a", "b"}` -> `{"result": a - b}` (f64).
async fn spawn_floatsubtract() -> String {
    let app = AxumRouter::new().route(
        "/",
        post(|AxumJson(body): AxumJson<Value>| async move {
            let a = body.get("a").and_then(Value::as_f64).unwrap_or(0.0);
            let b = body.get("b").and_then(Value::as_f64).unwrap_or(0.0);
            Json(json!({ "result": a - b }))
        }),
    );
    serve(app).await
}

/// `srvcs-floatpower`: `{"base", "exp"}` -> `{"result": base.powf(exp)}` (f64).
async fn spawn_floatpower() -> String {
    let app = AxumRouter::new().route(
        "/",
        post(|AxumJson(body): AxumJson<Value>| async move {
            let base = body.get("base").and_then(Value::as_f64).unwrap_or(0.0);
            let exp = body.get("exp").and_then(Value::as_f64).unwrap_or(0.0);
            Json(json!({ "result": base.powf(exp) }))
        }),
    );
    serve(app).await
}

/// `srvcs-ln`: `{"value"}` -> `{"result": value.ln()}` (f64).
async fn spawn_ln() -> String {
    let app = AxumRouter::new().route(
        "/",
        post(|AxumJson(body): AxumJson<Value>| async move {
            let value = body.get("value").and_then(Value::as_f64).unwrap_or(1.0);
            Json(json!({ "result": value.ln() }))
        }),
    );
    serve(app).await
}

/// `srvcs-multiply`: `{"a", "b"}` -> `{"result": a * b}` (i64).
async fn spawn_multiply() -> String {
    let app = AxumRouter::new().route(
        "/",
        post(|AxumJson(body): AxumJson<Value>| async move {
            let a = body.get("a").and_then(Value::as_i64).unwrap_or(0);
            let b = body.get("b").and_then(Value::as_i64).unwrap_or(0);
            Json(json!({ "result": a * b }))
        }),
    );
    serve(app).await
}

/// `srvcs-reciprocal`: `{"value"}` -> `{"result": 1 / value}` (f64).
async fn spawn_reciprocal() -> String {
    let app = AxumRouter::new().route(
        "/",
        post(|AxumJson(body): AxumJson<Value>| async move {
            let value = body.get("value").and_then(Value::as_f64).unwrap_or(1.0);
            Json(json!({ "result": 1.0 / value }))
        }),
    );
    serve(app).await
}

/// `srvcs-root`: `{"value", "n"}` -> `{"result": value.powf(1/n)}` (f64).
async fn spawn_root() -> String {
    let app = AxumRouter::new().route(
        "/",
        post(|AxumJson(body): AxumJson<Value>| async move {
            let value = body.get("value").and_then(Value::as_f64).unwrap_or(0.0);
            let n = body.get("n").and_then(Value::as_f64).unwrap_or(1.0);
            Json(json!({ "result": value.powf(1.0 / n) }))
        }),
    );
    serve(app).await
}

/// Spawn a mock returning a fixed status + body (used for error-path tests).
async fn spawn_fixed(status: StatusCode, body: Value) -> String {
    let app = AxumRouter::new().route(
        "/",
        post(move || {
            let body = body.clone();
            async move { (status, Json(body)) }
        }),
    );
    serve(app).await
}

async fn serve(app: AxumRouter) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    format!("http://{addr}")
}

fn app(floatpower_url: &str) -> axum::Router {
    router(
        telemetry::metrics_handle_for_tests(),
        Deps {
            floatpower_url: floatpower_url.to_string(),
        },
    )
}

async fn exponent(floatpower_url: &str, base: f64, exp: f64) -> (StatusCode, Value) {
    let res = app(floatpower_url)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/")
                .header("content-type", "application/json")
                .body(Body::from(json!({ "base": base, "exp": exp }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

async fn status_of(uri: &str) -> StatusCode {
    app(DEAD_URL)
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap()
        .status()
}

fn result_of(body: &Value) -> f64 {
    body["result"].as_f64().expect("result is a number")
}

/// POST `body` to a computing mock at `url` and read its numeric `result`.
/// Uses the same minimal localhost HTTP/1.1 dance as the production client.
async fn mock_result(url: &str, body: &Value) -> f64 {
    let rest = url.strip_prefix("http://").unwrap();
    let mut stream = tokio::net::TcpStream::connect(rest).await.unwrap();
    let payload = body.to_string();
    let req = format!(
        "POST / HTTP/1.1\r\nHost: {rest}\r\nContent-Type: application/json\r\n\
         Content-Length: {len}\r\nConnection: close\r\n\r\n{payload}",
        len = payload.len(),
    );
    stream.write_all(req.as_bytes()).await.unwrap();
    stream.flush().await.unwrap();
    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).await.unwrap();
    let text = String::from_utf8_lossy(&raw);
    let (_, b) = text.split_once("\r\n\r\n").unwrap();
    let v: Value = serde_json::from_str(b).unwrap();
    v["result"].as_f64().unwrap()
}

// --- Standard endpoints. ---

#[tokio::test]
async fn healthz_ok() {
    assert_eq!(status_of("/healthz").await, StatusCode::OK);
}

#[tokio::test]
async fn readyz_reflects_state() {
    health::set_ready(true);
    assert_eq!(status_of("/readyz").await, StatusCode::OK);
}

#[tokio::test]
async fn metrics_ok() {
    assert_eq!(status_of("/metrics").await, StatusCode::OK);
}

#[tokio::test]
async fn openapi_ok() {
    assert_eq!(status_of("/openapi.json").await, StatusCode::OK);
}

#[tokio::test]
async fn generates_request_id_when_absent() {
    let res = app(DEAD_URL)
        .oneshot(
            Request::builder()
                .uri("/healthz")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(
        res.headers().contains_key("x-request-id"),
        "response must carry a generated x-request-id"
    );
}

#[tokio::test]
async fn index_reports_identity() {
    let res = app(DEAD_URL)
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let body: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["service"], "srvcs-exponent");
    assert_eq!(body["concern"], "arithmetic: base raised to exp (real)");
    assert_eq!(body["depends_on"], json!(["srvcs-floatpower"]));
}

// --- Correctness cases, against the computing floatpower mock. ---

#[tokio::test]
async fn exponent_2_10_is_1024() {
    let fp = spawn_floatpower().await;
    let (status, body) = exponent(&fp, 2.0, 10.0).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["base"], 2.0);
    assert_eq!(body["exp"], 10.0);
    assert!((result_of(&body) - 1024.0).abs() < 1e-9);
}

#[tokio::test]
async fn exponent_9_half_is_3() {
    let fp = spawn_floatpower().await;
    let (status, body) = exponent(&fp, 9.0, 0.5).await;
    assert_eq!(status, StatusCode::OK);
    // 9 ^ 0.5 == sqrt(9) == 3
    assert!((result_of(&body) - 3.0).abs() < 1e-9);
}

#[tokio::test]
async fn exponent_anything_to_zero_is_one() {
    let fp = spawn_floatpower().await;
    let (status, body) = exponent(&fp, 7.5, 0.0).await;
    assert_eq!(status, StatusCode::OK);
    assert!((result_of(&body) - 1.0).abs() < 1e-9);
}

#[tokio::test]
async fn exponent_negative_exp_is_reciprocal_power() {
    let fp = spawn_floatpower().await;
    let (status, body) = exponent(&fp, 2.0, -2.0).await;
    assert_eq!(status, StatusCode::OK);
    // 2 ^ -2 == 0.25
    assert!((result_of(&body) - 0.25).abs() < 1e-9);
}

#[tokio::test]
async fn exponent_fractional_base_and_exp() {
    let fp = spawn_floatpower().await;
    let (status, body) = exponent(&fp, 2.5, 1.5).await;
    assert_eq!(status, StatusCode::OK);
    let expected = 2.5_f64.powf(1.5);
    assert!((result_of(&body) - expected).abs() < 1e-9);
}

// --- Error / degraded paths. ---

#[tokio::test]
async fn degrades_when_floatpower_unreachable() {
    let (status, body) = exponent(DEAD_URL, 2.0, 10.0).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["dependency"], "srvcs-floatpower");
}

#[tokio::test]
async fn forwards_422_from_floatpower() {
    let fp = spawn_fixed(
        StatusCode::UNPROCESSABLE_ENTITY,
        json!({ "error": "value is not a number" }),
    )
    .await;
    let (status, _) = exponent(&fp, 2.0, 10.0).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn malformed_floatpower_result_is_500() {
    let fp = spawn_fixed(StatusCode::OK, json!({ "result": "not-a-number" })).await;
    let (status, body) = exponent(&fp, 2.0, 10.0).await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(body["dependency"], "srvcs-floatpower");
}

// --- The remaining computing mocks model the other srvcs dependency kinds;
//     exercise each so it is genuinely computing (approximate comparison). ---

#[tokio::test]
async fn computing_mocks_compute() {
    let add = spawn_floatadd().await;
    let mul = spawn_floatmultiply().await;
    let div = spawn_floatdivide().await;
    let sub = spawn_floatsubtract().await;
    let pow = spawn_floatpower().await;
    let ln = spawn_ln().await;
    let imul = spawn_multiply().await;
    let recip = spawn_reciprocal().await;
    let root = spawn_root().await;

    assert!((mock_result(&add, &json!({"a": 1.5, "b": 2.0})).await - 3.5).abs() < 1e-9);
    assert!((mock_result(&mul, &json!({"a": 3.0, "b": 4.0})).await - 12.0).abs() < 1e-9);
    assert!((mock_result(&div, &json!({"a": 9.0, "b": 2.0})).await - 4.5).abs() < 1e-9);
    assert!((mock_result(&sub, &json!({"a": 5.0, "b": 2.0})).await - 3.0).abs() < 1e-9);
    assert!((mock_result(&pow, &json!({"base": 2.0, "exp": 3.0})).await - 8.0).abs() < 1e-9);
    assert!((mock_result(&ln, &json!({"value": 1.0})).await - 0.0).abs() < 1e-9);
    assert!((mock_result(&imul, &json!({"a": 6, "b": 7})).await - 42.0).abs() < 1e-9);
    assert!((mock_result(&recip, &json!({"value": 4.0})).await - 0.25).abs() < 1e-9);
    assert!((mock_result(&root, &json!({"value": 27.0, "n": 3.0})).await - 3.0).abs() < 1e-9);
}
