//! Drawing, storage and TRNG plumbing for Contact Book. All decisions are
//! made by `contacts_core::Ui`; this file maps keys to it, carries out its
//! effects, and draws its state.

use contacts_core::contact::is_record_key;
use contacts_core::{Contact, Effect, Entropy, EntropyError, Entry, Field, Input, Screen, Ui};
use gam::menu::Point;
use gam::GlyphStyle;
use precursor_appkit::keys::Key;
use precursor_appkit::layout::ListState;
use precursor_appkit::rng::Trng;
use precursor_appkit::screen::{Screen as Canvas, MARGIN};
use precursor_appkit::store::Store;
use zeroize::Zeroizing;

/// PDDB dictionary. One key per contact (`c.<16 hex>`).
pub const DICT: &str = "contacts.data";

#[derive(PartialEq, Eq)]
pub enum Action {
    Stay,
    Leave,
}

/// The hardware TRNG as a fallible core entropy source (used only to name
/// new contact records). An IPC error means the contact isn't saved.
struct TrngSource<'a>(&'a Trng);

impl Entropy for TrngSource<'_> {
    fn try_u32(&mut self) -> Result<u32, EntropyError> { self.0.try_u32().map_err(|_| EntropyError) }
}

fn to_input(key: Key) -> Input {
    match key {
        Key::Up => Input::Up,
        Key::Down => Input::Down,
        Key::Left => Input::Left,
        Key::Right => Input::Right,
        Key::Enter => Input::Enter,
        Key::Backspace => Input::Backspace,
        Key::F(n) => Input::F(n),
        Key::Char(c) => Input::Char(c),
    }
}

pub struct App {
    canvas: Canvas,
    store: Store,
    trng: Trng,
    ui: Ui,
    list: ListState,
    visible_rows: usize,
}

impl App {
    pub fn new(xns: &xous_names::XousNames, canvas: Canvas, store: Store) -> Self {
        let trng = Trng::new(xns).expect("can't connect to TRNG");
        let mut app = Self { canvas, store, trng, ui: Ui::new(), list: ListState::default(), visible_rows: 1 };
        app.reload();
        app
    }

    pub fn screen(&self) -> &Canvas { &self.canvas }

    /// (Re)load all contacts. Runs at start and whenever a basis is locked or
    /// unlocked, so contacts from a just-locked basis leave memory and screen,
    /// and contacts in a just-unlocked basis appear. Unreadable records are
    /// counted and never rewritten.
    pub fn reload(&mut self) {
        let mut entries = Vec::new();
        let mut unreadable = 0;
        match self.store.keys() {
            Ok(names) => {
                for name in names.into_iter().filter(|n| is_record_key(n)) {
                    match self.store.read_bytes(&name) {
                        Ok(Some(bytes)) => {
                            let bytes = Zeroizing::new(bytes);
                            match Contact::parse(&bytes) {
                                Ok(contact) => {
                                    let basis = self.store.basis_of(&name).ok().flatten();
                                    entries.push(Entry { key: name, contact, basis });
                                }
                                Err(e) => {
                                    log::warn!("record {} unreadable: {:?}", name, e);
                                    unreadable += 1;
                                }
                            }
                        }
                        Ok(None) => {}
                        Err(e) => {
                            log::warn!("couldn't read {}: {:?}", name, e.kind());
                            unreadable += 1;
                        }
                    }
                }
            }
            Err(e) => {
                log::error!("couldn't list keys: {:?}", e.kind());
                self.ui.status = "Couldn't read contacts".into();
            }
        }
        self.ui.set_entries(entries, unreadable);
        self.draw();
    }

    pub fn on_key(&mut self, key: Key) -> Action {
        match self.ui.on_key(to_input(key), &mut TrngSource(&self.trng)) {
            Effect::None => {}
            Effect::Leave => return Action::Leave,
            Effect::Save(key, contact) => self.save(key, contact),
            Effect::Delete(key) => match self.store.delete(&key) {
                Ok(()) => self.ui.on_deleted(&key),
                Err(e) => {
                    log::error!("delete failed: {:?}", e.kind());
                    self.ui.on_delete_failed(&e.kind().to_string());
                }
            },
        }
        Action::Stay
    }

