//! Shared auth helpers for the relay.

/// Constant-time byte comparison, so password checks don't leak length-prefix
/// timing. Used by both the HTTP visitor path and the QUIC attach path.
#[inline]
pub(crate) fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// Check a visitor-supplied password against a tunnel's expected password.
/// Public tunnels (`expected == None`) accept everyone.
pub(crate) fn password_ok(expected: Option<&str>, supplied: Option<&str>) -> bool {
    match expected {
        None => true,
        Some(pw) => supplied.is_some_and(|s| constant_time_eq(s.as_bytes(), pw.as_bytes())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constant_time_eq_matches() {
        assert!(constant_time_eq(b"secret", b"secret"));
        assert!(!constant_time_eq(b"secret", b"secreT"));
        assert!(!constant_time_eq(b"secret", b"secre"));
        assert!(constant_time_eq(b"", b""));
    }

    #[test]
    fn public_tunnel_accepts_anyone() {
        assert!(password_ok(None, None));
        assert!(password_ok(None, Some("anything")));
    }

    #[test]
    fn private_tunnel_requires_match() {
        assert!(password_ok(Some("hunter2"), Some("hunter2")));
        assert!(!password_ok(Some("hunter2"), Some("wrong")));
        assert!(!password_ok(Some("hunter2"), None));
    }
}
