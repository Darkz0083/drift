//! Markdown notes stored as files the person owns.
#![forbid(unsafe_code)]

mod folder;
mod index;
pub mod legacy;
mod library;
mod markdown;
mod names;
mod service;

pub use service::{Host, NoteService};
// The index and what it remembers about pages are a storage API in their own
// right: the command surface that records a page reads it directly rather than
// going through the note files, which have nothing to say about a URL.
pub use index::{Index, NewPageMemory, PageMemory};
