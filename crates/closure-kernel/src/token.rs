//! Session tokens.
//!
//! A token is the handle a player carries from the CLI to the web surface. It
//! names a session on the server; it is not a credential for anything else,
//! and it is deliberately short enough to retype.
//!
//! Format: `CLOSURE-XXXX-XXXX-XXXX` over an unambiguous alphabet (no `0`,
//! `O`, `1`, `I`), which is 15 characters of entropy at ~5.17 bits each,
//! about 77 bits.

use serde::{Deserialize, Serialize};

/// The alphabet: Crockford-style, minus the characters people mistype.
const ALPHABET: &[u8] = b"23456789ABCDEFGHJKLMNPQRSTUVWXYZ";
const GROUPS: usize = 3;
const GROUP_LEN: usize = 4;

/// A session token.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionToken(String);

impl SessionToken {
    /// Mint a token from a source of randomness.
    #[must_use]
    pub fn generate(rng: &mut impl rand::Rng) -> Self {
        let mut s = String::from("CLOSURE");
        for _ in 0..GROUPS {
            s.push('-');
            for _ in 0..GROUP_LEN {
                let i = rng.random_range(0..ALPHABET.len());
                s.push(ALPHABET[i] as char);
            }
        }
        Self(s)
    }

    /// Parse a token, accepting lowercase and missing dashes so that a player
    /// retyping it from a terminal is not punished for either.
    ///
    /// # Errors
    /// If the body is not exactly `GROUPS * GROUP_LEN` characters drawn from
    /// the alphabet.
    pub fn parse(raw: &str) -> crate::Result<Self> {
        let cleaned: String = raw
            .trim()
            .to_ascii_uppercase()
            .strip_prefix("CLOSURE")
            .unwrap_or(&raw.trim().to_ascii_uppercase())
            .chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .collect();

        if cleaned.len() != GROUPS * GROUP_LEN {
            return Err(crate::Error::Token(format!(
                "expected {} characters after the prefix, got {}",
                GROUPS * GROUP_LEN,
                cleaned.len()
            )));
        }
        if let Some(bad) = cleaned.chars().find(|c| !ALPHABET.contains(&(*c as u8))) {
            return Err(crate::Error::Token(format!(
                "character {bad:?} is not in the token alphabet"
            )));
        }
        let grouped: Vec<String> = cleaned
            .as_bytes()
            .chunks(GROUP_LEN)
            .map(|c| String::from_utf8_lossy(c).into_owned())
            .collect();
        Ok(Self(format!("CLOSURE-{}", grouped.join("-"))))
    }

    /// The canonical string form.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SessionToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    fn rng() -> rand_chacha::ChaCha8Rng {
        rand_chacha::ChaCha8Rng::seed_from_u64(20260902)
    }

    #[test]
    fn generated_tokens_round_trip() {
        let mut r = rng();
        for _ in 0..256 {
            let t = SessionToken::generate(&mut r);
            assert_eq!(SessionToken::parse(t.as_str()).unwrap(), t);
        }
    }

    #[test]
    fn parsing_is_forgiving_about_case_and_dashes() {
        let mut r = rng();
        let t = SessionToken::generate(&mut r);
        let mangled = t.as_str().to_ascii_lowercase().replace('-', " ");
        assert_eq!(SessionToken::parse(&mangled).unwrap(), t);
    }

    #[test]
    fn ambiguous_characters_are_rejected() {
        assert!(SessionToken::parse("CLOSURE-OOOO-1111-IIII").is_err());
    }

    #[test]
    fn wrong_length_is_rejected() {
        assert!(SessionToken::parse("CLOSURE-ABC").is_err());
    }
}
