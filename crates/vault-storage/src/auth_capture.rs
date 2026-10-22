//! Structural ChatGPT OAuth capture only. JWT signatures, expiry and authority
//! are not verified here; the owned official helper must accept the account.
//! Shapes follow OpenAI codex-rs/login/src/{auth/storage,token_data}.rs and
//! app-server-protocol/src/protocol/common.rs AuthMode. No credential I/O.
#![forbid(unsafe_code)]

use crate::{engine::coordinator::Snapshot, Capture, StorageError};
use codex_accounts_vault::{
    validate_json_object, Identity, Resource, ResourceId, ResourceShape, MAX_RESOURCE_BYTES,
};
use serde::{de::IgnoredAny, Deserialize, Deserializer};
use zeroize::Zeroizing;

const MAX_TOKEN_BYTES: usize = 64 * 1024;
const AUTH_SCHEMA: u32 = 1;

// No Debug implementation exposes the retained tokens or identity strings.
// Application-owned buffers are wiped; serde's internal escape scratch has the
// same limitation documented by the shared strict JSON validator.
struct Secret(Zeroizing<String>);
impl<'de> Deserialize<'de> for Secret {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Visitor;
        impl serde::de::Visitor<'_> for Visitor {
            type Value = Secret;
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("a bounded string")
            }
            fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Secret, E> {
                if value.len() > MAX_TOKEN_BYTES {
                    return Err(E::custom("string limit"));
                }
                Ok(Secret(Zeroizing::new(value.to_owned())))
            }
        }
        deserializer.deserialize_str(Visitor)
    }
}
impl Secret {
    fn text(&self) -> &str {
        &self.0
    }
    fn take(mut self) -> String {
        std::mem::take(&mut *self.0)
    }
}

#[derive(Deserialize)]
struct Auth {
    auth_mode: Option<Secret>,
    #[serde(rename = "OPENAI_API_KEY")]
    api_key: Option<IgnoredAny>,
    agent_identity: Option<IgnoredAny>,
    personal_access_token: Option<IgnoredAny>,
    bedrock_api_key: Option<IgnoredAny>,
    bedrock_access_keys: Option<IgnoredAny>,
    tokens: Tokens,
}
#[derive(Deserialize)]
struct Tokens {
    id_token: Secret,
    access_token: Secret,
    refresh_token: Secret,
    account_id: Option<Secret>,
}
#[derive(Deserialize)]
struct Claims {
    iss: Secret,
    sub: Secret,
    #[serde(rename = "https://api.openai.com/auth")]
    auth: ChatGptClaims,
}
#[derive(Deserialize)]
struct ChatGptClaims {
    chatgpt_account_id: Secret,
}
#[derive(Deserialize)]
struct Header {
    alg: Secret,
}

/// Takes ownership so rejected input is wiped too. Original successful bytes are
/// preserved, never reserialized. This is not a verified credential acceptance.
pub(crate) fn snapshot(bytes: Vec<u8>) -> Result<Snapshot, StorageError> {
    let mut bytes = Zeroizing::new(bytes);
    let (_, identity) = parse_auth(&bytes)?;
    Ok(Snapshot {
        identity: Some(identity),
        schema: AUTH_SCHEMA,
        resources: vec![Resource::present(
            ResourceId::new(0)?,
            std::mem::take(&mut *bytes),
        )?],
    })
}

