//! Screen state machine. Pure: the app crate maps device keys to [`Input`],
//! carries out the returned [`Effect`] (storage, leaving), reports the outcome
//! back, and draws from the public state.
//!
//! Every screen has an exit on Backspace or F4. The menu key `∴` is consumed
//! by GAM and never reaches the app, so nothing is bound to it.

use crate::contact::{new_key, Contact, SortMode, MAX_EMAIL, MAX_NAME, MAX_NOTES, MAX_PHONE};
use crate::entropy::Entropy;

pub const MAX_QUERY: usize = 30;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Input {
    Up,
    Down,
    Left,
    Right,
    Enter,
    Backspace,
    F(u8),
    Char(char),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    List,
    View,
    Edit,
    Search,
    ConfirmDelete,
    ConfirmDiscard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    Name,
    Phone,
    Email,
    Notes,
    Category,
    Save,
}

impl Field {
    pub const ALL: [Field; 6] = [Field::Name, Field::Phone, Field::Email, Field::Notes, Field::Category, Field::Save];

    pub fn label(self) -> &'static str {
        match self {
            Field::Name => "Name",
            Field::Phone => "Phone",
            Field::Email => "Email",
            Field::Notes => "Notes",
            Field::Category => "Category",
            Field::Save => "Save",
        }
    }

    pub fn max_chars(self) -> usize {
        match self {
            Field::Name => MAX_NAME,
            Field::Phone => MAX_PHONE,
            Field::Email => MAX_EMAIL,
            Field::Notes => MAX_NOTES,
            Field::Category | Field::Save => 0,
        }
    }

    fn text(self, c: &mut Contact) -> Option<&mut String> {
        match self {
            Field::Name => Some(&mut c.name),
            Field::Phone => Some(&mut c.phone),
            Field::Email => Some(&mut c.email),
            Field::Notes => Some(&mut c.notes),
            Field::Category | Field::Save => None,
        }
    }
}

/// Work for the app crate.
#[derive(Debug)]
pub enum Effect {
    None,
    /// Hand focus back to GAM.
    Leave,
    /// Write this contact under this PDDB key, then call `on_saved` / `on_save_failed`.
    Save(String, Contact),
    /// Delete this PDDB key, then call `on_deleted` / `on_delete_failed`.
    Delete(String),
}

/// A loaded contact with its PDDB key and the basis it lives in.
#[derive(Debug)]
pub struct Entry {
    pub key: String,
    pub contact: Contact,
    pub basis: Option<String>,
}

pub struct Ui {
    pub screen: Screen,
    pub entries: Vec<Entry>,
    /// Indices into `entries` that match `query`, in display order.
    pub filtered: Vec<usize>,
    /// Position in `filtered`.
    pub cursor: usize,
    pub sort: SortMode,
    pub query: String,
    /// Key of the contact on the View / ConfirmDelete screen.
    pub target: Option<String>,
    /// The form: `None` key = new contact.
    pub form: Contact,
    pub form_key: Option<String>,
    pub field: usize,
    form_dirty: bool,
    /// Screen to return to when a delete is cancelled.
    delete_from: Screen,
    /// Records present in storage that couldn't be read (never rewritten).
    pub unreadable: usize,
    pub status: String,
}

impl Default for Ui {
    fn default() -> Self { Self::new() }
}

impl Ui {
    pub fn new() -> Self {
        Ui {
            screen: Screen::List,
            entries: Vec::new(),
            filtered: Vec::new(),
            cursor: 0,
            sort: SortMode::Alpha,
            query: String::new(),
            target: None,
            form: Contact::new(),
            form_key: None,
            field: 0,
            form_dirty: false,
            delete_from: Screen::List,
            unreadable: 0,
            status: String::new(),
        }
    }

