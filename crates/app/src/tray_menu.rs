//! The tray: turns the tray menu model into `tray-icon` menu items, and draws the icon.

use resvg::tiny_skia::{Pixmap, Transform};
use resvg::usvg::Tree;
use tray_icon::Icon;
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

/// The Ebb tray icon at the system's small icon size, grey while breaks are paused.
pub(crate) fn tray_icon(paused: bool) -> Icon {
    let svg: &[u8] = if paused {
        include_bytes!("../ui/icons/ebb-tray-paused.svg")
    } else {
        include_bytes!("../ui/icons/ebb-tray.svg")
    };
    let size = platform::small_icon_size();
    let tree = Tree::from_data(svg, &resvg::usvg::Options::default()).expect("bundled tray icon");
    let mut pixmap = Pixmap::new(size, size).expect("tray icon size");
    let scale = size as f32 / tree.size().width();
    resvg::render(
        &tree,
        Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    // tiny-skia stores premultiplied alpha; Windows icons take straight alpha.
    let rgba = pixmap
        .pixels()
        .iter()
        .flat_map(|pixel| {
            let colour = pixel.demultiply();
            [colour.red(), colour.green(), colour.blue(), colour.alpha()]
        })
        .collect();
    Icon::from_rgba(rgba, size, size).expect("tray icon image")
}
