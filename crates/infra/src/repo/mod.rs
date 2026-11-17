pub mod audit;
pub mod books;
pub mod members;
pub mod users;

/// Escapes `%`, `_` and `\` so user input is matched literally inside ILIKE.
pub fn like_pattern(q: &str) -> String {
    let escaped = q.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_");
    format!("%{escaped}%")
}

#[cfg(test)]
mod tests {
    #[test]
    fn like_pattern_escapes() {
        assert_eq!(super::like_pattern("50%_off"), "%50\\%\\_off%");
    }
}
