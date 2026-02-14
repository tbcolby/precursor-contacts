//! Application state machine for Contact Book.

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;

use crate::contact::{self, Category, Contact};
use crate::storage::Storage;

const KEY_UP: char = '\u{2191}';
const KEY_DOWN: char = '\u{2193}';
const KEY_LEFT: char = '\u{2190}';
const KEY_RIGHT: char = '\u{2192}';
const KEY_ENTER: char = '\r';
const KEY_BACKSPACE: char = '\u{0008}';
const KEY_MENU: char = '\u{2234}';

const MAX_NAME: usize = 40;
const MAX_PHONE: usize = 20;
const MAX_EMAIL: usize = 60;
const MAX_NOTES: usize = 120;

#[derive(Debug, Clone, PartialEq)]
pub enum AppState {
    List,
    View,
    Add,
    Edit,
    Search,
    ConfirmDelete,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EditField {
    Name,
    Phone,
    Email,
    Notes,
    Category,
    Save,
}

impl EditField {
    pub fn all() -> &'static [EditField] {
        &[EditField::Name, EditField::Phone, EditField::Email,
          EditField::Notes, EditField::Category, EditField::Save]
    }

    pub fn label(&self) -> &'static str {
        match self {
            EditField::Name => "Name",
            EditField::Phone => "Phone",
            EditField::Email => "Email",
            EditField::Notes => "Notes",
            EditField::Category => "Category",
            EditField::Save => "[SAVE]",
        }
    }

    pub fn max_len(&self) -> usize {
        match self {
            EditField::Name => MAX_NAME,
            EditField::Phone => MAX_PHONE,
            EditField::Email => MAX_EMAIL,
            EditField::Notes => MAX_NOTES,
            EditField::Category | EditField::Save => 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SortMode {
    Alpha,
    ByCategory,
}

pub struct ContactsApp {
    pub state: AppState,
    pub needs_redraw: bool,

    // Contact list
    pub contacts: Vec<Contact>,
    pub filtered: Vec<usize>,  // indices into contacts
    pub list_cursor: usize,
    pub list_scroll: usize,    // scroll offset for long lists
    pub sort_mode: SortMode,

    // Search
    pub search_query: String,

    // View
    pub viewed_index: usize,

    // Edit/Add
    pub edit_contact: Option<Contact>,
    pub edit_field_cursor: usize,
    pub editing_field: bool,  // true = typing into field, false = navigating fields

    // Next ID
    pub next_id: u32,

    // Storage
    pub storage: Option<Storage>,
}

const LIST_VISIBLE: usize = 18; // contacts visible on screen

impl ContactsApp {
    pub fn new() -> Self {
        Self {
            state: AppState::List,
            needs_redraw: true,
            contacts: Vec::new(),
            filtered: Vec::new(),
            list_cursor: 0,
            list_scroll: 0,
            sort_mode: SortMode::Alpha,
            search_query: String::new(),
            viewed_index: 0,
            edit_contact: None,
            edit_field_cursor: 0,
            editing_field: false,
            next_id: 1,
            storage: None,
        }
    }

    pub fn init_storage(&mut self) {
        if let Ok(mut stor) = Storage::new() {
            self.contacts = stor.load_all_contacts();
            self.next_id = stor.load_next_id();
            self.storage = Some(stor);
            self.sort_contacts();
            self.rebuild_filtered();
        }
    }

    pub fn save_state(&mut self) {
        if let Some(ref mut stor) = self.storage {
            stor.save_index(&self.contacts);
            stor.save_next_id(self.next_id);
        }
    }

    fn sort_contacts(&mut self) {
        match self.sort_mode {
            SortMode::Alpha => contact::sort_alpha(&mut self.contacts),
            SortMode::ByCategory => contact::sort_by_category(&mut self.contacts),
        }
    }

    fn rebuild_filtered(&mut self) {
        self.filtered = (0..self.contacts.len())
            .filter(|&i| self.contacts[i].matches(&self.search_query))
            .collect();
        if self.list_cursor >= self.filtered.len() && !self.filtered.is_empty() {
            self.list_cursor = self.filtered.len() - 1;
        }
        self.clamp_scroll();
    }

    fn clamp_scroll(&mut self) {
        if self.list_cursor < self.list_scroll {
            self.list_scroll = self.list_cursor;
        }
        if self.list_cursor >= self.list_scroll + LIST_VISIBLE {
            self.list_scroll = self.list_cursor + 1 - LIST_VISIBLE;
        }
    }

    fn selected_contact_index(&self) -> Option<usize> {
        self.filtered.get(self.list_cursor).copied()
    }

    pub fn handle_key(&mut self, key: char) -> bool {
        self.needs_redraw = true;

        match self.state {
            AppState::List => self.handle_list_key(key),
            AppState::View => self.handle_view_key(key),
            AppState::Add | AppState::Edit => self.handle_edit_key(key),
            AppState::Search => self.handle_search_key(key),
            AppState::ConfirmDelete => self.handle_confirm_delete_key(key),
        }
    }

    // ─── List ───────────────────────────────────────────────────────────────

    fn handle_list_key(&mut self, key: char) -> bool {
        match key {
            KEY_UP => {
                if self.list_cursor > 0 {
                    self.list_cursor -= 1;
                    self.clamp_scroll();
                }
            }
            KEY_DOWN => {
                if self.list_cursor + 1 < self.filtered.len() {
                    self.list_cursor += 1;
                    self.clamp_scroll();
                }
            }
            KEY_ENTER | ' ' => {
                if let Some(idx) = self.selected_contact_index() {
                    self.viewed_index = idx;
                    self.state = AppState::View;
                }
            }
            'n' | 'N' => {
                let mut c = Contact::new(self.next_id);
                self.next_id += 1;
                self.edit_contact = Some(c);
                self.edit_field_cursor = 0;
                self.editing_field = true;
                self.state = AppState::Add;
            }
            's' | 'S' => {
                self.search_query.clear();
                self.state = AppState::Search;
            }
            'o' | 'O' => {
                self.sort_mode = match self.sort_mode {
                    SortMode::Alpha => SortMode::ByCategory,
                    SortMode::ByCategory => SortMode::Alpha,
                };
                self.sort_contacts();
                self.rebuild_filtered();
            }
            'd' | 'D' => {
                if self.selected_contact_index().is_some() {
                    self.state = AppState::ConfirmDelete;
                }
            }
            'q' | 'Q' | KEY_MENU => return false,
            _ => {}
        }
        true
    }

    // ─── View ───────────────────────────────────────────────────────────────

    fn handle_view_key(&mut self, key: char) -> bool {
        match key {
            'e' | 'E' => {
                if self.viewed_index < self.contacts.len() {
                    self.edit_contact = Some(self.contacts[self.viewed_index].clone());
                    self.edit_field_cursor = 0;
                    self.editing_field = false;
                    self.state = AppState::Edit;
                }
            }
            'd' | 'D' => {
                self.state = AppState::ConfirmDelete;
            }
            'q' | 'Q' | KEY_BACKSPACE | KEY_MENU => {
                self.state = AppState::List;
            }
            _ => {}
        }
        true
    }

    // ─── Edit / Add ─────────────────────────────────────────────────────────

    fn handle_edit_key(&mut self, key: char) -> bool {
        let fields = EditField::all();

        if self.editing_field {
            // Currently typing into a field
            let field = fields[self.edit_field_cursor];

            match key {
                KEY_ENTER => {
                    if field == EditField::Save || field == EditField::Category {
                        self.editing_field = false;
                    } else {
                        // Done editing this field, move to next
                        self.editing_field = false;
                    }
                }
                KEY_BACKSPACE => {
                    if let Some(ref mut c) = self.edit_contact {
                        match field {
                            EditField::Name => { c.name.pop(); }
                            EditField::Phone => { c.phone.pop(); }
                            EditField::Email => { c.email.pop(); }
                            EditField::Notes => { c.notes.pop(); }
                            _ => {}
                        }
                    }
                }
                c if !c.is_control() && c != KEY_UP && c != KEY_DOWN
                    && c != KEY_LEFT && c != KEY_RIGHT => {
                    if let Some(ref mut contact) = self.edit_contact {
                        let (buf, max) = match field {
                            EditField::Name => (&mut contact.name, MAX_NAME),
                            EditField::Phone => (&mut contact.phone, MAX_PHONE),
                            EditField::Email => (&mut contact.email, MAX_EMAIL),
                            EditField::Notes => (&mut contact.notes, MAX_NOTES),
                            _ => return true,
                        };
                        if buf.len() < max {
                            buf.push(c);
                        }
                    }
                }
                _ => {}
            }
        } else {
            // Navigating fields
            match key {
                KEY_UP => {
                    if self.edit_field_cursor > 0 {
                        self.edit_field_cursor -= 1;
                    }
                }
                KEY_DOWN | '\t' => {
                    if self.edit_field_cursor + 1 < fields.len() {
                        self.edit_field_cursor += 1;
                    }
                }
                KEY_ENTER | ' ' => {
                    let field = fields[self.edit_field_cursor];
                    match field {
                        EditField::Save => {
                            self.save_current_contact();
                            self.state = AppState::List;
                        }
                        EditField::Category => {
                            if let Some(ref mut c) = self.edit_contact {
                                c.category = c.category.next();
                            }
                        }
                        _ => {
                            self.editing_field = true;
                        }
                    }
                }
                'c' | 'C' => {
                    if let Some(ref mut c) = self.edit_contact {
                        c.category = c.category.next();
                    }
                }
                'q' | 'Q' | KEY_MENU => {
                    // Cancel — don't save
                    self.edit_contact = None;
                    self.state = AppState::List;
                }
                _ => {}
            }
        }
        true
    }

    fn save_current_contact(&mut self) {
        if let Some(contact) = self.edit_contact.take() {
            if contact.name.is_empty() {
                return; // Don't save nameless contacts
            }

            // Check if editing existing or adding new
            if let Some(pos) = self.contacts.iter().position(|c| c.id == contact.id) {
                self.contacts[pos] = contact.clone();
            } else {
                self.contacts.push(contact.clone());
            }

            if let Some(ref mut stor) = self.storage {
                stor.save_contact(&contact);
                stor.save_index(&self.contacts);
                stor.save_next_id(self.next_id);
            }

            self.sort_contacts();
            self.rebuild_filtered();
        }
    }

    // ─── Search ─────────────────────────────────────────────────────────────

    fn handle_search_key(&mut self, key: char) -> bool {
        match key {
            KEY_ENTER => {
                self.rebuild_filtered();
                self.list_cursor = 0;
                self.list_scroll = 0;
                self.state = AppState::List;
            }
            KEY_BACKSPACE => {
                self.search_query.pop();
                self.rebuild_filtered();
            }
            'q' | 'Q' | KEY_MENU if self.search_query.is_empty() => {
                self.search_query.clear();
                self.rebuild_filtered();
                self.state = AppState::List;
            }
            c if !c.is_control() && c != KEY_UP && c != KEY_DOWN
                && c != KEY_LEFT && c != KEY_RIGHT => {
                if self.search_query.len() < 30 {
                    self.search_query.push(c);
                    self.rebuild_filtered();
                }
            }
            _ => {}
        }
        true
    }

    // ─── Confirm Delete ─────────────────────────────────────────────────────

    fn handle_confirm_delete_key(&mut self, key: char) -> bool {
        match key {
            'y' | 'Y' | KEY_ENTER => {
                let idx = if self.state == AppState::ConfirmDelete
                    && self.contacts.get(self.viewed_index).is_some()
                    && matches!(self.state, AppState::ConfirmDelete)
                {
                    // Came from View or List
                    if let Some(ci) = self.selected_contact_index() {
                        Some(ci)
                    } else {
                        Some(self.viewed_index)
                    }
                } else {
                    self.selected_contact_index()
                };

                if let Some(i) = idx {
                    if i < self.contacts.len() {
                        let id = self.contacts[i].id;
                        self.contacts.remove(i);
                        if let Some(ref mut stor) = self.storage {
                            stor.delete_contact(id);
                            stor.save_index(&self.contacts);
                        }
                        self.rebuild_filtered();
                    }
                }
                self.state = AppState::List;
            }
            _ => {
                // Any other key = cancel
                self.state = AppState::List;
            }
        }
        true
    }
}
