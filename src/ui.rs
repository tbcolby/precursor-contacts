//! UI rendering for Contact Book.

use crate::app::{AppState, ContactsApp, EditField, SortMode};
use crate::contact::Contact;

use gam::*;

const SCREEN_WIDTH: isize = 336;
const HEADER_HEIGHT: isize = 30;
const FOOTER_HEIGHT: isize = 46;
const CONTENT_TOP: isize = HEADER_HEIGHT;
const CONTENT_BOTTOM: isize = 536 - FOOTER_HEIGHT;

const REGULAR_HEIGHT: isize = 15;
const LINE_GAP: isize = 4;
const LINE_HEIGHT: isize = REGULAR_HEIGHT + LINE_GAP;

pub fn draw(app: &ContactsApp, gam: &Gam, canvas: graphics_server::Gid) {
    let clear_rect = graphics_server::Rectangle::new_coords_with_style(
        0, 0, SCREEN_WIDTH, 536,
        graphics_server::DrawStyle::new(
            graphics_server::PixelColor::Light,
            graphics_server::PixelColor::Light,
            0,
        ),
    );
    gam.draw_rectangle(canvas, clear_rect).ok();

    match app.state {
        AppState::List => draw_list(app, gam, canvas),
        AppState::View => draw_view(app, gam, canvas),
        AppState::Add => draw_edit(app, gam, canvas, "New Contact"),
        AppState::Edit => draw_edit(app, gam, canvas, "Edit Contact"),
        AppState::Search => draw_search(app, gam, canvas),
        AppState::ConfirmDelete => draw_confirm_delete(app, gam, canvas),
    }

    gam.redraw().ok();
}

fn draw_header(gam: &Gam, canvas: graphics_server::Gid, title: &str) {
    let bg = graphics_server::Rectangle::new_coords_with_style(
        0, 0, SCREEN_WIDTH, HEADER_HEIGHT,
        graphics_server::DrawStyle::new(
            graphics_server::PixelColor::Dark,
            graphics_server::PixelColor::Dark,
            0,
        ),
    );
    gam.draw_rectangle(canvas, bg).ok();

    let mut tv = TextView::new(
        canvas,
        TextBounds::BoundingBox(graphics_server::Rectangle::new_coords(
            8, 2, SCREEN_WIDTH - 8, HEADER_HEIGHT - 2,
        )),
    );
    tv.style = GlyphStyle::Bold;
    tv.invert = true;
    tv.draw_border = false;
    tv.margin = Point::new(0, 0);
    write!(tv, "{}", title).ok();
    gam.post_textview(&mut tv).ok();
}

fn draw_footer(gam: &Gam, canvas: graphics_server::Gid, text: &str) {
    let y = CONTENT_BOTTOM;
    let sep = graphics_server::Line::new_with_style(
        Point::new(0, y), Point::new(SCREEN_WIDTH, y),
        graphics_server::DrawStyle::new(
            graphics_server::PixelColor::Dark,
            graphics_server::PixelColor::Dark, 1,
        ),
    );
    gam.draw_line(canvas, sep).ok();

    let mut tv = TextView::new(
        canvas,
        TextBounds::BoundingBox(graphics_server::Rectangle::new_coords(
            8, y + 4, SCREEN_WIDTH - 8, y + FOOTER_HEIGHT - 4,
        )),
    );
    tv.style = GlyphStyle::Small;
    tv.draw_border = false;
    tv.margin = Point::new(0, 0);
    write!(tv, "{}", text).ok();
    gam.post_textview(&mut tv).ok();
}

fn draw_text(gam: &Gam, canvas: graphics_server::Gid,
    x: isize, y: isize, w: isize, text: &str, style: GlyphStyle) {
    let mut tv = TextView::new(
        canvas,
        TextBounds::BoundingBox(graphics_server::Rectangle::new_coords(
            x, y, x + w, y + LINE_HEIGHT,
        )),
    );
    tv.style = style;
    tv.draw_border = false;
    tv.margin = Point::new(0, 0);
    write!(tv, "{}", text).ok();
    gam.post_textview(&mut tv).ok();
}

