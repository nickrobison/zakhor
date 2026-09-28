use oxiri::{Iri, IriParseError};
use serde::{Deserialize, Serialize};

macro_rules! define_uri_newtype {
    ($name:ident) => {
        #[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(Iri<String>);

        impl $name {
            /// Parses and validates an absolute IRI.
            ///
            /// # Errors
            /// Returns [`IriParseError`] when `s` is not a well-formed absolute IRI.
            pub fn parse(s: impl Into<String>) -> Result<Self, IriParseError> {
                Iri::parse(s.into()).map(Self)
            }

            /// Returns the IRI's original string representation.
            pub fn as_str(&self) -> &str {
                self.0.as_str()
            }

            /// Returns the wrapped IRI for storage-layer APIs.
            pub fn as_iri(&self) -> &Iri<String> {
                &self.0
            }
        }
    };
}

define_uri_newtype!(EntityUri);
define_uri_newtype!(ObservationUri);
define_uri_newtype!(ProjectUri);
define_uri_newtype!(RepositoryUri);
define_uri_newtype!(DecisionUri);
define_uri_newtype!(RelationPredicate);
