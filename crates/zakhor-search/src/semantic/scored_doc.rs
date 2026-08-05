/// A scored document returned by semantic or lexical search.
#[derive(Debug, Clone)]
pub struct ScoredDoc {
    pub id: String,
    pub score: f64,
    pub text: String,
}