    /// Replace all contacts (startup and after a basis change). Screens that
    /// show a contact which is no longer loaded fall back to the list; an
    /// open form for a new contact is kept, an edit of a vanished one is dropped.
    pub fn set_entries(&mut self, entries: Vec<Entry>, unreadable: usize) {
        self.entries = entries;
        self.unreadable = unreadable;
        self.refresh();
        let target_gone = self.target.as_ref().is_some_and(|k| self.find(k).is_none());
        let form_gone = self.form_key.as_ref().is_some_and(|k| self.find(k).is_none());
        match self.screen {
            Screen::View | Screen::ConfirmDelete if target_gone => {
                self.target = None;
                self.screen = Screen::List;
            }
            Screen::Edit | Screen::ConfirmDiscard if form_gone => {
                self.close_form();
                self.screen = Screen::List;
                self.status = "Contact's basis was locked".into();
            }
            _ => {}
        }
    }

    fn find(&self, key: &str) -> Option<usize> { self.entries.iter().position(|e| e.key == key) }

    /// Re-sort and re-filter, keeping the cursor on the same contact if possible.
    fn refresh(&mut self) {
        let keep = self.selected().map(|e| e.key.clone());
        let sort = self.sort;
        self.entries.sort_by(|a, b| sort.cmp(&a.contact, &b.contact));
        self.filtered = (0..self.entries.len()).filter(|&i| self.entries[i].contact.matches(&self.query)).collect();
        if let Some(k) = keep {
            if let Some(pos) = self.filtered.iter().position(|&i| self.entries[i].key == k) {
                self.cursor = pos;
            }
        }
        self.cursor = self.cursor.min(self.filtered.len().saturating_sub(1));
    }

    /// The highlighted contact in the list.
    pub fn selected(&self) -> Option<&Entry> { self.filtered.get(self.cursor).and_then(|&i| self.entries.get(i)) }

    /// The contact on the View / ConfirmDelete screen.
    pub fn target(&self) -> Option<&Entry> { self.target.as_ref().and_then(|k| self.find(k)).map(|i| &self.entries[i]) }

    pub fn current_field(&self) -> Field { Field::ALL[self.field] }

    pub fn is_new(&self) -> bool { self.form_key.is_none() }

    pub fn on_key<E: Entropy + ?Sized>(&mut self, key: Input, rng: &mut E) -> Effect {
        self.status.clear();
        match self.screen {
            Screen::List => self.key_list(key),
            Screen::View => self.key_view(key),
            Screen::Edit => self.key_edit(key, rng),
            Screen::Search => self.key_search(key),
            Screen::ConfirmDelete => self.key_confirm_delete(key),
            Screen::ConfirmDiscard => self.key_confirm_discard(key),
        }
    }

    fn open_form(&mut self, existing: Option<usize>) {
        match existing {
            Some(i) => {
                self.form = self.entries[i].contact.clone();
                self.form_key = Some(self.entries[i].key.clone());
            }
            None => {
                self.form = Contact::new();
                self.form_key = None;
            }
        }
        self.field = 0;
        self.form_dirty = false;
        self.screen = Screen::Edit;
    }

    fn close_form(&mut self) {
        self.form = Contact::new(); // the old one is wiped on drop
        self.form_key = None;
        self.form_dirty = false;
    }

    fn start_delete(&mut self, from: Screen) {
        self.delete_from = from;
        self.screen = Screen::ConfirmDelete;
    }

    fn key_list(&mut self, key: Input) -> Effect {
        let n = self.filtered.len();
        match key {
            Input::Up => self.cursor = self.cursor.saturating_sub(1),
            Input::Down => self.cursor = (self.cursor + 1).min(n.saturating_sub(1)),
            Input::Enter => {
                if let Some(k) = self.selected().map(|e| e.key.clone()) {
                    self.target = Some(k);
                    self.screen = Screen::View;
                }
            }
            Input::Char('n') | Input::Char('N') | Input::F(1) => self.open_form(None),
            Input::Char('s') | Input::Char('S') | Input::Char('/') | Input::F(2) => self.screen = Screen::Search,
            Input::Char('o') | Input::Char('O') => {
                self.sort = self.sort.toggle();
                self.refresh();
            }
            Input::Char('d') | Input::Char('D') | Input::F(3) => {
                if let Some(k) = self.selected().map(|e| e.key.clone()) {
                    self.target = Some(k);
                    self.start_delete(Screen::List);
                }
            }
            // Backspace first clears an active filter, then leaves.
            Input::Backspace if !self.query.is_empty() => {
                self.query.clear();
                self.refresh();
            }
            // Leaving hands focus back to GAM; the process keeps running
            // (regression: `q` used to terminate it, and apps aren't respawned).
            Input::Backspace | Input::F(4) => return Effect::Leave,
            _ => {}
        }
        Effect::None
    }

