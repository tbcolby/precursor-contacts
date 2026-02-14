//! Contact data model.

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;

use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Category {
    Personal,
    Work,
    Family,
    Other,
}

impl Category {
    pub fn label(&self) -> &'static str {
        match self {
            Category::Personal => "Personal",
            Category::Work => "Work",
            Category::Family => "Family",
            Category::Other => "Other",
        }
    }

    pub fn short(&self) -> &'static str {
        match self {
            Category::Personal => "PER",
            Category::Work => "WRK",
            Category::Family => "FAM",
            Category::Other => "OTH",
        }
    }

    pub fn next(&self) -> Category {
        match self {
            Category::Personal => Category::Work,
            Category::Work => Category::Family,
            Category::Family => Category::Other,
            Category::Other => Category::Personal,
        }
    }

    pub fn all() -> &'static [Category] {
        &[Category::Personal, Category::Work, Category::Family, Category::Other]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contact {
    pub id: u32,
    pub name: String,
    pub phone: String,
    pub email: String,
    pub notes: String,
    pub category: Category,
}

impl Contact {
    pub fn new(id: u32) -> Self {
        Self {
            id,
            name: String::new(),
            phone: String::new(),
            email: String::new(),
            notes: String::new(),
            category: Category::Personal,
        }
    }

    /// Check if this contact matches a search query (case-insensitive name search).
    pub fn matches(&self, query: &str) -> bool {
        if query.is_empty() {
            return true;
        }
        // Simple case-insensitive substring match
        let name_lower: Vec<u8> = self.name.bytes().map(|b| {
            if b >= b'A' && b <= b'Z' { b + 32 } else { b }
        }).collect();
        let query_lower: Vec<u8> = query.bytes().map(|b| {
            if b >= b'A' && b <= b'Z' { b + 32 } else { b }
        }).collect();

        if query_lower.len() > name_lower.len() {
            return false;
        }

        for i in 0..=(name_lower.len() - query_lower.len()) {
            if name_lower[i..i + query_lower.len()] == query_lower[..] {
                return true;
            }
        }
        false
    }

    /// Display line for the contact list.
    pub fn list_line(&self) -> String {
        let mut s = String::new();
        s += &self.name;
        if !self.phone.is_empty() {
            s += " | ";
            s += &self.phone;
        }
        s
    }
}

/// Sort contacts alphabetically by name.
pub fn sort_alpha(contacts: &mut [Contact]) {
    contacts.sort_by(|a, b| {
        let a_lower: Vec<u8> = a.name.bytes().map(|b| {
            if b >= b'A' && b <= b'Z' { b + 32 } else { b }
        }).collect();
        let b_lower: Vec<u8> = b.name.bytes().map(|b_byte| {
            if b_byte >= b'A' && b_byte <= b'Z' { b_byte + 32 } else { b_byte }
        }).collect();
        a_lower.cmp(&b_lower)
    });
}

/// Sort contacts by category, then alphabetically within each.
pub fn sort_by_category(contacts: &mut [Contact]) {
    contacts.sort_by(|a, b| {
        let cat_cmp = (a.category as u8).cmp(&(b.category as u8));
        if cat_cmp == core::cmp::Ordering::Equal {
            a.name.cmp(&b.name)
        } else {
            cat_cmp
        }
    });
}
