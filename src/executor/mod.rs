#[cfg(test)]
#[macro_use]
mod test;

mod code;
mod element;
mod main;
mod markup;
mod styles;
mod tdoc;
mod utils;
mod value;

pub use element::{Code, Column, Common, Container, Element, Event, Image, Row, Text};
pub use main::{ExecuteDoc, RT};
pub use styles::{
    AlignSelf, Alignment, Anchor, Background, Color, ColorValue, Cursor, FontSize, Length,
    Overflow, Region, Resize, Resizing, ResponsiveType, SpacingMode, TextAlign, TextTransform,
    WhiteSpace,
};
pub(crate) use tdoc::TDoc;
pub(crate) use value::Value;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("InterpreterError: {}", _0)]
    InterpreterError(#[from] ftd::interpreter2::Error),

    #[error("{doc_id}:{line_number} -> {message}")]
    ParseError {
        message: String,
        doc_id: String,
        line_number: usize,
    },

    #[error("syntect error: {source}")]
    Syntect {
        #[from]
        source: syntect::Error,
    },
}

pub type Result<T> = std::result::Result<T, Error>;