    fn key_view(&mut self, key: Input) -> Effect {
        match key {
            Input::Char('e') | Input::Char('E') | Input::Enter | Input::F(1) => {
                if let Some(i) = self.target.as_ref().and_then(|k| self.find(k)) {
                    self.open_form(Some(i));
                }
            }
            Input::Char('d') | Input::Char('D') | Input::F(3) => {
                if self.target().is_some() {
                    self.start_delete(Screen::View);
                }
            }
            Input::Backspace | Input::F(4) => {
                self.target = None;
                self.screen = Screen::List;
            }
            _ => {}
        }
        Effect::None
    }

    fn key_edit<E: Entropy + ?Sized>(&mut self, key: Input, rng: &mut E) -> Effect {
        let field = self.current_field();
        let last = Field::ALL.len() - 1;
        match key {
            Input::Up => self.field = self.field.saturating_sub(1),
            Input::Down => self.field = (self.field + 1).min(last),
            Input::Left | Input::Right if field == Field::Category => {
                self.form.category = if key == Input::Left { self.form.category.prev() } else { self.form.category.next() };
                self.form_dirty = true;
            }
            Input::F(2) => return self.save(rng),
            Input::Enter => match field {
                Field::Save => return self.save(rng),
                Field::Category => {
                    self.form.category = self.form.category.next();
                    self.form_dirty = true;
                }
                // Regression: both Enter branches used to do the same thing and
                // never advanced; Enter now moves to the next field.
                _ => self.field += 1,
            },
            Input::F(4) => {
                if self.form_dirty {
                    self.screen = Screen::ConfirmDiscard;
                } else {
                    self.leave_form();
                }
            }
            Input::Backspace => match field.text(&mut self.form) {
                Some(t) if !t.is_empty() => {
                    t.pop();
                    self.form_dirty = true;
                }
                Some(_) => self.field = self.field.saturating_sub(1),
                None => self.field = self.field.saturating_sub(1),
            },
            Input::Char(c) => {
                let max = field.max_chars();
                match field.text(&mut self.form) {
                    Some(t) if t.chars().count() < max => {
                        t.push(c);
                        self.form_dirty = true;
                    }
                    Some(_) => self.status = format!("{} is at most {} characters", field.label(), max),
                    None if c == ' ' && field == Field::Category => {
                        self.form.category = self.form.category.next();
                        self.form_dirty = true;
                    }
                    None => {}
                }
            }
            _ => {}
        }
        Effect::None
    }

    fn leave_form(&mut self) {
        let back_to_view = self.form_key.is_some();
        self.close_form();
        self.screen = if back_to_view && self.target().is_some() { Screen::View } else { Screen::List };
    }

    fn save<E: Entropy + ?Sized>(&mut self, rng: &mut E) -> Effect {
        // Regression: an empty name silently discarded every other field.
        if self.form.name.trim().is_empty() {
            self.field = 0;
            self.status = "Name is required".into();
            return Effect::None;
        }
        let key = match &self.form_key {
            Some(k) => k.clone(),
            None => match new_key(rng) {
                Ok(k) => k,
                Err(_) => {
                    self.status = "TRNG error: not saved".into();
                    return Effect::None;
                }
            },
        };
        Effect::Save(key, self.form.clone())
    }

