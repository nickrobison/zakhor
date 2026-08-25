mod index;
mod scored_doc;
mod simd;

#[cfg(test)]
mod tests;

pub use index::SemanticIndex;
pub use scored_doc::ScoredDoc;
#[cfg(test)]
pub(crate) use simd::{cosine_similarity, cosine_similarity_scalar};