fn parse_auth(bytes: &[u8]) -> Result<(Auth, Identity), StorageError> {
    if bytes.len() > MAX_RESOURCE_BYTES {
        return Err(StorageError::InputLimit);
    }
    validate_json_object(bytes)?;
    let auth: Auth = serde_json::from_slice(bytes).map_err(|_| StorageError::InvalidData)?;
    if auth
        .auth_mode
        .as_ref()
        .is_some_and(|mode| mode.text() != "chatgpt")
        || auth.api_key.is_some()
        || auth.agent_identity.is_some()
        || auth.personal_access_token.is_some()
        || auth.bedrock_api_key.is_some()
        || auth.bedrock_access_keys.is_some()
    {
        return Err(StorageError::InvalidData);
    }
    let id_payload = jwt_payload(auth.tokens.id_token.text())?;
    let claims: Claims =
        serde_json::from_slice(&id_payload).map_err(|_| StorageError::InvalidData)?;
    // Access tokens are JWTs in official TokenData; do not retain their claims.
    let _access_payload = jwt_payload(auth.tokens.access_token.text())?;
    let refresh = auth.tokens.refresh_token.text();
    if refresh.is_empty() || refresh.trim() != refresh || refresh.chars().any(char::is_control) {
        return Err(StorageError::InvalidData);
    }
    if auth
        .tokens
        .account_id
        .as_ref()
        .is_some_and(|id| id.text() != claims.auth.chatgpt_account_id.text())
    {
        return Err(StorageError::IdentityMismatch);
    }
    let identity = Identity::new(
        claims.iss.take(),
        claims.sub.take(),
        claims.auth.chatgpt_account_id.take(),
    )?;
    Ok((auth, identity))
}

/// Only the existing access token leaves this parser. Refresh tokens never reach
/// the usage transport, and all input remains borrowed from zeroizing storage.
#[cfg(all(windows, target_arch = "x86_64"))]
pub(crate) fn usage_credentials(
    bytes: &[u8],
) -> Result<(Zeroizing<String>, Identity), StorageError> {
    let (auth, identity) = parse_auth(bytes)?;
    Ok((Zeroizing::new(auth.tokens.access_token.take()), identity))
}

pub(crate) fn capture(bytes: Vec<u8>) -> Result<Capture, StorageError> {
    let parsed = snapshot(bytes)?;
    Capture::new(
        parsed.identity.ok_or(StorageError::InvalidData)?,
        parsed.schema,
        vec![(ResourceId::new(0)?, ResourceShape::JsonObject, true)],
        parsed.resources,
    )
}

fn jwt_payload(jwt: &str) -> Result<Zeroizing<Vec<u8>>, StorageError> {
    if jwt.len() > MAX_TOKEN_BYTES {
        return Err(StorageError::InputLimit);
    }
    let mut parts = jwt.split('.');
    let header = decode(parts.next().ok_or(StorageError::InvalidData)?)?;
    let payload = decode(parts.next().ok_or(StorageError::InvalidData)?)?;
    let _signature = decode(parts.next().ok_or(StorageError::InvalidData)?)?;
    if parts.next().is_some() {
        return Err(StorageError::InvalidData);
    }
    validate_json_object(&header)?;
    validate_json_object(&payload)?;
    let header: Header = serde_json::from_slice(&header).map_err(|_| StorageError::InvalidData)?;
    if header.alg.text().is_empty()
        || header.alg.text().eq_ignore_ascii_case("none")
        || header
            .alg
            .text()
            .chars()
            .any(|c| c.is_control() || c.is_whitespace())
    {
        return Err(StorageError::InvalidData);
    }
    Ok(payload)
}

