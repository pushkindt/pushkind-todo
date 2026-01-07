//! Strongly-typed value objects used by domain entities.
//!
//! These wrappers enforce basic invariants (e.g., positive identifiers,
//! normalized/validated email) so that once a value reaches the domain layer it
//! can be treated as trusted.
use std::ops::Deref;

use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use thiserror::Error;
use validator::ValidateEmail;

/// Errors produced when attempting to construct a constrained value object.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum TypeConstraintError {
    /// Provided identifier is zero or negative.
    #[error("id must be greater than zero")]
    NonPositiveId,
    /// Provided email failed format validation.
    #[error("invalid email address")]
    InvalidEmail,
    /// Provided string contained no non-whitespace characters.
    #[error("value cannot be empty")]
    EmptyString,

    #[error("invalid value for task priority")]
    InvalidTaskPriority,

    #[error("invalid value for task status")]
    InvalidTaskStatus,

    #[error("invalid value HTML text")]
    InvalidHtml,
}

/// Macro to generate lightweight newtypes for positive identifiers.
macro_rules! id_newtype {
    ($name:ident, $doc:expr) => {
        #[doc = $doc]
        #[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
        pub struct $name(i32);

        impl $name {
            /// Creates a new identifier ensuring it is greater than zero.
            pub fn new(value: i32) -> Result<Self, TypeConstraintError> {
                if value > 0 {
                    Ok(Self(value))
                } else {
                    Err(TypeConstraintError::NonPositiveId)
                }
            }

            /// Returns the raw `i32` backing this identifier.
            pub const fn get(self) -> i32 {
                self.0
            }
        }

        impl Display for $name {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl TryFrom<i32> for $name {
            type Error = TypeConstraintError;

            fn try_from(value: i32) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }

        impl From<$name> for i32 {
            fn from(value: $name) -> Self {
                value.0
            }
        }
    };
}

id_newtype!(UserId, "Unique identifier for a user.");
id_newtype!(HubId, "Unique identifier for a hub.");
id_newtype!(TaskId, "Unique identifier for a task.");
id_newtype!(TaskEventId, "Unique identifier for a task event.");
id_newtype!(ClientId, "Unique identifier for a customer.");

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
/// Lower-cased and validated email address.
pub struct UserEmail(String);

impl UserEmail {
    /// Validates and normalizes an email string.
    pub fn new<S: Into<String>>(email: S) -> Result<Self, TypeConstraintError> {
        let normalized = normalize_email(email)?;
        Ok(Self(normalized))
    }

    /// Borrow the email as a `&str`.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Convert into the owned inner `String`.
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl Display for UserEmail {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for UserEmail {
    type Error = TypeConstraintError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<&str> for UserEmail {
    type Error = TypeConstraintError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<UserEmail> for String {
    fn from(value: UserEmail) -> Self {
        value.0
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
/// Wrapper for non-empty, trimmed strings.
pub struct NonEmptyString(String);

impl NonEmptyString {
    /// Trims whitespace and rejects empty inputs.
    pub fn new<S: Into<String>>(value: S) -> Result<Self, TypeConstraintError> {
        let trimmed = value.into().trim().to_string();
        if trimmed.is_empty() {
            return Err(TypeConstraintError::EmptyString);
        }
        Ok(Self(trimmed))
    }

    /// Borrow the inner string.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume the wrapper returning the owned string.
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl Display for NonEmptyString {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for NonEmptyString {
    type Error = TypeConstraintError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<&str> for NonEmptyString {
    type Error = TypeConstraintError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<NonEmptyString> for String {
    fn from(value: NonEmptyString) -> Self {
        value.0
    }
}

macro_rules! non_empty_string_newtype {
    ($name:ident, $doc:expr) => {
        #[doc = $doc]
        #[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(String);

        impl $name {
            /// Constructs a trimmed, non-empty value.
            pub fn new<S: Into<String>>(value: S) -> Result<Self, TypeConstraintError> {
                let inner = NonEmptyString::new(value)?;
                Ok(Self(inner.into_inner()))
            }

            /// Borrow the value as a string slice.
            pub fn as_str(&self) -> &str {
                &self.0
            }

            /// Consume the wrapper and return the owned string.
            pub fn into_inner(self) -> String {
                self.0
            }
        }

        impl Deref for $name {
            type Target = str;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl Display for $name {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl TryFrom<String> for $name {
            type Error = TypeConstraintError;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }

        impl TryFrom<&str> for $name {
            type Error = TypeConstraintError;

            fn try_from(value: &str) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }

        impl From<$name> for String {
            fn from(value: $name) -> Self {
                value.0
            }
        }
    };
}

non_empty_string_newtype!(UserName, "User name wrapper enforcing non-empty values.");

non_empty_string_newtype!(TaskTitle, "Task title wrapper enforcing non-empty values.");

non_empty_string_newtype!(TaskTrack, "Task track wrapper enforcing non-empty values.");

non_empty_string_newtype!(
    ClientName,
    "Customer name wrapper enforcing non-empty values."
);

non_empty_string_newtype!(
    PublicId,
    "Customer name wrapper enforcing non-empty values."
);

macro_rules! html_string_newtype {
    ($name:ident, $doc:expr) => {
        #[doc = $doc]
        #[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
        pub struct $name(String);

        impl $name {
            pub fn new<S: Into<String>>(value: S) -> Result<Self, TypeConstraintError> {
                let sanitized = ammonia::clean(&value.into());
                let inner = NonEmptyString::new(sanitized)?;
                Ok(Self(inner.into_inner()))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }

            pub fn into_inner(self) -> String {
                self.0
            }
        }

        impl Display for $name {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl From<$name> for String {
            fn from(value: $name) -> Self {
                value.0
            }
        }
    };
}

html_string_newtype!(
    TaskDescription,
    "Task description wrapper for optional HTML content."
);

html_string_newtype!(
    TaskComment,
    "Task comment wrapper for optional HTML content."
);

/// Normalizes and validates an email string.
fn normalize_email<S: Into<String>>(email: S) -> Result<String, TypeConstraintError> {
    let normalized = email.into().trim().to_lowercase();
    if normalized.validate_email() {
        Ok(normalized)
    } else {
        Err(TypeConstraintError::InvalidEmail)
    }
}
