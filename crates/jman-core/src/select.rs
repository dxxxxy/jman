use anyhow::bail;

use crate::detect::Jdk;
use crate::version;

/// Resolves a user query to a JDK. Accepted forms:
/// - an exact id: `azul-21.0.9`
/// - a version or version prefix: `21`, `1.8`, `17.0.9`
/// - a vendor/name filter with an optional version: `temurin`, `temurin-21`, `zulu@17`
///
/// When several JDKs match, the newest wins.
pub fn resolve<'a>(query: &str, jdks: &'a [Jdk]) -> anyhow::Result<&'a Jdk> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        bail!("empty JDK query");
    }
    if let Some(jdk) = jdks.iter().find(|j| j.id.to_lowercase() == q) {
        return Ok(jdk);
    }

    let (name, ver) = split_query(&q);
    let matches = |j: &&Jdk| {
        let name_ok = name.is_empty() || {
            let id = j.id.to_lowercase();
            let vendor = j.vendor.as_deref().unwrap_or("").to_lowercase();
            id.contains(name) || vendor.contains(name)
        };
        let ver_ok = ver.is_empty() || version::matches_prefix(&j.version, ver);
        name_ok && ver_ok
    };

    match jdks.iter().filter(matches).max_by(|a, b| version::compare(&a.version, &b.version)) {
        Some(jdk) => Ok(jdk),
        None => bail!("no installed JDK matches '{query}' (see `jman list`)"),
    }
}

/// Splits `temurin-21` / `zulu@17` / `21` / `temurin` into (name, version).
fn split_query(q: &str) -> (&str, &str) {
    if q.starts_with(|c: char| c.is_ascii_digit()) {
        return ("", q);
    }
    if let Some(i) = q.rfind(['@', '-', ' ']) {
        let (name, ver) = (&q[..i], &q[i + 1..]);
        if ver.starts_with(|c: char| c.is_ascii_digit()) {
            return (name, ver);
        }
    }
    (q, "")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn jdk(id: &str, version: &str, vendor: &str) -> Jdk {
        Jdk {
            id: id.into(),
            home: id.into(),
            version: version.into(),
            major: version::major(version),
            vendor: Some(vendor.into()),
            arch: None,
            managed: false,
            source: "IntelliJ".into(),
        }
    }

    #[test]
    fn resolves_queries() {
        let jdks = vec![
            jdk("azul-25.0.1", "25.0.1", "Azul Systems, Inc."),
            jdk("temurin-21.0.5", "21.0.5", "Eclipse Adoptium"),
            jdk("azul-21.0.9", "21.0.9", "Azul Systems, Inc."),
            jdk("azul-1.8.0_472", "1.8.0_472", "Azul Systems, Inc."),
            jdk("azul-1.8.0_492", "1.8.0_492", "Azul Systems, Inc."),
        ];
        let id = |q: &str| resolve(q, &jdks).unwrap().id.clone();
        assert_eq!(id("azul-1.8.0_472"), "azul-1.8.0_472");
        assert_eq!(id("8"), "azul-1.8.0_492");
        assert_eq!(id("1.8"), "azul-1.8.0_492");
        assert_eq!(id("21"), "azul-21.0.9");
        assert_eq!(id("temurin"), "temurin-21.0.5");
        assert_eq!(id("temurin-21"), "temurin-21.0.5");
        assert_eq!(id("adoptium@21"), "temurin-21.0.5");
        assert_eq!(id("AZUL"), "azul-25.0.1");
        assert!(resolve("17", &jdks).is_err());
        assert!(resolve("corretto", &jdks).is_err());
    }
}
