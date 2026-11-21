//! Contact records, search and sorting.
//!
//! Each contact is its own PDDB key (`c.<16 hex>`, random), so a contact lives
//! in exactly one basis, deleting it deletes its key, and there is no separate
//! index or id counter that can be lost and collide with existing records.

use core::cmp::Ordering;
use core::fmt;

use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

use crate::entropy::{Entropy, EntropyError};

/// Bump when the format changes. Records with a newer version are counted as
/// unreadable and never rewritten.
pub const RECORD_VERSION: u32 = 1;
pub const KEY_PREFIX: &str = "c.";

pub const MAX_NAME: usize = 40;
pub const MAX_PHONE: usize = 20;
pub const MAX_EMAIL: usize = 60;
pub const MAX_NOTES: usize = 120;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Category {
    Personal,
    Work,
    Family,
    Other,
}

impl Category {
    pub const ALL: [Category; 4] = [Category::Personal, Category::Work, Category::Family, Category::Other];

    pub fn label(self) -> &'static str {
        match self {
            Category::Personal => "Personal",
            Category::Work => "Work",
            Category::Family => "Family",
            Category::Other => "Other",
        }
    }

    pub fn short(self) -> &'static str {
        match self {
            Category::Personal => "PER",
            Category::Work => "WRK",
            Category::Family => "FAM",
            Category::Other => "OTH",
        }
    }

    pub fn next(self) -> Category {
        match self {
            Category::Personal => Category::Work,
            Category::Work => Category::Family,
            Category::Family => Category::Other,
            Category::Other => Category::Personal,
        }
    }

    pub fn prev(self) -> Category {
        match self {
            Category::Personal => Category::Other,
            Category::Work => Category::Personal,
            Category::Family => Category::Work,
            Category::Other => Category::Family,
        }
    }
}

/// One contact as stored.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Contact {
    pub version: u32,
    pub name: String,
    pub phone: String,
    pub email: String,
    pub notes: String,
    pub category: Category,
}

/// Personal data: `Debug` shows only the shape, so a stray `{:?}` in a log
/// line can't leak who is in the address book.
impl fmt::Debug for Contact {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Contact {{ {:?}, <{} name chars redacted> }}", self.category, self.name.chars().count())
    }
}

impl Drop for Contact {
    fn drop(&mut self) {
        self.name.zeroize();
        self.phone.zeroize();
        self.email.zeroize();
        self.notes.zeroize();
    }
}

impl Default for Contact {
    fn default() -> Self { Contact::new() }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordError {
    Unparsable,
    NewerVersion(u32),
}

/// Lowercase for comparison (Unicode-aware, so "Émile" matches "émile").
fn fold(s: &str) -> String { s.to_lowercase() }

impl Contact {
    pub fn new() -> Self {
        Contact {
            version: RECORD_VERSION,
            name: String::new(),
            phone: String::new(),
            email: String::new(),
            notes: String::new(),
            category: Category::Personal,
        }
    }

    /// Parse stored bytes. A parse failure is an error, never "empty": the
    /// caller leaves the record alone instead of overwriting it.
    pub fn parse(bytes: &[u8]) -> Result<Contact, RecordError> {
        #[derive(Deserialize)]
        struct V {
            version: u32,
        }
        let v: V = serde_json::from_slice(bytes).map_err(|_| RecordError::Unparsable)?;
        if v.version > RECORD_VERSION {
            return Err(RecordError::NewerVersion(v.version));
        }
        serde_json::from_slice(bytes).map_err(|_| RecordError::Unparsable)
    }

    /// Case-insensitive substring match on name, phone, email and notes.
    /// An empty query matches everything.
    pub fn matches(&self, query: &str) -> bool {
        let q = fold(query.trim());
        if q.is_empty() {
            return true;
        }
        [&self.name, &self.phone, &self.email, &self.notes].iter().any(|f| fold(f).contains(&q))
    }

    /// Display line for the contact list.
    pub fn list_line(&self) -> String {
        let mut s = format!("[{}] {}", self.category.short(), self.name);
        if !self.phone.is_empty() {
            s.push_str(" · ");
            s.push_str(&self.phone);
        }
        s
    }
}

/// Name order shared by both sort modes: case-insensitive, then exact as a
/// tie-break so the order is total and stable across reloads.
pub fn cmp_name(a: &Contact, b: &Contact) -> Ordering { fold(&a.name).cmp(&fold(&b.name)).then_with(|| a.name.cmp(&b.name)) }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortMode {
    Alpha,
    ByCategory,
}

impl SortMode {
    pub fn toggle(self) -> SortMode {
        match self {
            SortMode::Alpha => SortMode::ByCategory,
            SortMode::ByCategory => SortMode::Alpha,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            SortMode::Alpha => "A-Z",
            SortMode::ByCategory => "Category",
        }
    }

