//! Strongly-typed value objects used by domain entities.
//!
//! These wrappers enforce basic invariants (e.g., positive identifiers,
//! normalized/validated email) so that once a value reaches the domain layer it
//! can be treated as trusted.
use std::{ops::Deref, str::FromStr};

use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use thiserror::Error;
use uuid::Uuid;
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

    /// Provided uuid failed format validation.
    #[error("invalid uuid value")]
    InvalidUuid,
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
        let mut normalized = email.into();
        let trimmed = normalized.trim();
        if trimmed.len() != normalized.len() {
            normalized = trimmed.to_string();
        }
        if normalized.is_ascii() {
            normalized.make_ascii_lowercase();
        } else {
            normalized = normalized.to_lowercase();
        }
        if normalized.validate_email() {
            Ok(Self(normalized))
        } else {
            Err(TypeConstraintError::InvalidEmail)
        }
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
    ClientPublicId,
    "Customer public identifier wrapper enforcing non-empty values."
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TaskPublicId(Uuid);

impl TaskPublicId {
    /// Generate a new random public ID
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Parse from raw bytes (DB boundary)
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, TypeConstraintError> {
        Ok(Self(
            Uuid::from_slice(bytes).map_err(|_| TypeConstraintError::InvalidUuid)?,
        ))
    }

    /// Convert to raw bytes (DB boundary)
    pub fn as_bytes(&self) -> &[u8; 16] {
        self.0.as_bytes()
    }
}

impl Display for TaskPublicId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for TaskPublicId {
    type Err = TypeConstraintError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(
            Uuid::parse_str(s).map_err(|_| TypeConstraintError::InvalidUuid)?,
        ))
    }
}

impl Default for TaskPublicId {
    fn default() -> Self {
        Self::new()
    }
}
