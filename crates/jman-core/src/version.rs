use std::cmp::Ordering;

/// Splits a Java version string into numeric components, normalizing the legacy
/// `1.x` scheme so that `1.8.0_472` becomes `[8, 0, 472]`.
pub fn components(version: &str) -> Vec<u32> {
    let mut parts: Vec<u32> = version
        .split(|c: char| !c.is_ascii_digit())
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse().ok())
        .collect();
    if parts.len() > 1 && parts[0] == 1 {
        parts.remove(0);
    }
    parts
}

pub fn major(version: &str) -> u32 {
    components(version).first().copied().unwrap_or(0)
}

pub fn compare(a: &str, b: &str) -> Ordering {
    components(a).cmp(&components(b))
}

/// True when every component of `prefix` matches the start of `version`.
pub fn matches_prefix(version: &str, prefix: &str) -> bool {
    let (v, p) = (components(version), components(prefix));
    !p.is_empty() && v.len() >= p.len() && v[..p.len()] == p[..]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_legacy_versions() {
        assert_eq!(components("1.8.0_472"), vec![8, 0, 472]);
        assert_eq!(major("1.8.0_472"), 8);
        assert_eq!(major("21.0.9"), 21);
        assert_eq!(major("25"), 25);
        assert_eq!(components("21.0.12.1+1"), vec![21, 0, 12, 1, 1]);
    }

    #[test]
    fn compares_and_matches() {
        assert_eq!(compare("1.8.0_492", "1.8.0_472"), Ordering::Greater);
        assert_eq!(compare("21.0.9", "25.0.1"), Ordering::Less);
        assert!(matches_prefix("1.8.0_472", "8"));
        assert!(matches_prefix("1.8.0_472", "1.8"));
        assert!(matches_prefix("17.0.9", "17.0"));
        assert!(!matches_prefix("17.0.9", "17.1"));
        assert!(!matches_prefix("21.0.9", "2"));
    }
}