    /// Write one contact. An existing contact is rewritten in its own basis
    /// (appkit Store deletes and recreates the key there); a new one goes to
    /// the most recently unlocked basis.
    fn save(&mut self, key: String, contact: Contact) {
        let bytes = match serde_json::to_vec(&contact) {
            Ok(b) => Zeroizing::new(b),
            Err(_) => {
                self.ui.on_save_failed("encoding");
                return;
            }
        };
        match self.store.write_bytes(&key, &bytes) {
            Ok(()) => {
                let basis = self.store.basis_of(&key).ok().flatten();
                self.ui.on_saved(key, contact, basis);
            }
            Err(e) => {
                log::error!("save failed: {:?}", e.kind());
                self.ui.on_save_failed(&e.kind().to_string());
            }
        }
    }

    pub fn on_foreground(&mut self) {
        self.canvas.refresh_size();
        self.draw();
    }

    fn footer_and_status(&self, hint: &str) -> isize {
        let bottom = self.canvas.footer(hint);
        if self.ui.status.is_empty() {
            return bottom;
        }
        let h = self.canvas.line_height(GlyphStyle::Small);
        self.canvas.text(bottom - h, GlyphStyle::Small, &self.ui.status);
        bottom - h
    }

    /// Wrapped text in a box from `y` to `bottom`. Returns the next free y.
    fn para(&self, y: isize, bottom: isize, style: GlyphStyle, text: &str) -> isize {
        if y >= bottom {
            return y;
        }
        let w = self.canvas.size().x;
        y + self.canvas.text_in(Point::new(MARGIN, y), Point::new(w - MARGIN, bottom), style, text) + 4
    }

    pub fn draw(&mut self) {
        self.canvas.clear();
        match self.ui.screen {
            Screen::List => self.draw_list("⏎ view · N new · S search · O sort · D del · ⌫ quit"),
            Screen::Search => self.draw_search(),
            Screen::View => self.draw_view(),
            Screen::Edit => self.draw_edit(),
            Screen::ConfirmDelete => self.draw_confirm_delete(),
            Screen::ConfirmDiscard => self.draw_confirm_discard(),
        }
        self.canvas.flush();
    }

    fn draw_contacts(&mut self, top: isize, bottom: isize) {
        let n = self.ui.filtered.len();
        if n == 0 {
            let msg = if self.ui.entries.is_empty() { "No contacts yet. Press N to add one." } else { "No matches." };
            self.para(top + 10, bottom, GlyphStyle::Regular, msg);
            return;
        }
        let labels: Vec<Zeroizing<String>> =
            self.ui.filtered.iter().map(|&i| Zeroizing::new(self.ui.entries[i].contact.list_line())).collect();
        let labels: Vec<&str> = labels.iter().map(|s| s.as_str()).collect();
        self.list.selected = self.ui.cursor;
        self.list.clamp(n, self.visible_rows);
        self.visible_rows = self.canvas.list(top, bottom, &labels, &self.list).max(1);
    }

    fn draw_list(&mut self, hint: &str) {
        let mut top = self.canvas.header(&format!("Contacts ({}) · {}", self.ui.entries.len(), self.ui.sort.label()));
        let mut bottom = self.footer_and_status(hint);
        let small = self.canvas.line_height(GlyphStyle::Small);
        if !self.ui.query.is_empty() {
            let q = Zeroizing::new(format!("Filter \"{}\": {} match(es) · ⌫ clears", self.ui.query, self.ui.filtered.len()));
            top += self.canvas.text(top, GlyphStyle::Small, &q);
        }
        if self.ui.unreadable > 0 {
            self.canvas.text(bottom - small, GlyphStyle::Small, &format!("{} unreadable record(s) left untouched", self.ui.unreadable));
            bottom -= small;
        }
        self.draw_contacts(top, bottom);
    }

