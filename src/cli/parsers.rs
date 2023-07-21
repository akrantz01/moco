use clap::error::{ContextKind, ContextValue};
use clap::{
    builder::{NonEmptyStringValueParser, TypedValueParser},
    error::ErrorKind,
    Arg, Command, Error,
};
use std::{ffi::OsStr, time::Duration};

/// Parse as a non-empty string
pub fn string() -> NonEmptyStringValueParser {
    NonEmptyStringValueParser::default()
}

/// Parse a duration with support for hours (h), minutes (m), and seconds (s) suffixes
pub fn duration() -> DurationValueParser {
    DurationValueParser::default()
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
            while let Some(c) = chars.next() {
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
                        let arg = arg
                            .map(|a| a.to_string())
                            .unwrap_or_else(|| "...".to_owned());

                        let mut error = Error::new(ErrorKind::ValueValidation).with_cmd(cmd);
                        error.insert(ContextKind::InvalidArg, ContextValue::String(arg));
                        error.insert(ContextKind::InvalidValue, ContextValue::String(raw));
                        error.insert(
                            ContextKind::ValidValue,
                            ContextValue::Strings(vec![String::new()]),
                        );
                        return Err(error);
                    }
                }
            }

            seconds += n;
            break;
        }

        Ok(Duration::from_secs(seconds))
    }
}
