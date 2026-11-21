//! Pure logic for Contact Book: contact records, search, sorting, and the
//! screen state machine. No Xous dependencies; everything here is
//! unit-tested on the host.

pub mod contact;
pub mod entropy;
pub mod ui;

pub use contact::{Category, Contact, SortMode};
pub use entropy::{Entropy, EntropyError};
pub use ui::{Effect, Entry, Field, Input, Screen, Ui};
