#[derive(Clone, Debug)]
struct VersionRange {
    source: String,
    alternatives: Vec<semver::VersionReq>,
}
impl VersionRange {
    fn parse(text: &str) -> Result<Self, String> {
        if text.is_empty() || text.len() > 256 {
            return Err("version range must contain 1..256 bytes".into());
        }
        let mut alternatives = Vec::new();
        for arm in text.split("||") {
            let arm = arm.trim();
            if arm.is_empty() {
                return Err("empty version range alternative".into());
            }
            let normalized = if let Some((low, high)) = arm.split_once(" - ") {
                let lower = Self::partial(low.trim())?;
                let upper = Self::partial(high.trim())?;
                let count = high.trim().split('.').count();
                if count == 1 {
                    format!(
                        ">={lower}, <{}.0.0",
                        upper.major.checked_add(1).ok_or("version overflow")?
                    )
                } else if count == 2 {
                    format!(
                        ">={lower}, <{}.{}.0",
                        upper.major,
                        upper.minor.checked_add(1).ok_or("version overflow")?
                    )
                } else {
                    format!(">={lower}, <={upper}")
                }
            } else {
                let mut terms = Vec::new();
                let mut prefix = String::new();
                for part in arm.replace(',', " ").split_whitespace() {
                    if [">", ">=", "<", "<=", "=", "^", "~"].contains(&part) {
                        if !prefix.is_empty() {
                            return Err("missing comparator version".into());
                        }
                        prefix = part.into();
                    } else {
                        terms.push(format!("{prefix}{part}"));
                        prefix.clear();
                    }
                }
                if !prefix.is_empty() {
                    return Err("missing comparator version".into());
                }
                terms.join(", ")
            };
            alternatives.push(
                semver::VersionReq::parse(&normalized)
                    .map_err(|e| format!("invalid version range: {e}"))?,
            );
        }
        Ok(Self {
            source: text.into(),
            alternatives,
        })
    }
    fn partial(text: &str) -> Result<semver::Version, String> {
        let dots = text.split('.').count();
        semver::Version::parse(&format!(
            "{text}{}",
            match dots {
                1 => ".0.0",
                2 => ".0",
                _ => "",
            }
        ))
        .map_err(|e| e.to_string())
    }
    fn matches(&self, version: &semver::Version) -> bool {
        self.alternatives.iter().any(|r| r.matches(version))
    }
}
impl std::fmt::Display for VersionRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.source)
    }
}
#[cfg(test)]
mod range_tests {
    use super::*;
    #[test]
    fn unions_intersections_and_hyphens() {
        let matches = |r: &str, v: &str| {
            VersionRange::parse(r)
                .unwrap()
                .matches(&semver::Version::parse(v).unwrap())
        };
        assert!(matches("^1 || ^3", "3.2.0"));
        assert!(!matches("^1 || ^3", "2.0.0"));
        assert!(matches(">= 1.2 <2", "1.9.0"));
        assert!(!matches(">=1.2 <2", "2.0.0"));
        assert!(matches("1.2 - 2.3", "2.3.99"));
        assert!(!matches("1.2 - 2.3", "2.4.0"));
        assert!(!matches("*", "1.0.0-beta.1"));
        assert!(VersionRange::parse("^1 ||").is_err());
    }
}
