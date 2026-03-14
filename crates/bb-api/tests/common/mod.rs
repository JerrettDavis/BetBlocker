#[allow(dead_code)]
use bb_api::config::ApiConfig;
use bb_api::routes;
use bb_api::state::AppState;
use ring::signature::KeyPair;
use serde_json::Value;
use std::io::Write;
use uuid::Uuid;

/// A test application that spawns the API against a real test database.
#[allow(dead_code)]
pub struct TestApp {
    pub address: String,
    pub client: reqwest::Client,
    pub db_name: String,
    pub admin_db_url: String,
}

#[allow(dead_code)]
impl TestApp {
    /// Spawn a test API server with a unique database per test run.
    pub async fn spawn() -> Self {
        let db_name = format!("betblocker_test_{}", Uuid::new_v4().simple());
        let admin_db_url = std::env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/postgres".into());

        // Flush Redis to clear stale lockout keys from previous test runs
        let redis_url = std::env::var("TEST_REDIS_URL")
            .unwrap_or_else(|_| "redis://localhost:6379".into());
        if let Ok(redis_client) = redis::Client::open(redis_url.as_str()) {
            if let Ok(mut conn) = redis_client.get_multiplexed_async_connection().await {
                let _: Result<(), _> = redis::cmd("FLUSHDB").query_async(&mut conn).await;
            }
        }

        // Create test database
        let admin_pool = sqlx::PgPool::connect(&admin_db_url).await.unwrap();
        sqlx::query(&format!("CREATE DATABASE \"{db_name}\""))
            .execute(&admin_pool)
            .await
            .unwrap();
        admin_pool.close().await;

        // Derive the test DB URL from the admin URL by replacing the database name
        let db_url = {
            let base = admin_db_url.rsplitn(2, '/').last().unwrap_or(&admin_db_url);
            format!("{base}/{db_name}")
        };
        let db = sqlx::PgPool::connect(&db_url).await.unwrap();

        // Run migrations: execute each SQL file in order
        let mut migration_files: Vec<_> = std::fs::read_dir("../../migrations")
            .or_else(|_| std::fs::read_dir("migrations"))
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .extension()
                    .is_some_and(|ext| ext == "sql")
            })
            .collect();
        migration_files.sort_by_key(|e| e.file_name());

        for entry in &migration_files {
            let sql = std::fs::read_to_string(entry.path()).unwrap();
            // Split on semicolons, but respect $$ dollar-quoted blocks
            let statements = split_sql_statements(&sql);
            for statement in &statements {
                let trimmed = statement.trim();
                if !trimmed.is_empty() {
                    if let Err(e) = sqlx::query(trimmed).execute(&db).await {
                        eprintln!(
                            "Migration error in {}: {e}",
                            entry.file_name().to_string_lossy()
                        );
                    }
                }
            }
        }

        // Generate ephemeral Ed25519 key pair for tests
        let rng = ring::rand::SystemRandom::new();
        let pkcs8_doc = ring::signature::Ed25519KeyPair::generate_pkcs8(&rng).unwrap();

        // Convert to PEM format
        let private_key_path = std::env::temp_dir().join(format!("{db_name}_private.pem"));
        let public_key_path = std::env::temp_dir().join(format!("{db_name}_public.pem"));

        // Write PKCS8 private key as PEM
        let private_pem = format!(
            "-----BEGIN PRIVATE KEY-----\n{}\n-----END PRIVATE KEY-----\n",
            base64_encode(pkcs8_doc.as_ref())
        );
        std::fs::File::create(&private_key_path)
            .unwrap()
            .write_all(private_pem.as_bytes())
            .unwrap();

        // Extract public key from the keypair
        let key_pair =
            ring::signature::Ed25519KeyPair::from_pkcs8(pkcs8_doc.as_ref()).unwrap();
        let public_key_bytes = key_pair.public_key().as_ref();

        // Create SubjectPublicKeyInfo DER for Ed25519
        // Algorithm OID for Ed25519: 1.3.101.112
        let mut spki_der = Vec::new();
        // SEQUENCE {
        //   SEQUENCE { OID 1.3.101.112 }
        //   BIT STRING (public key)
        // }
        let algorithm_seq = [0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70];
        let bit_string_len = 1 + public_key_bytes.len(); // 1 byte for unused bits count
        let mut bit_string = vec![0x03, bit_string_len as u8, 0x00];
        bit_string.extend_from_slice(public_key_bytes);

        let inner_len = algorithm_seq.len() + bit_string.len();
        spki_der.push(0x30); // SEQUENCE
        spki_der.push(inner_len as u8);
        spki_der.extend_from_slice(&algorithm_seq);
        spki_der.extend_from_slice(&bit_string);

        let public_pem = format!(
            "-----BEGIN PUBLIC KEY-----\n{}\n-----END PUBLIC KEY-----\n",
            base64_encode(&spki_der)
        );
        std::fs::File::create(&public_key_path)
            .unwrap()
            .write_all(public_pem.as_bytes())
            .unwrap();

        let config = ApiConfig {
            database_url: db_url,
            redis_url: std::env::var("TEST_REDIS_URL")
                .unwrap_or_else(|_| "redis://localhost:6379".into()),
            host: "127.0.0.1".into(),
            port: 0, // OS-assigned
            jwt_private_key_path: private_key_path.to_string_lossy().into(),
            jwt_public_key_path: public_key_path.to_string_lossy().into(),
            jwt_access_token_ttl_secs: 3600,
            jwt_refresh_token_ttl_days: 30,
            cors_allowed_origins: vec!["*".into()],
            billing_enabled: false,
            stripe_secret_key: None,
            stripe_webhook_secret: None,
            public_base_url: None,
        };

        let state = AppState::new(config).await.unwrap();
        let app = routes::router(state);

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        Self {
            address: format!("http://{addr}"),
            client: reqwest::Client::new(),
            db_name,
            admin_db_url,
        }
    }

    /// Register a test user. Returns (account_id, access_token, refresh_token).
    pub async fn register_user(
        &self,
        email: &str,
        password: &str,
    ) -> (String, String, String) {
        let resp = self
            .client
            .post(format!("{}/v1/auth/register", self.address))
            .json(&serde_json::json!({
                "email": email,
                "password": password,
                "display_name": "Test User"
            }))
            .send()
            .await
            .unwrap();

        let status = resp.status();
        let body: Value = resp.json().await.unwrap();

        assert_eq!(status.as_u16(), 201, "register failed: {body}");

        let account_id = body["data"]["account"]["id"]
            .as_str()
            .unwrap()
            .to_string();
        let access_token = body["data"]["access_token"]
            .as_str()
            .unwrap()
            .to_string();
        let refresh_token = body["data"]["refresh_token"]
            .as_str()
            .unwrap()
            .to_string();

        (account_id, access_token, refresh_token)
    }

    /// Login and return (access_token, refresh_token).
    pub async fn login(&self, email: &str, password: &str) -> (String, String) {
        let resp = self
            .client
            .post(format!("{}/v1/auth/login", self.address))
            .json(&serde_json::json!({
                "email": email,
                "password": password,
            }))
            .send()
            .await
            .unwrap();

        let body: Value = resp.json().await.unwrap();
        let access_token = body["data"]["access_token"]
            .as_str()
            .unwrap()
            .to_string();
        let refresh_token = body["data"]["refresh_token"]
            .as_str()
            .unwrap()
            .to_string();

        (access_token, refresh_token)
    }

    /// GET with auth header.
    pub async fn authed_get(&self, path: &str, token: &str) -> reqwest::Response {
        self.client
            .get(format!("{}{}", self.address, path))
            .header("Authorization", format!("Bearer {token}"))
            .send()
            .await
            .unwrap()
    }

    /// POST with auth header and JSON body.
    pub async fn authed_post(
        &self,
        path: &str,
        token: &str,
        body: &Value,
    ) -> reqwest::Response {
        self.client
            .post(format!("{}{}", self.address, path))
            .header("Authorization", format!("Bearer {token}"))
            .json(body)
            .send()
            .await
            .unwrap()
    }

    /// PATCH with auth header and JSON body.
    pub async fn authed_patch(
        &self,
        path: &str,
        token: &str,
        body: &Value,
    ) -> reqwest::Response {
        self.client
            .patch(format!("{}{}", self.address, path))
            .header("Authorization", format!("Bearer {token}"))
            .json(body)
            .send()
            .await
            .unwrap()
    }

    /// DELETE with auth header.
    pub async fn authed_delete(&self, path: &str, token: &str) -> reqwest::Response {
        self.client
            .delete(format!("{}{}", self.address, path))
            .header("Authorization", format!("Bearer {token}"))
            .send()
            .await
            .unwrap()
    }
}

