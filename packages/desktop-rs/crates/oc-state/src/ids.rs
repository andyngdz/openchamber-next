//! Typed ids, so a session id, a message id and a call id never trade places.

use std::fmt;

macro_rules! string_id {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(String);

        impl $name {
            /// The id as the server spells it.
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl From<&str> for $name {
            fn from(raw: &str) -> Self {
                Self(raw.to_owned())
            }
        }

        impl From<String> for $name {
            fn from(raw: String) -> Self {
                Self(raw)
            }
        }

        impl PartialEq<str> for $name {
            fn eq(&self, other: &str) -> bool {
                self.0 == other
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(&self.0)
            }
        }
    };
}

string_id!(
    /// `ses_…` id of a session.
    SessionId
);
string_id!(
    /// `msg_…` id of a user or assistant message.
    MessageId
);
string_id!(
    /// Id of one tool call, unique inside its assistant message.
    CallId
);
string_id!(
    /// Id of a permission request (`per_…`) or a form.
    RequestId
);