/// Canonical URL-safe base64 without padding, as used by the official decoder.
/// Reject nonzero unused bits as well as bad alphabet, size and length.
fn decode(encoded: &str) -> Result<Zeroizing<Vec<u8>>, StorageError> {
    if encoded.is_empty() || encoded.len() > MAX_TOKEN_BYTES || encoded.len() % 4 == 1 {
        return Err(StorageError::InvalidData);
    }
    let mut output = Zeroizing::new(Vec::with_capacity(encoded.len() * 3 / 4));
    let mut accumulator = 0u32;
    let mut bits = 0u32;
    for byte in encoded.bytes() {
        let value = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'-' => 62,
            b'_' => 63,
            _ => return Err(StorageError::InvalidData),
        };
        accumulator = (accumulator << 6) | u32::from(value);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            output.push((accumulator >> bits) as u8);
            accumulator &= (1 << bits) - 1;
        }
    }
    if accumulator != 0 {
        return Err(StorageError::InvalidData);
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encode(bytes: &[u8]) -> String {
        const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
        let mut result = String::new();
        let mut accumulator = 0u32;
        let mut bits = 0u32;
        for byte in bytes {
            accumulator = (accumulator << 8) | u32::from(*byte);
            bits += 8;
            while bits >= 6 {
                bits -= 6;
                result.push(ALPHABET[((accumulator >> bits) & 63) as usize] as char);
            }
            accumulator &= (1 << bits) - 1;
        }
        if bits != 0 {
            result.push(ALPHABET[(accumulator << (6 - bits)) as usize] as char);
        }
        result
    }
    fn jwt(payload: &str) -> String {
        format!(
            "{}.{}.AA",
            encode(br#"{"alg":"RS256"}"#),
            encode(payload.as_bytes())
        )
    }
    fn fixture(claims: &str) -> Vec<u8> {
        serde_json::to_vec(&serde_json::json!({
            "auth_mode":"chatgpt", "OPENAI_API_KEY":null,
            "tokens":{"id_token":jwt(claims),"access_token":jwt("{}"),
                "refresh_token":"synthetic-refresh", "account_id":"workspace"}
        }))
        .unwrap()
    }
    const CLAIMS: &str = r#"{"iss":"synthetic-issuer","sub":"subject","email":"ignored@example.invalid","https://api.openai.com/auth":{"chatgpt_account_id":"workspace"}}"#;

    #[test]
    fn preserves_bytes_and_stable_identity_and_capture_rules() {
        let bytes = fixture(CLAIMS);
        let parsed = snapshot(bytes.clone()).unwrap();
        assert_eq!(
            parsed.identity.as_ref().unwrap().components(),
            ("synthetic-issuer", "subject", "workspace")
        );
        assert_eq!(parsed.resources[0].as_bytes(), Some(bytes.as_slice()));
        let captured = capture(bytes).unwrap();
        assert_eq!(captured.schema, 1);
        assert_eq!(
            captured.rules,
            vec![(ResourceId::new(0).unwrap(), ResourceShape::JsonObject, true)]
        );
        let legacy = String::from_utf8(fixture(CLAIMS))
            .unwrap()
            .replace("\"auth_mode\":\"chatgpt\",", "")
            .replace("\"account_id\":\"workspace\",", "");
        assert!(snapshot(legacy.into_bytes()).is_ok());
    }

    #[test]
    fn rejects_unsupported_modes_missing_tokens_and_identity_mismatch() {
        for mode in [
            "apikey",
            "chatgptAuthTokens",
            "personalAccessToken",
            "headers",
            "unknown",
        ] {
            let input = String::from_utf8(fixture(CLAIMS)).unwrap().replace(
                "\"auth_mode\":\"chatgpt\"",
                &format!("\"auth_mode\":\"{mode}\""),
            );
            assert!(matches!(
                snapshot(input.into_bytes()),
                Err(StorageError::InvalidData)
            ));
        }
        assert!(snapshot(br#"{"tokens":{}}"#.to_vec()).is_err());
        let claims = CLAIMS.replace("workspace", "other-workspace");
        assert!(matches!(
            snapshot(fixture(&claims)),
            Err(StorageError::IdentityMismatch)
        ));
        let input = String::from_utf8(fixture(CLAIMS)).unwrap().replace(
            "\"OPENAI_API_KEY\":null",
            "\"OPENAI_API_KEY\":\"synthetic-key\"",
        );
        assert!(snapshot(input.into_bytes()).is_err());
    }

    #[test]
    fn rejects_duplicates_malformed_jwts_and_excessive_input() {
        assert!(snapshot(fixture(&CLAIMS.replace(
            "\"sub\":\"subject\"",
            "\"sub\":\"subject\",\"sub\":\"other\""
        )))
        .is_err());
        let input = String::from_utf8(fixture(CLAIMS)).unwrap().replacen(
            "{",
            "{\"auth_mode\":\"chatgpt\",",
            1,
        );
        assert!(snapshot(input.into_bytes()).is_err());
        for jwt in [
            "a.b.c",
            "e30.e30.AB",
            "e30.e30.AA.extra",
            "e30=.e30.AA",
            ".e30.AA",
        ] {
            assert!(jwt_payload(jwt).is_err());
        }
        assert!(matches!(
            snapshot(vec![b' '; MAX_RESOURCE_BYTES + 1]),
            Err(StorageError::InputLimit)
        ));
        assert_eq!(&*decode("Zg").unwrap(), b"f");
        assert_eq!(&*decode("Zm9v").unwrap(), b"foo");
    }
}