impl Drop for TestApp {
    fn drop(&mut self) {
        // Best-effort cleanup of test database
        let db_name = self.db_name.clone();
        let admin_url = self.admin_db_url.clone();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                if let Ok(pool) = sqlx::PgPool::connect(&admin_url).await {
                    // Terminate connections
                    let _ = sqlx::query(&format!(
                        "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = '{db_name}'"
                    ))
                    .execute(&pool)
                    .await;
                    let _ = sqlx::query(&format!("DROP DATABASE IF EXISTS \"{db_name}\""))
                        .execute(&pool)
                        .await;
                }
            });
        });
    }
}

fn base64_encode(data: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(data)
}

/// Split SQL text into individual statements on `;`, respecting `$$` dollar-quoted blocks.
fn split_sql_statements(sql: &str) -> Vec<String> {
    let mut statements = Vec::new();
    let mut current = String::new();
    let mut in_dollar_quote = false;
    let mut chars = sql.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '$' && chars.peek() == Some(&'$') {
            // Consume the second '$'
            chars.next();
            current.push('$');
            current.push('$');
            in_dollar_quote = !in_dollar_quote;
        } else if ch == ';' && !in_dollar_quote {
            let trimmed = current.trim().to_string();
            if !trimmed.is_empty() {
                statements.push(trimmed);
            }
            current.clear();
        } else if ch == '-' && chars.peek() == Some(&'-') && !in_dollar_quote {
            // Skip single-line comments
            current.push(ch);
            while let Some(c) = chars.next() {
                current.push(c);
                if c == '\n' {
                    break;
                }
            }
        } else {
            current.push(ch);
        }
    }

    let trimmed = current.trim().to_string();
    if !trimmed.is_empty() {
        statements.push(trimmed);
    }

    statements
}
