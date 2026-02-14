//! PDDB storage for Contact Book.
//!
//! Dictionary: contacts.data
//! Keys: config, index, contact.{id}

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;
use alloc::format;

use crate::contact::Contact;

const DICT: &str = "contacts.data";
const KEY_CONFIG: &str = "config";
const KEY_INDEX: &str = "index";

pub struct Storage {
    pddb: pddb::Pddb,
}

impl Storage {
    pub fn new() -> Result<Self, ()> {
        let pddb = pddb::Pddb::new();
        pddb.is_mounted_blocking();
        Ok(Self { pddb })
    }

    fn read_key(&mut self, key: &str) -> Option<Vec<u8>> {
        let mut handle = self
            .pddb
            .get(DICT, key, None, false, false, None, None::<fn()>)
            .ok()?;
        let mut buf = Vec::new();
        use std::io::Read;
        handle.read_to_end(&mut buf).ok()?;
        Some(buf)
    }

    fn write_key(&mut self, key: &str, data: &[u8]) {
        if let Ok(mut handle) = self.pddb.get(
            DICT, key, None, true, true, Some(data.len()), None::<fn()>,
        ) {
            use std::io::{Seek, Write};
            handle.seek(std::io::SeekFrom::Start(0)).ok();
            handle.write_all(data).ok();
            handle.set_len(data.len() as u64).ok();
        }
        self.pddb.sync().ok();
    }

    fn delete_key(&mut self, key: &str) {
        let _ = self.pddb.delete_key(DICT, key, None);
        self.pddb.sync().ok();
    }

    // ── Config ──

    pub fn load_next_id(&mut self) -> u32 {
        let buf = match self.read_key(KEY_CONFIG) {
            Some(b) => b,
            None => return 1,
        };
        let json: serde_json::Value = match serde_json::from_slice(&buf) {
            Ok(j) => j,
            Err(_) => return 1,
        };
        json.get("next_id").and_then(|v| v.as_u64()).unwrap_or(1) as u32
    }

    pub fn save_next_id(&mut self, next_id: u32) {
        let json = serde_json::json!({ "next_id": next_id });
        let data = serde_json::to_vec(&json).unwrap_or_default();
        self.write_key(KEY_CONFIG, &data);
    }

    // ── Contacts ──

    pub fn load_all_contacts(&mut self) -> Vec<Contact> {
        let mut contacts = Vec::new();

        // Load index
        let ids: Vec<u32> = match self.read_key(KEY_INDEX) {
            Some(buf) => serde_json::from_slice(&buf).unwrap_or_default(),
            None => return contacts,
        };

        for id in ids {
            let key_name = format!("contact.{}", id);
            if let Some(buf) = self.read_key(&key_name) {
                if let Ok(contact) = serde_json::from_slice::<Contact>(&buf) {
                    contacts.push(contact);
                }
            }
        }

        contacts
    }

    pub fn save_contact(&mut self, contact: &Contact) {
        let key_name = format!("contact.{}", contact.id);
        let data = serde_json::to_vec(contact).unwrap_or_default();
        self.write_key(&key_name, &data);
    }

    pub fn delete_contact(&mut self, id: u32) {
        let key_name = format!("contact.{}", id);
        self.delete_key(&key_name);
    }

    pub fn save_index(&mut self, contacts: &[Contact]) {
        let ids: Vec<u32> = contacts.iter().map(|c| c.id).collect();
        let data = serde_json::to_vec(&ids).unwrap_or_default();
        self.write_key(KEY_INDEX, &data);
    }
}
