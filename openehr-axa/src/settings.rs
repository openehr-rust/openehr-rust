//! Optional settings that turn a protection on (`db:PR12.24`, `db:D-30`).
//!
//! One reader, so the four states, absent, empty, invalid and configured, are
//! told apart the same way for every such setting. Before it, the access log
//! treated *every* read error as "auditing off", and the token verifier used
//! `.ok()` on its issuer, audience and implicit assertion. A value that was
//! present but not Unicode read as absent, and so turned the protection off.
//!
//! Each reader takes a `lookup` rather than reading the process environment,
//! so tests can supply any value, including one that is not Unicode, without
//! touching global state. `from_env` callers pass [`std::env::var_os`].

use std::ffi::OsString;

/// A claim constraint: absent and empty are both `None`; anything else must
/// be Unicode.
///
/// Empty means absent because that is what a Compose `${VAR:-}` default
/// produces, and because an empty issuer or audience matches no real token:
/// treating it as configured refused every request, the opposite failure from
/// the one `.ok()` had.
///
/// # Errors
///
/// A message naming `name`, never quoting the value, when the value is not
/// Unicode.
pub fn claim(
    name: &str,
    lookup: &impl Fn(&str) -> Option<OsString>,
) -> Result<Option<String>, String> {
    match lookup(name) {
        None => Ok(None),
        Some(raw) => {
            let text = raw.into_string().map_err(|_| {
                format!("{name} is set but is not valid Unicode; refusing to start")
            })?;
            Ok((!text.trim().is_empty()).then_some(text))
        }
    }
}

/// A path setting: absent is `None`, empty is refused, and anything else is
/// the path, read as an operating-system path and not as text.
///
/// # Errors
///
/// A message naming `name` when it is set to the empty string.
pub fn path(
    name: &str,
    lookup: &impl Fn(&str) -> Option<OsString>,
) -> Result<Option<std::path::PathBuf>, String> {
    match lookup(name) {
        None => Ok(None),
        Some(raw) if raw.is_empty() => Err(format!(
            "{name} is set but empty; give it a path, or unset it to turn this off"
        )),
        Some(raw) => Ok(Some(std::path::PathBuf::from(raw))),
    }
}

#[cfg(test)]
mod tests {
    use super::{claim, path};
    use std::ffi::OsString;

    fn only(value: Option<OsString>) -> impl Fn(&str) -> Option<OsString> {
        move |_| value.clone()
    }

    #[cfg(unix)]
    fn not_unicode() -> OsString {
        use std::os::unix::ffi::OsStringExt as _;
        OsString::from_vec(vec![0x66, 0xff, 0x6f])
    }

    #[test]
    fn a_claim_is_absent_empty_configured_or_refused() {
        assert_eq!(claim("X", &only(None)), Ok(None));
        assert_eq!(claim("X", &only(Some("".into()))), Ok(None));
        assert_eq!(claim("X", &only(Some("  ".into()))), Ok(None));
        assert_eq!(
            claim("X", &only(Some("aud-1".into()))),
            Ok(Some("aud-1".to_owned()))
        );
        #[cfg(unix)]
        {
            let error = claim("X", &only(Some(not_unicode()))).expect_err("not Unicode");
            assert!(
                error.starts_with("X is set but is not valid Unicode"),
                "{error}"
            );
        }
    }

    #[test]
    fn a_path_is_absent_refused_when_empty_and_otherwise_any_os_path() {
        assert_eq!(path("P", &only(None)), Ok(None));
        let error = path("P", &only(Some("".into()))).expect_err("empty");
        assert!(error.contains("unset it"), "{error}");
        assert_eq!(
            path("P", &only(Some("/var/log/a".into()))),
            Ok(Some("/var/log/a".into()))
        );
        #[cfg(unix)]
        assert_eq!(
            path("P", &only(Some(not_unicode()))),
            Ok(Some(not_unicode().into()))
        );
    }
}