// ─── Contact List ───────────────────────────────────────────────────────────

fn draw_list(app: &ContactsApp, gam: &Gam, canvas: graphics_server::Gid) {
    let sort_label = match app.sort_mode {
        SortMode::Alpha => "A-Z",
        SortMode::ByCategory => "Cat",
    };
    let title = alloc::format!("Contacts ({}) [{}]", app.contacts.len(), sort_label);
    draw_header(gam, canvas, &title);

    let mut y = CONTENT_TOP + 6;

    if !app.search_query.is_empty() {
        let filter = alloc::format!("Filter: \"{}\" ({} matches)",
            app.search_query, app.filtered.len());
        draw_text(gam, canvas, 16, y, 300, &filter, GlyphStyle::Small);
        y += LINE_HEIGHT + 2;
    }

    if app.filtered.is_empty() {
        if app.contacts.is_empty() {
            draw_text(gam, canvas, 16, y + 20, 300,
                "No contacts yet.", GlyphStyle::Regular);
            draw_text(gam, canvas, 16, y + 50, 300,
                "Press N to add one.", GlyphStyle::Bold);
        } else {
            draw_text(gam, canvas, 16, y + 20, 300,
                "No matches found.", GlyphStyle::Regular);
        }
    } else {
        let end = (app.list_scroll + 18).min(app.filtered.len());
        for vi in app.list_scroll..end {
            let ci = app.filtered[vi];
            let contact = &app.contacts[ci];
            let selected = vi == app.list_cursor;

            if selected {
                let hl = graphics_server::Rectangle::new_coords_with_style(
                    4, y - 1, SCREEN_WIDTH - 4, y + LINE_HEIGHT + 1,
                    graphics_server::DrawStyle::new(
                        graphics_server::PixelColor::Dark,
                        graphics_server::PixelColor::Dark, 0,
                    ),
                );
                gam.draw_rectangle(canvas, hl).ok();
            }

            // Category tag + name + phone
            let cat = contact.category.short();
            let line = if !contact.phone.is_empty() {
                alloc::format!("[{}] {} - {}", cat, contact.name, contact.phone)
            } else {
                alloc::format!("[{}] {}", cat, contact.name)
            };

            let mut tv = TextView::new(
                canvas,
                TextBounds::BoundingBox(graphics_server::Rectangle::new_coords(
                    8, y, SCREEN_WIDTH - 8, y + LINE_HEIGHT,
                )),
            );
            tv.style = GlyphStyle::Small;
            tv.invert = selected;
            tv.draw_border = false;
            tv.margin = Point::new(0, 0);
            write!(tv, "{}", line).ok();
            gam.post_textview(&mut tv).ok();

            y += LINE_HEIGHT + 2;
        }
    }

    draw_footer(gam, canvas, "Enter:View N:New S:Search O:Sort D:Del Q:Quit");
}

// ─── Contact View ───────────────────────────────────────────────────────────