    /// The app wrote the contact. `basis` is where it landed.
    pub fn on_saved(&mut self, key: String, contact: Contact, basis: Option<String>) {
        self.status = match &basis {
            Some(b) => format!("Saved (basis {b})"),
            None => "Saved".into(),
        };
        match self.find(&key) {
            Some(i) => {
                self.entries[i].contact = contact;
                self.entries[i].basis = basis;
            }
            None => self.entries.push(Entry { key: key.clone(), contact, basis }),
        }
        self.close_form();
        // Show the saved contact in the list, even if the filter would hide it.
        if !self.find(&key).is_some_and(|i| self.entries[i].contact.matches(&self.query)) {
            self.query.clear();
        }
        self.refresh();
        if let Some(pos) = self.filtered.iter().position(|&i| self.entries[i].key == key) {
            self.cursor = pos;
        }
        self.target = Some(key);
        self.screen = Screen::View;
    }

    /// The write failed: stay on the form with everything typed.
    pub fn on_save_failed(&mut self, why: &str) { self.status = format!("Save failed: {why}"); }

    fn key_search(&mut self, key: Input) -> Effect {
        match key {
            Input::Enter => {
                self.cursor = 0;
                self.screen = Screen::List;
            }
            Input::Backspace if !self.query.is_empty() => {
                self.query.pop();
                self.refresh();
            }
            // Backspace on an empty query, or F4, cancels the search.
            // (Regression: `q` on an empty query exited, so names starting
            // with Q couldn't be searched.)
            Input::Backspace | Input::F(4) => {
                self.query.clear();
                self.refresh();
                self.screen = Screen::List;
            }
            Input::Char(c) => {
                if self.query.chars().count() < MAX_QUERY {
                    self.query.push(c);
                    self.cursor = 0;
                    self.refresh();
                }
            }
            _ => {}
        }
        Effect::None
    }

    fn key_confirm_delete(&mut self, key: Input) -> Effect {
        // Regression: the target used to be re-derived from the list cursor
        // through redundant conditions; it is fixed when the dialog opens.
        match (key, self.target().map(|e| e.key.clone())) {
            (Input::Char('y') | Input::Char('Y'), Some(k)) => {
                self.screen = Screen::List;
                Effect::Delete(k)
            }
            _ => {
                self.screen = self.delete_from;
                self.status = "Not deleted".into();
                Effect::None
            }
        }
    }

    pub fn on_deleted(&mut self, key: &str) {
        self.entries.retain(|e| e.key != key);
        if self.target.as_deref() == Some(key) {
            self.target = None;
        }
        self.refresh();
        self.screen = Screen::List;
        self.status = "Deleted".into();
    }

    pub fn on_delete_failed(&mut self, why: &str) { self.status = format!("Delete failed: {why}"); }

