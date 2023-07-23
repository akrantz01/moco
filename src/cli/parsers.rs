use clap::{
    builder::{NonEmptyStringValueParser, StyledStr, TypedValueParser},
    error::{ContextKind, ContextValue, ErrorKind},
    Arg, Command, Error,
};
use std::{ffi::OsStr, time::Duration};
use url::Host;

/// Parse as a non-empty string
pub fn string() -> NonEmptyStringValueParser {
    NonEmptyStringValueParser::default()
}

/// Parse a duration with support for hours (h), minutes (m), and seconds (s) suffixes
pub fn duration() -> DurationValueParser {
    DurationValueParser::default()
}

/// Parse a domain
pub fn domain() -> DomainValueParser {
    DomainValueParser::default()
}

#[derive(Clone, Debug, Default)]
pub struct DurationValueParser {
    inner: NonEmptyStringValueParser,
}

impl TypedValueParser for DurationValueParser {
    type Value = Duration;

    fn parse_ref(
        &self,
        cmd: &Command,
        arg: Option<&Arg>,
        value: &OsStr,
    ) -> Result<Self::Value, Error> {
        let raw = self.inner.parse_ref(cmd, arg, value)?;

        if raw == "0" {
            return Ok(Duration::from_secs(0));
        }

        let mut chars = raw.chars();

        let mut seconds = 0u64;
        'outer: loop {
            let mut n = 0u64;
            for c in &mut chars {
                match c {
                    '0'..='9' => n = n * 10 + (c as u64 - '0' as u64),
                    c if c.is_whitespace() => {}
                    'h' => {
                        seconds += n * 60 * 60;
                        continue 'outer;
                    }
                    'm' => {
                        seconds += n * 60;
                        continue 'outer;
                    }
                    's' => {
                        seconds += n;
                        continue 'outer;
                    }
                    _ => {
                        return Err(validation_error(
                            cmd,
                            arg,
                            raw,
                            format!("unknown unit {c:?} — valid units are 'h', 'm', and 's'"),
                        ));
                    }
                }
            }

            seconds += n;
            break;
        }

        Ok(Duration::from_secs(seconds))
    }
}

#[derive(Clone, Debug, Default)]
pub struct DomainValueParser {
    inner: NonEmptyStringValueParser,
}

impl TypedValueParser for DomainValueParser {
    type Value = String;

    fn parse_ref(
        &self,
        cmd: &Command,
        arg: Option<&Arg>,
        value: &OsStr,
    ) -> Result<Self::Value, Error> {
        let raw = self.inner.parse_ref(cmd, arg, value)?;

        let host = Host::parse(&raw).map_err(|e| validation_error(cmd, arg, raw.clone(), e))?;
        match host {
            Host::Domain(d) => Ok(d),
            Host::Ipv4(_) | Host::Ipv6(_) => Err(validation_error(
                cmd,
                arg,
                raw,
                "IP addresses cannot be used for Lemmy servers",
            )),
        }
    }
}

fn validation_error(
    cmd: &Command,
    arg: Option<&Arg>,
    value: String,
    message: impl std::fmt::Display,
) -> Error {
    let arg = arg
        .map(|a| a.to_string())
        .unwrap_or_else(|| "...".to_owned());

    let mut error = Error::new(ErrorKind::ValueValidation).with_cmd(cmd);
    error.insert(ContextKind::InvalidArg, ContextValue::String(arg));
    error.insert(ContextKind::InvalidValue, ContextValue::String(value));

    let message = StyledStr::from(format!("  reason: {message}"));
    error.insert(ContextKind::Usage, ContextValue::StyledStr(message));

    error
}