fn draw_view(app: &ContactsApp, gam: &Gam, canvas: graphics_server::Gid) {
    let contact = match app.contacts.get(app.viewed_index) {
        Some(c) => c,
        None => {
            draw_header(gam, canvas, "Contact");
            draw_text(gam, canvas, 16, CONTENT_TOP + 20, 300,
                "Contact not found.", GlyphStyle::Regular);
            draw_footer(gam, canvas, "Q:Back");
            return;
        }
    };

    draw_header(gam, canvas, "Contact Details");

    let mut y = CONTENT_TOP + 15;

    // Name (large)
    let mut tv = TextView::new(
        canvas,
        TextBounds::BoundingBox(graphics_server::Rectangle::new_coords(
            16, y, SCREEN_WIDTH - 16, y + 24,
        )),
    );
    tv.style = GlyphStyle::Bold;
    tv.draw_border = false;
    tv.margin = Point::new(0, 0);
    write!(tv, "{}", contact.name).ok();
    gam.post_textview(&mut tv).ok();
    y += 28;

    // Category
    let cat_line = alloc::format!("Category: {}", contact.category.label());
    draw_text(gam, canvas, 16, y, 300, &cat_line, GlyphStyle::Small);
    y += LINE_HEIGHT + 8;

    // Separator
    let sep = graphics_server::Line::new_with_style(
        Point::new(16, y), Point::new(SCREEN_WIDTH - 16, y),
        graphics_server::DrawStyle::new(
            graphics_server::PixelColor::Dark,
            graphics_server::PixelColor::Dark, 1,
        ),
    );
    gam.draw_line(canvas, sep).ok();
    y += 8;

    // Fields
    if !contact.phone.is_empty() {
        draw_text(gam, canvas, 16, y, 80, "Phone:", GlyphStyle::Small);
        draw_text(gam, canvas, 80, y, 240, &contact.phone, GlyphStyle::Regular);
        y += LINE_HEIGHT + 6;
    }

    if !contact.email.is_empty() {
        draw_text(gam, canvas, 16, y, 80, "Email:", GlyphStyle::Small);
        draw_text(gam, canvas, 80, y, 240, &contact.email, GlyphStyle::Regular);
        y += LINE_HEIGHT + 6;
    }

    if !contact.notes.is_empty() {
        draw_text(gam, canvas, 16, y, 80, "Notes:", GlyphStyle::Small);
        y += LINE_HEIGHT;

        let mut tv = TextView::new(
            canvas,
            TextBounds::BoundingBox(graphics_server::Rectangle::new_coords(
                24, y, SCREEN_WIDTH - 16, y + 80,
            )),
        );
        tv.style = GlyphStyle::Small;
        tv.draw_border = false;
        tv.margin = Point::new(0, 0);
        write!(tv, "{}", contact.notes).ok();
        gam.post_textview(&mut tv).ok();
    }

    draw_footer(gam, canvas, "E:Edit  D:Delete  Q:Back");
}

// ─── Edit/Add Screen ────────────────────────────────────────────────────────

fn draw_edit(app: &ContactsApp, gam: &Gam, canvas: graphics_server::Gid, title: &str) {
    draw_header(gam, canvas, title);

    let contact = match &app.edit_contact {
        Some(c) => c,
        None => return,
    };

    let fields = EditField::all();
    let mut y = CONTENT_TOP + 10;

    for (i, field) in fields.iter().enumerate() {
        let selected = i == app.edit_field_cursor;
        let is_editing = selected && app.editing_field;

        // Field label
        let marker = if selected { ">" } else { " " };

        match field {
            EditField::Save => {
                y += 8;
                if selected {
                    let hl = graphics_server::Rectangle::new_coords_with_style(
                        16, y, 120, y + LINE_HEIGHT + 4,
                        graphics_server::DrawStyle::new(
                            graphics_server::PixelColor::Dark,
                            graphics_server::PixelColor::Dark, 0,
                        ),
                    );
                    gam.draw_rectangle(canvas, hl).ok();
                }
                let mut tv = TextView::new(
                    canvas,
                    TextBounds::BoundingBox(graphics_server::Rectangle::new_coords(
                        20, y + 2, 116, y + LINE_HEIGHT + 2,
                    )),
                );
                tv.style = GlyphStyle::Bold;
                tv.invert = selected;
                tv.draw_border = false;
                tv.margin = Point::new(0, 0);
                write!(tv, "  SAVE").ok();
                gam.post_textview(&mut tv).ok();
                y += LINE_HEIGHT + 12;
            }
            EditField::Category => {
                let label = alloc::format!("{} {}: {}",
                    marker, field.label(), contact.category.label());
                draw_text(gam, canvas, 12, y, 310, &label,
                    if selected { GlyphStyle::Bold } else { GlyphStyle::Regular });
                y += LINE_HEIGHT + 6;
            }
            _ => {
                let value = match field {
                    EditField::Name => &contact.name,
                    EditField::Phone => &contact.phone,
                    EditField::Email => &contact.email,
                    EditField::Notes => &contact.notes,
                    _ => unreachable!(),
                };

                let label = alloc::format!("{} {}:", marker, field.label());
                draw_text(gam, canvas, 12, y, 310, &label,
                    if selected { GlyphStyle::Bold } else { GlyphStyle::Small });
                y += LINE_HEIGHT;

                // Value box
                let box_style = if is_editing {
                    graphics_server::DrawStyle::new(
                        graphics_server::PixelColor::Dark,
                        graphics_server::PixelColor::Light, 2,
                    )
                } else {
                    graphics_server::DrawStyle::new(
                        graphics_server::PixelColor::Dark,
                        graphics_server::PixelColor::Light, 1,
                    )
                };
                let rect = graphics_server::Rectangle::new_coords_with_style(
                    20, y, SCREEN_WIDTH - 16, y + LINE_HEIGHT + 4, box_style);
                gam.draw_rectangle(canvas, rect).ok();

                let display = if is_editing {
                    alloc::format!("{}|", value)
                } else {
                    value.clone()
                };
                draw_text(gam, canvas, 24, y + 2, 284, &display, GlyphStyle::Regular);
                y += LINE_HEIGHT + 10;
            }
        }
    }

    let hint = if app.editing_field {
        "Type to edit  Enter:Done  Bksp:Delete"
    } else {
        "Arrows:Navigate  Enter:Edit/Save  C:Category  Q:Cancel"
    };
    draw_footer(gam, canvas, hint);
}

