//! Turns the tray menu model into `tray-icon` menu items.

use tray_icon::menu::{IsMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};

use crate::logic::MenuEntry;

pub fn build_menu(entries: &[MenuEntry]) -> Menu {
    let menu = Menu::new();
    for item in menu_items(entries) {
        let _ = menu.append(item.as_ref());
    }
    menu
}

fn menu_items(entries: &[MenuEntry]) -> Vec<Box<dyn IsMenuItem>> {
    entries
        .iter()
        .map(|entry| -> Box<dyn IsMenuItem> {
            match entry {
                MenuEntry::Item { label, action } => {
                    Box::new(MenuItem::with_id(action.id(), label, true, None))
                }
                MenuEntry::Text(text) => Box::new(MenuItem::new(text, false, None)),
                MenuEntry::Submenu { label, entries } => {
                    let items = menu_items(entries);
                    let items: Vec<&dyn IsMenuItem> = items.iter().map(AsRef::as_ref).collect();
                    Box::new(Submenu::with_items(label, true, &items).expect("tray submenu"))
                }
                MenuEntry::Separator => Box::new(PredefinedMenuItem::separator()),
            }
        })
        .collect()
}
