//! Port of source/interface/localinterface/keyboardinterface.h + source/interface/localinterface/keyboardinterface.cpp
//! On-screen keyboard: types into the caption of a target button.

use std::cell::RefCell;

use crate::ctx::Ctx;
use crate::eclipse::Button;
use crate::impl_as_any;
use crate::interface::interfacescreen::InterfaceScreen;
use crate::interface::localinterface::localinterfacescreen;

thread_local! {
    /// C++ `static char *KeyboardInterface::targetbuttonname`: the button whose caption the keys edit.
    // PORT: a thread-local rather than a field, because the static Keypress callback cannot reach an instance.
    static TARGET_BUTTON_NAME: RefCell<Option<String>> = const { RefCell::new(None) };
}

#[derive(Debug, Clone, Default)]
pub struct KeyboardInterface {}

/// The key letter for grid cell (`dy`, `dx`): `'a' + dy * 9 + dx`. The cell (2, 8) is never used.
fn key_letter(dy: i32, dx: i32) -> char {
    (b'a' + (dy * 9 + dx) as u8) as char
}

/// C++ `RemoveKeyboardInterface` body, shared by the close key and the trait `remove`.
fn remove_keyboard_buttons(g: &mut Ctx) {
    for dy in 0..3 {
        for dx in 0..9 {
            if dy != 2 || dx != 8 {
                let name = format!("keyboard {}", key_letter(dy, dx));
                g.ecl.remove_button(&name);
            }
        }
    }
    g.ecl.remove_button("keyboard back");
    g.ecl.remove_button("keyboard clear");
    g.ecl.remove_button("keyboard space");
    g.ecl.remove_button("keyboard close");
}

impl KeyboardInterface {
    /// C++ `KeyboardInterface::RemoveKeyboardInterface` (static button callback: the close key).
    pub fn remove_keyboard_interface(g: &mut Ctx, _button: &Button) {
        remove_keyboard_buttons(g);
    }

    /// C++ `KeyboardInterface::Keypress` (static button callback for every key).
    pub fn keypress(g: &mut Ctx, button: &Button) {
        // C++ `sscanf(button->name, "keyboard %s", letter)`
        let letter = button.name.strip_prefix("keyboard ").unwrap_or("").to_string();

        let target = TARGET_BUTTON_NAME.with(|t| t.borrow().clone());
        let Some(target) = target else { return };
        let Some(tbutton) = g.ecl.get_button_mut(&target) else { return };

        let mut newcaption = tbutton.caption.clone();
        match letter.as_str() {
            "back" => {
                newcaption.pop();
            }
            "clear" => {
                newcaption.clear();
            }
            "space" => {
                newcaption.push(' ');
            }
            _ => {
                newcaption.push_str(&letter);
            }
        }
        tbutton.set_caption(&newcaption);
    }

    /// C++ `KeyboardInterface::SetTargetButton`
    pub fn set_target_button(&mut self, bname: &str) {
        TARGET_BUTTON_NAME.with(|t| *t.borrow_mut() = Some(bname.to_string()));
    }

    /// C++ `KeyboardInterface::Create(int x, int y)`: the keyboard needs a position, so it is
    /// not started through `InterfaceScreen::create`.
    pub fn create_at(&mut self, g: &mut Ctx, x: i32, y: i32) {
        if self.is_visible(g) {
            return;
        }

        debug_assert!(TARGET_BUTTON_NAME.with(|t| t.borrow().is_some()), "UplinkAssert(targetbuttonname)");

        for dy in 0..3 {
            for dx in 0..9 {
                if dy != 2 || dx != 8 {
                    let letter = key_letter(dy, dx).to_string();
                    let name = format!("keyboard {letter}");
                    g.ecl.register_button(x + dx * 20, y + dy * 20, 18, 18, &letter, "Press this key", &name);
                    g.ecl.register_button_callback(&name, Self::keypress);
                }
            }
        }

        g.ecl.register_button(x, y + 60, 38, 18, "Back", "Delete the last character", "keyboard back");
        g.ecl.register_button(x + 40, y + 60, 38, 18, "Clear", "Clear the text box", "keyboard clear");
        g.ecl.register_button(x + 80, y + 60, 38, 18, "Space", "Add a space", "keyboard space");
        g.ecl.register_button(x + 120, y + 60, 38, 18, "Close", "Close the keyboard", "keyboard close");
        g.ecl.register_button_callback("keyboard back", Self::keypress);
        g.ecl.register_button_callback("keyboard clear", Self::keypress);
        g.ecl.register_button_callback("keyboard space", Self::keypress);
        g.ecl.register_button_callback("keyboard close", Self::remove_keyboard_interface);
    }
}

impl InterfaceScreen for KeyboardInterface {
    /// C++ `KeyboardInterface::Create(int x, int y)` hides, but does not override, the virtual `Create()`,
    /// so the virtual one is still `LocalInterfaceScreen::Create` (the panel background, guarded
    /// by this class's `IsVisible`). The real entry point is [`KeyboardInterface::create_at`].
    fn create(&mut self, g: &mut Ctx) {
        if !self.is_visible(g) {
            localinterfacescreen::create(g);
        }
    }

    fn remove(&mut self, g: &mut Ctx) {
        // C++ `Remove()` is `RemoveKeyboardInterface(NULL)`.
        remove_keyboard_buttons(g);
    }

    fn is_visible(&self, g: &Ctx) -> bool {
        g.ecl.get_button("keyboard back").is_some()
    }

    impl_as_any!();
}