// ─── Search Screen ──────────────────────────────────────────────────────────

fn draw_search(app: &ContactsApp, gam: &Gam, canvas: graphics_server::Gid) {
    draw_header(gam, canvas, "Search Contacts");

    let mut y = CONTENT_TOP + 20;
    draw_text(gam, canvas, 16, y, 300, "Type to search:", GlyphStyle::Regular);
    y += LINE_HEIGHT + 8;

    // Search box
    let rect = graphics_server::Rectangle::new_coords_with_style(
        16, y, SCREEN_WIDTH - 16, y + LINE_HEIGHT + 8,
        graphics_server::DrawStyle::new(
            graphics_server::PixelColor::Dark,
            graphics_server::PixelColor::Light, 2,
        ),
    );
    gam.draw_rectangle(canvas, rect).ok();

    let display = alloc::format!("{}|", app.search_query);
    draw_text(gam, canvas, 20, y + 4, 290, &display, GlyphStyle::Regular);
    y += LINE_HEIGHT + 20;

    // Show match count
    let matches = alloc::format!("{} match{}",
        app.filtered.len(),
        if app.filtered.len() != 1 { "es" } else { "" });
    draw_text(gam, canvas, 16, y, 300, &matches, GlyphStyle::Small);

    draw_footer(gam, canvas, "Enter:Apply  Bksp:Delete  Q:Cancel");
}

// ─── Confirm Delete ─────────────────────────────────────────────────────────

fn draw_confirm_delete(app: &ContactsApp, gam: &Gam, canvas: graphics_server::Gid) {
    draw_header(gam, canvas, "Delete Contact?");

    let name = app.selected_contact_index()
        .and_then(|i| app.contacts.get(i))
        .map(|c| c.name.as_str())
        .unwrap_or("Unknown");

    let mut y = CONTENT_TOP + 60;
    draw_text(gam, canvas, 16, y, 300, "Delete this contact?", GlyphStyle::Regular);
    y += LINE_HEIGHT + 12;

    let mut tv = TextView::new(
        canvas,
        TextBounds::BoundingBox(graphics_server::Rectangle::new_coords(
            40, y, SCREEN_WIDTH - 40, y + 24,
        )),
    );
    tv.style = GlyphStyle::Bold;
    tv.draw_border = false;
    tv.margin = Point::new(0, 0);
    write!(tv, "{}", name).ok();
    gam.post_textview(&mut tv).ok();
    y += 40;

    draw_text(gam, canvas, 16, y, 300, "This cannot be undone.", GlyphStyle::Small);

    draw_footer(gam, canvas, "Y:Yes, delete  Any other key:Cancel");
}
