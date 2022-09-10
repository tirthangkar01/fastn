#[cfg(test)]
#[macro_use]
mod test;

mod import;
mod main;
mod utils;

pub use ftd::ast::import::Import;
pub use ftd::ast::main::Ast;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("P1Error: {}", _0)]
    P1Error(#[from] ftd::p11::Error),

    #[error("ASTParseError: {doc_id}:{line_number} -> {message}")]
    ParseError {
        message: String,
        doc_id: String,
        line_number: usize,
    },
}

pub type Result<T> = std::result::Result<T, Error>;

pub fn parse_error<T, S1>(m: S1, doc_id: &str, line_number: usize) -> ftd::ast::Result<T>
where
    S1: Into<String>,
{
    Err(Error::ParseError {
        message: m.into(),
        doc_id: doc_id.to_string(),
        line_number,
    })
}