    pub fn cmp(self, a: &Contact, b: &Contact) -> Ordering {
        match self {
            SortMode::Alpha => cmp_name(a, b),
            SortMode::ByCategory => a.category.cmp(&b.category).then_with(|| cmp_name(a, b)),
        }
    }
}

/// True for PDDB key names this app wrote.
pub fn is_record_key(name: &str) -> bool {
    name.len() == KEY_PREFIX.len() + 16
        && name.starts_with(KEY_PREFIX)
        && name[KEY_PREFIX.len()..].bytes().all(|b| b.is_ascii_hexdigit())
}

/// A fresh random key name for a new contact.
pub fn new_key<E: Entropy + ?Sized>(e: &mut E) -> Result<String, EntropyError> {
    let hi = e.try_u32()?;
    let lo = e.try_u32()?;
    Ok(format!("{KEY_PREFIX}{hi:08x}{lo:08x}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entropy::testing::*;

    fn c(name: &str, cat: Category) -> Contact {
        let mut x = Contact::new();
        x.name = name.into();
        x.category = cat;
        x
    }

    #[test]
    fn roundtrip() {
        let mut a = c("Ada Lovelace", Category::Work);
        a.phone = "+44 20 7946 0000".into();
        a.notes = "Analytical engine".into();
        let j = serde_json::to_vec(&a).unwrap();
        assert_eq!(Contact::parse(&j).unwrap(), a);
    }

    #[test]
    fn corrupt_is_an_error_not_skipped_silently() {
        // Regression: a shorter in-place rewrite (set_len missing) left a stale
        // tail and load_all_contacts skipped the record without a word.
        let mut j = serde_json::to_vec(&c("Bob", Category::Other)).unwrap();
        j.extend_from_slice(b"nal\"}");
        assert_eq!(Contact::parse(&j), Err(RecordError::Unparsable));
        assert_eq!(Contact::parse(br#"{"version":4}"#), Err(RecordError::NewerVersion(4)));
    }

    #[test]
    fn debug_does_not_leak() {
        let mut a = c("Ada Lovelace", Category::Work);
        a.phone = "5551234".into();
        let s = format!("{a:?}");
        assert!(!s.contains("Ada") && !s.contains("5551234"), "{}", s);
    }

    #[test]
    fn search_covers_all_fields_case_insensitively() {
        let mut a = c("Grace Hopper", Category::Work);
        a.phone = "555-0101".into();
        a.email = "grace@navy.mil".into();
        a.notes = "COBOL".into();
        assert!(a.matches(""));
        assert!(a.matches("hopper"));
        assert!(a.matches("GRACE"));
        assert!(a.matches("0101"));
        assert!(a.matches("navy"));
        assert!(a.matches("cobol"));
        assert!(!a.matches("lovelace"));
        assert!(c("Émile", Category::Other).matches("émi"));
        // A query starting with q is an ordinary query (regression: q exited search).
        assert!(c("Quentin", Category::Other).matches("q"));
    }

    #[test]
    fn both_sorts_share_case_insensitive_name_order() {
        // Regression: category sort used case-sensitive name order.
        let mut v = vec![c("bob", Category::Work), c("Alice", Category::Work), c("carol", Category::Personal)];
        v.sort_by(|a, b| SortMode::Alpha.cmp(a, b));
        let names: Vec<&str> = v.iter().map(|x| x.name.as_str()).collect();
        assert_eq!(names, ["Alice", "bob", "carol"]);
        v.sort_by(|a, b| SortMode::ByCategory.cmp(a, b));
        let names: Vec<&str> = v.iter().map(|x| x.name.as_str()).collect();
        assert_eq!(names, ["carol", "Alice", "bob"]);
    }

    #[test]
    fn category_cycles() {
        for cat in Category::ALL {
            assert_eq!(cat.next().prev(), cat);
        }
        assert_eq!(Category::Other.next(), Category::Personal);
    }

    #[test]
    fn keys() {
        let k = new_key(&mut Seq(vec![1, 0xabcdef01], 0)).unwrap();
        assert_eq!(k, "c.00000001abcdef01");
        assert!(is_record_key(&k));
        assert!(!is_record_key("contact.1"));
        assert!(!is_record_key("index"));
        assert!(new_key(&mut Broken).is_err());
    }

    #[test]
    fn list_line() {
        let mut a = c("Ada", Category::Family);
        assert_eq!(a.list_line(), "[FAM] Ada");
        a.phone = "123".into();
        assert_eq!(a.list_line(), "[FAM] Ada · 123");
    }
}
