//! Pairing-token interceptors for both ends of the scope link.

use tonic::metadata::{errors::InvalidMetadataValue, Ascii, MetadataValue};
use tonic::service::Interceptor;
use tonic::{Request, Status};

/// Compares all bytes of both tokens before deciding whether they match.
///
/// Iteration depends only on lengths. A length difference and every byte's XOR
/// are folded into one result; neither a matching prefix nor a differing byte
/// can finish the comparison early. Black boxes keep byte reads and the final
/// decision opaque to optimizations that could short-circuit the reduction.
pub fn tokens_match(a: &[u8], b: &[u8]) -> bool {
    let mut difference = a.len() ^ b.len();
    for index in 0..a.len().max(b.len()) {
        let left = std::hint::black_box(a.get(index).copied().unwrap_or(0));
        let right = std::hint::black_box(b.get(index).copied().unwrap_or(0));
        difference |= usize::from(left ^ right);
    }
    std::hint::black_box(difference) == 0
}

/// Server interceptor requiring `authorization: Bearer <session token>`.
///
/// Install on the generated server so every scope-link call is paired before
/// the service runs. Authentication failures never include the supplied token.
#[derive(Clone)]
pub struct TokenInterceptor {
    token: Vec<u8>,
}

impl TokenInterceptor {
    /// Uses the session's configured token, without logging or formatting it.
    pub fn new(token: impl AsRef<[u8]>) -> Self {
        Self {
            token: token.as_ref().to_vec(),
        }
    }
}

impl Interceptor for TokenInterceptor {
    fn call(&mut self, request: Request<()>) -> Result<Request<()>, Status> {
        let supplied = request
            .metadata()
            .get("authorization")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.strip_prefix("Bearer "));
        match supplied {
            Some(token) if tokens_match(token.as_bytes(), &self.token) => Ok(request),
            _ => Err(Status::unauthenticated(
                "scope link requires the session token",
            )),
        }
    }
}

/// Client interceptor adding the session token to every scope-link call.
#[derive(Clone)]
pub struct BearerToken {
    value: MetadataValue<Ascii>,
}

impl BearerToken {
    /// Prepares a sensitive authorization header, rejecting invalid metadata.
    pub fn new(token: impl AsRef<str>) -> Result<Self, InvalidMetadataValue> {
        let mut value: MetadataValue<Ascii> = format!("Bearer {}", token.as_ref()).parse()?;
        value.set_sensitive(true);
        Ok(Self { value })
    }
}

impl Interceptor for BearerToken {
    fn call(&mut self, mut request: Request<()>) -> Result<Request<()>, Status> {
        request
            .metadata_mut()
            .insert("authorization", self.value.clone());
        Ok(request)
    }
}

#[cfg(test)]
mod tests {
    use super::{tokens_match, BearerToken};
    use tonic::service::Interceptor;
    use tonic::Request;

    #[test]
    fn compares_equal_last_byte_and_different_lengths() {
        assert!(tokens_match(b"test-token", b"test-token"));
        assert!(!tokens_match(b"test-token", b"test-tokem"));
        assert!(!tokens_match(b"test-token", b"test-token-longer"));
        assert!(!tokens_match(b"test-token-longer", b"test-token"));
        assert!(!tokens_match(b"test-token", b"best-token"));
        assert!(!tokens_match(b"test-token", b"test-token\0"));
        assert!(tokens_match(b"", b""));
        assert!(!tokens_match(b"", b"\0"));
    }

    #[test]
    fn bearer_metadata_is_sensitive_and_rejects_invalid_header_bytes() {
        let mut bearer = BearerToken::new("test-token").unwrap();
        let request = bearer.call(Request::new(())).unwrap();
        let header = request.metadata().get("authorization").unwrap();
        assert_eq!(header, "Bearer test-token");
        assert!(header.is_sensitive());
        assert!(BearerToken::new("bad\ntoken").is_err());
    }
}
