use jsonwebtoken::{decode, DecodingKey, EncodingKey, Header, TokenData, Validation};
use serde::{Deserialize, Serialize};
use chrono::{Utc, Duration};
use anyhow::{bail, Result};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FileAccessClaim {
    pub path: String,
    pub permissions: Vec<String>,
    pub user: Uuid
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DataClaims {
    pub iss: String,
    pub sub: String,     // user id or session id
    pub aud: String,     // storage service audience
    pub exp: i64,
    pub iat: i64,
    pub jti: String,     // unique token id
    pub res: FileAccessClaim,     // resource path, e.g. "/users/123/files/*"
    // optional:
    pub ver: Option<u64>, // session version for revocation strategies
}

pub fn issue_scoped_token(
    issuer: &str,
    subject: &str,
    audience: &str,
    resource: FileAccessClaim,
    ttl_seconds: i64,
    encoding_key: &EncodingKey, // from RSA private key or HMAC secret
) -> Result<String> {
    let now = Utc::now();
    let exp = now + Duration::seconds(ttl_seconds);

    let claims = DataClaims {
        iss: issuer.to_owned(),
        sub: subject.to_owned(),
        aud: audience.to_owned(),
        iat: now.timestamp(),
        exp: exp.timestamp(),
        jti: uuid::Uuid::new_v4().to_string(),
        res: resource,
        ver: None,
    };

    let header = Header::new(jsonwebtoken::Algorithm::HS512);
    let token = jsonwebtoken::encode(&header, &claims, encoding_key)?;
    Ok(token)
}

pub fn verify_and_authorize(
    token: &str,
    decoding_key: &DecodingKey,
    expected_iss: &str,
    expected_aud: &str,
    requested_resource: &str,
    requested_perm: &str, // "read" / "write"
) -> Result<DataClaims> {
    let mut validation = Validation::new(jsonwebtoken::Algorithm::HS512);
    validation.set_audience(&[expected_aud]);
    validation.set_issuer(&[expected_iss]);
    validation.leeway = 60;

    let token_data: TokenData<DataClaims> = decode::<DataClaims>(token, decoding_key, &validation)?;
    let claims = token_data.claims;

    // Check expiration manually if needed
    let now_ts = Utc::now().timestamp();
    if claims.exp < now_ts {
        bail!("token expired");
    }

    // Resource match: implement your own matching rules:
    if !resource_matches(&claims.res.path, requested_resource) {
        bail!("resource not allowed by token");
    }

    // Permission check:
    if !claims.res.permissions.iter().any(|p| p == requested_perm) {
        bail!("permission denied");
    }

    Ok(claims)
}

// Example simplistic resource matcher (prefix + wildcard)
fn resource_matches(token_resource: &str, request_resource: &str) -> bool {
    if token_resource.ends_with("/*") {
        let prefix = token_resource.trim_end_matches("/*");
        request_resource.starts_with(prefix)
    } else {
        token_resource == request_resource
    }
}