    fn draw_search(&mut self) {
        let mut y = self.canvas.header("Search contacts");
        let bottom = self.footer_and_status("type · ⏎ done · ⌫ delete/cancel · F4 cancel");
        y += self.canvas.text(y, GlyphStyle::Small, "Name, phone, email or notes:");
        let mut q = Zeroizing::new(self.ui.query.clone());
        q.push('_');
        let h = self.canvas.line_height(GlyphStyle::Regular);
        let w = self.canvas.size().x;
        self.canvas.frame(Point::new(MARGIN - 2, y - 2), Point::new(w - MARGIN + 1, y + h + 2), 1);
        y += self.canvas.text(y, GlyphStyle::Regular, &q) + 6;
        y += self.canvas.text(y, GlyphStyle::Small, &format!("{} match(es)", self.ui.filtered.len()));
        self.draw_contacts(y, bottom);
    }

    fn draw_view(&self) {
        let Some(e) = self.ui.target() else { return };
        let c = &e.contact;
        let mut y = self.canvas.header(&c.name);
        let bottom = self.footer_and_status("E edit · D delete · ⌫ back");
        let basis = e.basis.as_deref().unwrap_or("?");
        y += self.canvas.text(y, GlyphStyle::Small, &format!("{} · basis {}", c.category.label(), basis));
        y += 4;
        for (label, value) in [("Phone", &c.phone), ("Email", &c.email)] {
            if !value.is_empty() {
                let line = Zeroizing::new(format!("{label}: {value}"));
                y = self.para(y, bottom, GlyphStyle::Regular, &line);
            }
        }
        if !c.notes.is_empty() {
            y += self.canvas.text(y, GlyphStyle::Small, "Notes:");
            self.para(y, bottom, GlyphStyle::Regular, &c.notes);
        }
    }

    fn draw_edit(&self) {
        let title = if self.ui.is_new() { "New contact" } else { "Edit contact" };
        let mut y = self.canvas.header(title);
        let bottom = self.footer_and_status("↑↓ field · ⏎ next · F2 save · F4 cancel");
        let w = self.canvas.size().x;
        let row_h = self.canvas.line_height(GlyphStyle::Regular) + 6;
        let f = &self.ui.form;
        for (i, field) in Field::ALL.iter().enumerate() {
            let selected = i == self.ui.field;
            let mut line = Zeroizing::new(String::new());
            line.push_str(if selected { "▸ " } else { "  " });
            match field {
                Field::Save => line.push_str("[ Save ]"),
                Field::Category => line.push_str(&format!("Category: ◂ {} ▸", f.category.label())),
                _ => {
                    let v = match field {
                        Field::Name => &f.name,
                        Field::Phone => &f.phone,
                        Field::Email => &f.email,
                        _ => &f.notes,
                    };
                    line.push_str(field.label());
                    line.push_str(": ");
                    line.push_str(v);
                    if selected {
                        line.push('_');
                    }
                }
            }
            // Notes may wrap; give it extra room.
            let h = if *field == Field::Notes { row_h * 3 } else { row_h };
            if y + h > bottom {
                break;
            }
            self.canvas.text_in(Point::new(MARGIN + 2, y + 3), Point::new(w - MARGIN - 2, y + h - 3), GlyphStyle::Regular, &line);
            if selected {
                self.canvas.frame(Point::new(MARGIN - 2, y), Point::new(w - MARGIN + 1, y + h - 1), 1);
            }
            y += h + 2;
        }
        if y < bottom && self.ui.is_new() {
            let target = self.store.target_basis().unwrap_or_else(|| "?".into());
            self.canvas.text(y, GlyphStyle::Small, &format!("New contacts go to basis: {target}"));
        }
    }

    fn draw_confirm_delete(&self) {
        let Some(e) = self.ui.target() else { return };
        let mut y = self.canvas.header("Delete contact?");
        let bottom = self.footer_and_status("Y delete · any other key cancels");
        y += self.canvas.text(y, GlyphStyle::Bold, &e.contact.name);
        y += 6;
        self.para(y, bottom, GlyphStyle::Small, "Removes the PDDB key. Old flash pages are not overwritten immediately.");
    }

    fn draw_confirm_discard(&self) {
        let mut y = self.canvas.header("Discard changes?");
        self.footer_and_status("Y discard · any other key keeps editing");
        y += 6;
        self.canvas.text(y, GlyphStyle::Regular, "Unsaved changes will be lost.");
    }
}
