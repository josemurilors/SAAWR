use crate::error::{AwrError, Result};

/// MixMods tutorial: the model/handling name must have at most 7 letters.
pub const SHORT_MAX: usize = 7;

/// Suggests a short name from a friendly name: `"GT R34"` -> `"gtr34"`.
pub fn short_from(name: &str) -> String {
    name.chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|c| c.to_ascii_lowercase())
        .take(SHORT_MAX)
        .collect()
}

/// `taken` answers whether a short is already a game model or another mod.
pub fn check_short(short: &str, taken: impl Fn(&str) -> bool) -> Result<()> {
    let valid = (1..=SHORT_MAX).contains(&short.len())
        && short
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        && short.as_bytes()[0].is_ascii_lowercase();
    if !valid {
        return Err(AwrError::InvalidName(format!(
            "'{short}': use 1 a {SHORT_MAX} letras minúsculas/dígitos, começando por letra"
        )));
    }
    if taken(short) {
        return Err(AwrError::DuplicateShort {
            short: short.into(),
        });
    }
    Ok(())
}

/// Weapon model names: up to 15 characters (tutorial), letters, digits and `_`.
pub fn check_weapon_name(name: &str, taken: impl Fn(&str) -> bool) -> Result<()> {
    let valid = (1..=15).contains(&name.len())
        && name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
        && name.as_bytes()[0].is_ascii_lowercase();
    if !valid {
        return Err(AwrError::InvalidName(format!(
            "'{name}': use 1 a 15 letras minúsculas, dígitos ou _, começando por letra"
        )));
    }
    if taken(name) {
        return Err(AwrError::DuplicateShort { short: name.into() });
    }
    Ok(())
}

/// IDE model names are limited to 23 characters.
pub fn check_part_name(name: &str) -> Result<()> {
    if name.len() > 23 {
        return Err(AwrError::InvalidName(format!(
            "'{name}': peça com nome longo demais (máx. 23); use um nome curto menor"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suggests_and_validates() {
        assert_eq!(short_from("GT R34 Skyline"), "gtr34sk");
        assert!(check_short("gtr34", |_| false).is_ok());
        for bad in ["", "gtr34sky", "GTR", "1abc", "gt-r"] {
            assert!(
                matches!(check_short(bad, |_| false), Err(AwrError::InvalidName(_))),
                "{bad}"
            );
        }
        assert!(matches!(
            check_short("landstal", |_| true),
            Err(AwrError::InvalidName(_))
        ));
        assert!(matches!(
            check_short("infern", |s| s == "infern"),
            Err(AwrError::DuplicateShort { .. })
        ));
    }
}