    fn key_confirm_discard(&mut self, key: Input) -> Effect {
        match key {
            Input::Char('y') | Input::Char('Y') => self.leave_form(),
            _ => self.screen = Screen::Edit,
        }
        Effect::None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contact::Category;
    use crate::entropy::testing::*;

    fn press(ui: &mut Ui, keys: &[Input]) -> Effect {
        let mut last = Effect::None;
        for &k in keys {
            last = ui.on_key(k, &mut XorShift(9));
        }
        last
    }

    fn type_str(ui: &mut Ui, s: &str) {
        for c in s.chars() {
            ui.on_key(Input::Char(c), &mut XorShift(9));
        }
    }

    fn entry(key: &str, name: &str, cat: Category) -> Entry {
        let mut c = Contact::new();
        c.name = name.into();
        c.category = cat;
        Entry { key: key.into(), contact: c, basis: Some(".System".into()) }
    }

    fn sample() -> Ui {
        let mut ui = Ui::new();
        ui.set_entries(
            vec![
                entry("c.3", "carol", Category::Personal),
                entry("c.1", "Alice", Category::Work),
                entry("c.2", "bob", Category::Work),
                entry("c.4", "Quinn", Category::Other),
            ],
            0,
        );
        ui
    }

    fn names(ui: &Ui) -> Vec<String> { ui.filtered.iter().map(|&i| ui.entries[i].contact.name.clone()).collect() }

    #[test]
    fn list_sorted_and_toggles() {
        let mut ui = sample();
        assert_eq!(names(&ui), ["Alice", "bob", "carol", "Quinn"]);
        press(&mut ui, &[Input::Char('o')]);
        assert_eq!(names(&ui), ["carol", "Alice", "bob", "Quinn"]);
    }

    #[test]
    fn sort_keeps_cursor_on_contact() {
        let mut ui = sample();
        press(&mut ui, &[Input::Down]); // bob
        press(&mut ui, &[Input::Char('o')]);
        assert_eq!(ui.selected().unwrap().contact.name, "bob");
    }

    #[test]
    fn new_contact_flow() {
        let mut ui = sample();
        press(&mut ui, &[Input::Char('n')]);
        assert_eq!(ui.screen, Screen::Edit);
        assert!(ui.is_new());
        type_str(&mut ui, "Dave");
        press(&mut ui, &[Input::Enter]); // -> phone
        assert_eq!(ui.current_field(), Field::Phone);
        type_str(&mut ui, "555");
        press(&mut ui, &[Input::Enter, Input::Enter, Input::Enter]); // email, notes, category
        assert_eq!(ui.current_field(), Field::Category);
        press(&mut ui, &[Input::Right]);
        assert_eq!(ui.form.category, Category::Work);
        let (key, c) = match press(&mut ui, &[Input::Down, Input::Enter]) {
            Effect::Save(k, c) => (k, c),
            e => panic!("{e:?}"),
        };
        assert!(crate::contact::is_record_key(&key));
        assert_eq!(c.phone, "555");
        ui.on_saved(key.clone(), c, Some("secret".into()));
        assert_eq!(ui.status, "Saved (basis secret)");
        assert_eq!(ui.screen, Screen::View);
        assert_eq!(ui.target().unwrap().contact.name, "Dave");
        assert_eq!(ui.entries.len(), 5);
    }

    #[test]
    fn empty_name_keeps_form_and_says_why() {
        // Regression: saving without a name silently threw away the form.
        let mut ui = sample();
        press(&mut ui, &[Input::Char('n'), Input::Enter]);
        type_str(&mut ui, "555-1234");
        assert!(matches!(press(&mut ui, &[Input::F(2)]), Effect::None));
        assert_eq!(ui.screen, Screen::Edit);
        assert_eq!(ui.status, "Name is required");
        assert_eq!(ui.form.phone, "555-1234");
        assert_eq!(ui.current_field(), Field::Name);
    }

    #[test]
    fn new_contact_fails_closed_without_trng() {
        let mut ui = sample();
        press(&mut ui, &[Input::Char('n')]);
        type_str(&mut ui, "Eve");
        assert!(matches!(ui.on_key(Input::F(2), &mut Broken), Effect::None));
        assert_eq!(ui.status, "TRNG error: not saved");
        assert_eq!(ui.form.name, "Eve");
    }

    #[test]
    fn new_keys_are_random_not_a_counter() {
        // Regression: a lost `config` reset next_id to 1 and the next new
        // contact overwrote contact.1. Keys are now random per record.
        let mut ui = sample();
        let mut keys = Vec::new();
        for seed in [1u32, 2, 3] {
            press(&mut ui, &[Input::Char('n')]);
            type_str(&mut ui, "X");
            if let Effect::Save(k, c) = ui.on_key(Input::F(2), &mut XorShift(seed)) {
                assert!(ui.find(&k).is_none());
                keys.push(k.clone());
                ui.on_saved(k, c, None);
                press(&mut ui, &[Input::F(4)]);
            } else {
                panic!("no save");
            }
        }
        keys.sort();
        keys.dedup();
        assert_eq!(keys.len(), 3);
    }

    #[test]
    fn edit_reuses_key() {
        let mut ui = sample();
        press(&mut ui, &[Input::Enter]); // view Alice
        press(&mut ui, &[Input::Char('e')]);
        assert!(!ui.is_new());
        type_str(&mut ui, " L");
        match press(&mut ui, &[Input::F(2)]) {
            Effect::Save(k, c) => {
                assert_eq!(k, "c.1");
                assert_eq!(c.name, "Alice L");
                ui.on_saved(k, c, None);
            }
            e => panic!("{e:?}"),
        }
        assert_eq!(ui.entries.len(), 4);
        assert_eq!(ui.target().unwrap().contact.name, "Alice L");
    }

    #[test]
    fn letters_type_into_fields_not_shortcuts() {
        let mut ui = sample();
        press(&mut ui, &[Input::Char('n')]);
        type_str(&mut ui, "Cody qd");
        assert_eq!(ui.form.name, "Cody qd");
        assert_eq!(ui.screen, Screen::Edit);
    }

    #[test]
    fn field_limits_count_chars() {
        let mut ui = sample();
        press(&mut ui, &[Input::Char('n')]);
        type_str(&mut ui, &"é".repeat(MAX_NAME + 3));
        assert_eq!(ui.form.name.chars().count(), MAX_NAME);
        assert!(ui.status.contains("at most"));
    }

    #[test]
    fn cancel_edit_asks_only_when_changed() {
        let mut ui = sample();
        press(&mut ui, &[Input::Char('n'), Input::F(4)]);
        assert_eq!(ui.screen, Screen::List);
        press(&mut ui, &[Input::Char('n')]);
        type_str(&mut ui, "Zed");
        press(&mut ui, &[Input::F(4)]);
        assert_eq!(ui.screen, Screen::ConfirmDiscard);
        press(&mut ui, &[Input::Char('n')]);
        assert_eq!(ui.screen, Screen::Edit);
        assert_eq!(ui.form.name, "Zed");
        press(&mut ui, &[Input::F(4), Input::Char('y')]);
        assert_eq!(ui.screen, Screen::List);
        assert_eq!(ui.entries.len(), 4);
    }

    #[test]
    fn backspace_deletes_then_moves_up() {
        let mut ui = sample();
        press(&mut ui, &[Input::Char('n')]);
        type_str(&mut ui, "Al");
        press(&mut ui, &[Input::Enter, Input::Backspace]);
        assert_eq!(ui.current_field(), Field::Name);
        press(&mut ui, &[Input::Backspace]);
        assert_eq!(ui.form.name, "A");
    }

    #[test]
    fn search_filters_all_fields_and_q_is_typable() {
        let mut ui = sample();
        ui.entries[0].contact.email = "alice@work.example".into();
        press(&mut ui, &[Input::Char('s')]);
        assert_eq!(ui.screen, Screen::Search);
        type_str(&mut ui, "q");
        assert_eq!(ui.screen, Screen::Search, "q is a query character, not exit");
        assert_eq!(names(&ui), ["Quinn"]);
        press(&mut ui, &[Input::Backspace]);
        type_str(&mut ui, "work.ex");
        assert_eq!(names(&ui), ["Alice"]);
        press(&mut ui, &[Input::Enter]);
        assert_eq!(ui.screen, Screen::List);
        assert_eq!(ui.selected().unwrap().contact.name, "Alice");
        // Backspace on the list clears the filter before leaving.
        assert!(matches!(press(&mut ui, &[Input::Backspace]), Effect::None));
        assert_eq!(ui.filtered.len(), 4);
        assert!(matches!(press(&mut ui, &[Input::Backspace]), Effect::Leave));
    }

    #[test]
    fn search_cancel() {
        let mut ui = sample();
        press(&mut ui, &[Input::Char('/')]);
        type_str(&mut ui, "b");
        press(&mut ui, &[Input::Backspace, Input::Backspace]);
        assert_eq!(ui.screen, Screen::List);
        assert!(ui.query.is_empty());
        press(&mut ui, &[Input::F(2)]);
        type_str(&mut ui, "zz");
        press(&mut ui, &[Input::F(4)]);
        assert_eq!(ui.screen, Screen::List);
        assert_eq!(ui.filtered.len(), 4);
    }

    #[test]
    fn delete_from_view_targets_viewed_contact() {
        let mut ui = sample();
        press(&mut ui, &[Input::Down, Input::Down, Input::Enter]); // carol
        press(&mut ui, &[Input::Up]); // ignored on View
        press(&mut ui, &[Input::Char('d')]);
        assert_eq!(ui.screen, Screen::ConfirmDelete);
        match press(&mut ui, &[Input::Char('Y')]) {
            Effect::Delete(k) => {
                assert_eq!(k, "c.3");
                ui.on_deleted(&k);
            }
            e => panic!("{e:?}"),
        }
        assert_eq!(names(&ui), ["Alice", "bob", "Quinn"]);
        assert_eq!(ui.screen, Screen::List);
    }

    #[test]
    fn delete_cancel_returns_where_it_came_from() {
        let mut ui = sample();
        press(&mut ui, &[Input::Enter, Input::F(3), Input::Char('x')]);
        assert_eq!(ui.screen, Screen::View);
        assert_eq!(ui.status, "Not deleted");
        press(&mut ui, &[Input::F(4), Input::Char('d'), Input::Enter]);
        assert_eq!(ui.screen, Screen::List, "Enter does not confirm a delete");
        assert_eq!(ui.entries.len(), 4);
    }

    #[test]
    fn every_screen_has_back() {
        let mut ui = sample();
        press(&mut ui, &[Input::Enter]);
        press(&mut ui, &[Input::Backspace]);
        assert_eq!(ui.screen, Screen::List);
        press(&mut ui, &[Input::Char('s'), Input::F(4)]);
        assert_eq!(ui.screen, Screen::List);
        press(&mut ui, &[Input::Char('n'), Input::F(4)]);
        assert_eq!(ui.screen, Screen::List);
        assert!(matches!(press(&mut ui, &[Input::F(4)]), Effect::Leave));
    }

    #[test]
    fn basis_lock_drops_contacts_from_screen() {
        let mut ui = sample();
        press(&mut ui, &[Input::Enter]); // view Alice (c.1)
        let keep: Vec<Entry> = vec![entry("c.2", "bob", Category::Work)];
        ui.set_entries(keep, 0);
        assert_eq!(ui.screen, Screen::List);
        assert!(ui.target().is_none());
        assert_eq!(names(&ui), ["bob"]);
    }

    #[test]
    fn basis_lock_closes_edit_of_vanished_contact() {
        let mut ui = sample();
        press(&mut ui, &[Input::Enter, Input::Char('e')]);
        type_str(&mut ui, "x");
        ui.set_entries(Vec::new(), 0);
        assert_eq!(ui.screen, Screen::List);
        assert!(ui.form.name.is_empty());
    }

    #[test]
    fn basis_change_keeps_new_contact_form() {
        let mut ui = sample();
        press(&mut ui, &[Input::Char('n')]);
        type_str(&mut ui, "Fay");
        ui.set_entries(Vec::new(), 2);
        assert_eq!(ui.screen, Screen::Edit);
        assert_eq!(ui.form.name, "Fay");
        assert_eq!(ui.unreadable, 2);
    }

    #[test]
    fn saving_hidden_by_filter_clears_filter() {
        let mut ui = sample();
        press(&mut ui, &[Input::Char('s')]);
        type_str(&mut ui, "bob");
        press(&mut ui, &[Input::Enter, Input::Char('n')]);
        type_str(&mut ui, "Zara");
        if let Effect::Save(k, c) = press(&mut ui, &[Input::F(2)]) {
            ui.on_saved(k, c, None);
        }
        assert!(ui.query.is_empty());
        assert_eq!(ui.selected().unwrap().contact.name, "Zara");
    }

    #[test]
    fn empty_list_keys_are_safe() {
        let mut ui = Ui::new();
        press(&mut ui, &[Input::Up, Input::Down, Input::Enter, Input::Char('d')]);
        assert_eq!(ui.screen, Screen::List);
        assert!(ui.selected().is_none());
    }
}
