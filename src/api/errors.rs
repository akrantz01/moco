use super::types::ServerError;
use std::fmt::{self, Formatter};

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

error!(
    ConnectError
        /// Federation is not supported on this instance
        FederationNotSupported => "federation not supported on this instance",
        /// The server is not a Lemmy instance
        NotLemmyInstance => "the requested URL does not resolve to a Lemmy instance",
);

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
