use super::types::ServerError;
use std::{
    fmt::{self, Formatter},
    io,
};

macro_rules! error {
    (
        $( #[$meta:meta] )*
        $name:ident
        $(
            $( #[$imeta:meta] )*
            $error:ident => $message:expr
        ),* $(,)?
    ) => {
        $( #[$meta ])*
        #[derive(Debug)]
        pub enum $name {
            $(
                $( #[$imeta] )*
                $error,
            )*
            /// An unexpected server error occurred
            ServerError(ServerError),
            /// An error that occurred while processing the request
            Request(reqwest::Error),
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                match self {
                    $( Self::$error => write!(f, $message), )*
                    Self::ServerError(ServerError { error, message }) => {
                        write!(f, "{error}")?;
                        if let Some(message) = message {
                            write!(f, ": {message}")?;
                        }

                        Ok(())
                    }
                    Self::Request(_) => write!(f, "failed to complete the request"),
                }
            }
        }

        impl std::error::Error for $name {
            fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
                match self {
                    Self::Request(err) => Some(err),
                    _ => None,
                }
            }
        }

        impl From<reqwest::Error> for $name {
            fn from(err: reqwest::Error) -> Self {
                // TODO: separate out certain errors
                Self::Request(err)
            }
        }
    };
}

/// Errors that can occur when initiating the connection to the Lemmy instance
#[derive(Debug)]
pub enum ConnectError {
    /// Federation is not supported on this instance
    FederationNotSupported,
    /// The server is not a Lemmy instance
    NotLemmyInstance,
    /// An error that occurred while processing the request
    Request(reqwest::Error),
    /// Invalid additional certificate
    ///
    /// Only returned if a certificate from the `EXTRA_CERTIFICATE_PATHS` environment variable is
    /// invalid
    InvalidCertificate(reqwest::Error),
    /// Could not read the custom certificate
    ///
    /// Only returned if a path in the `EXTRA_CERTIFICATE_PATHS` environment variable could not be
    /// read
    CertificateRead(io::Error),
}

impl std::error::Error for ConnectError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Request(err) => Some(err),
            Self::InvalidCertificate(err) => Some(err),
            Self::CertificateRead(err) => Some(err),
            _ => None,
        }
    }
}

impl fmt::Display for ConnectError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::FederationNotSupported => write!(f, "federation not supported on this instance"),
            Self::NotLemmyInstance => {
                write!(f, "the requested URL does not resolve to a Lemmy instance")
            }
            Self::Request(_) => write!(f, "failed to complete the request"),
            Self::InvalidCertificate(_) => write!(f, "invalid extra certificate"),
            Self::CertificateRead(_) => write!(f, "could not read extra certificate path"),
        }
    }
}

impl From<reqwest::Error> for ConnectError {
    fn from(err: reqwest::Error) -> ConnectError {
        Self::Request(err)
    }
}

impl From<io::Error> for ConnectError {
    fn from(err: io::Error) -> ConnectError {
        Self::CertificateRead(err)
    }
}

error!(
    LoginError
        /// The provided credentials are incorrect
        IncorrectCredentials => "invalid username or password",
        /// The user's email is not verified
        EmailNotVerified => "user's email not verified",
);

error!(
    FetchError
        /// The requested object couldn't be found
        NotFound => "not found",
);
