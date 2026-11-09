//! A native mod's own screen (`Screen::Mod`): opening it over the world or over a screen it
//! replaces and gives back, drawing that screen under it, closing it (docs/native-mods.md).
//! `native_mod_screen_open` and the screen hooks stay in `native_hooks`.
use super::{Game, Screen};
use horizon_client::hud::HudBatch;

/// The open mod screen (`Screen::Mod`): whose it is, and what it replaced.
#[derive(Default)]
pub(super) struct ModScreen {
    /// The mod whose own screen is open.
    pub(super) id: Option<String>,
    /// The screen the open mod screen replaced (None: it was over the world).
    under: Option<ModScreenUnder>,
}

/// The screen a native mod's own screen replaced
/// (`native_mod_screen_open_over`): what Escape returns to.
#[derive(Clone, Copy)]
pub(super) struct ModScreenUnder {
    screen: Screen,
    /// Drawn under the mod screen (the mod asked, and the screen is one that
    /// can be drawn without its input).
    drawn: bool,
    /// The panel it drew this frame (`screen_rect`), for the mod to place
    /// what it draws over it by.
    rect: Option<[f32; 4]>,
}

impl Game<'_> {
    /// Opens a screen of mod `mod_id`'s own like `native_mod_screen_open`, but
    /// over the options page (Options > Mods), the pause menu or an open
    /// inventory or container too, as vanilla's `setScreen` would replace
    /// them. The replaced screen comes back when the mod screen closes
    /// (Escape or `native_mod_screen_close`), as `lastScreen` does; a server
    /// that closes the container meanwhile closes it for good. With
    /// `keep_drawing_previous` the pause menu's title or the container's
    /// panel is still drawn under the mod screen, with no pointer over it (no
    /// slot highlight, no tooltip), and `native_mod_screen_under_rect` says
    /// where; the container takes no clicks or keys. A single-player game
    /// stays paused under a mod screen over a pausing screen. True when it is
    /// open (`mod_id`'s was already), false for the same screens as
    /// `native_mod_screen_open` and for the chat, the creative inventory and
    /// the other screens of the game's own. Bundles need "hooks": @API:mod_screen_over@.
    pub(super) fn native_mod_screen_open_over(
        &mut self,
        mod_id: &str,
        keep_drawing_previous: bool,
    ) -> bool {
        let blocked = self.mod_screen_blocked();
        let owner = self.native_mod_screen();
        if let Some(open) = mod_screen_opens_over(self.screen, blocked, owner, mod_id) {
            return open;
        }
        let under = (self.screen != Screen::Playing).then_some(ModScreenUnder {
            screen: self.screen,
            drawn: keep_drawing_previous && mod_screen_draws_under(self.screen),
            rect: None,
        });
        self.open_mod_screen(mod_id, under);
        true
    }
    /// What `screen_hooks_open` would refuse over the world: the mod's screen
    /// would be up with no hook to draw or click it.
    pub(super) fn mod_screen_blocked(&self) -> bool {
        self.title.is_some()
            || self.renderer.is_none()
            || self.lan_acting.is_some()
            || self.player.is_none()
            || self.screen_over_playing()
            || self.remote_loading()
    }
    pub(super) fn open_mod_screen(&mut self, mod_id: &str, under: Option<ModScreenUnder>) {
        self.native_mod_screen = ModScreen {
            id: Some(mod_id.to_owned()),
            under,
        };
        self.screen = Screen::Mod;
        self.keys.clear();
        self.left = false;
        self.right = false;
        self.mining = None;
        self.capture(false);
    }
    /// The panel of the screen under the open mod screen, in GUI pixels
    /// (`NativeScreenView.rect` as that screen has it), as the last frame drew
    /// it; None when the mod screen is over the world, or did not ask for the
    /// screen under it to be drawn. A container's panel is where its slots
    /// and overlays are. Bundles need "hooks": @API:mod_screen_over@.
    pub(super) fn native_mod_screen_under_rect(&self) -> Option<[f32; 4]> {
        if self.screen != Screen::Mod {
            return None;
        }
        self.native_mod_screen.under.filter(|u| u.drawn)?.rect
    }
    /// A mod's own screen closes (Escape, or `native_mod_screen_close`).
    pub(super) fn close_mod_screen(&mut self) {
        let under = std::mem::take(&mut self.native_mod_screen).under;
        if self.screen == Screen::Mod {
            // `lastScreen`: the screen it replaced comes back; the mouse
            // stays free, as that screen had it.
            self.screen = mod_screen_returns_to(under.map(|u| u.screen));
            if self.screen == Screen::Playing {
                self.capture(true);
            }
        }
    }
    /// Draws the screen under the open mod screen, if the mod asked
    /// (`native_mod_screen_open_over`): as itself, with the pointer far off so
    /// no slot lights up and no tooltip shows (its clicks are the mod
    /// screen's), then remembers its panel.
    pub(super) fn draw_mod_screen_under(&mut self, batch: &mut HudBatch) {
        let Some(under) = self.native_mod_screen.under.filter(|u| u.drawn) else {
            return;
        };
        let (screen, cursor) = (self.screen, self.cursor);
        self.screen = under.screen;
        self.cursor = (-1.0e6, -1.0e6);
        self.draw_screen(batch, under.screen);
        let rect = self.screen_rect(batch, under.screen);
        self.screen = screen;
        self.cursor = cursor;
        if let Some(under) = self.native_mod_screen.under.as_mut() {
            under.rect = Some(rect);
        }
    }
    /// A mod screen is up over the inventory or a container (a server's
    /// closing it closes the mod screen with it).
    fn mod_screen_covers_inventory(&self) -> bool {
        mod_screen_over_inventory(self.screen, self.native_mod_screen.under.map(|u| u.screen))
    }
    /// The inventory or a container is open, or a mod screen is up over it:
    /// a server's closing the container closes the mod screen with it.
    pub(super) fn screen_shows_inventory(&self) -> bool {
        self.screen == Screen::Inventory || self.mod_screen_covers_inventory()
    }
    /// The open screen, or the one an open mod screen replaced: a mod screen
    /// over a pausing screen keeps the game paused.
    pub(super) fn screen_under_mod(&self) -> Screen {
        match self.native_mod_screen.under {
            Some(under) if self.screen == Screen::Mod => under.screen,
            _ => self.screen,
        }
    }
}

/// What `native_mod_screen_open_over(mod_id)` does over `screen` (as
/// `mod_screen_opens`, which it widens): the options page, the pause menu, and
/// the inventory or a container (a server's or a block's) are replaced and
/// come back; the others are not.
fn mod_screen_opens_over(
    screen: Screen,
    blocked: bool,
    owner: Option<&str>,
    mod_id: &str,
) -> Option<bool> {
    if screen == Screen::Mod {
        return Some(owner == Some(mod_id));
    }
    // No `matches!`: horizon-mixins refuses macros in a hook point's body.
    let covered = screen == Screen::Playing
        || screen == Screen::Options
        || screen == Screen::Paused
        || screen == Screen::Inventory;
    if !covered || blocked {
        return Some(false);
    }
    None
}

/// Whether the open `screen`, with `under` the screen a mod screen replaced,
/// is the inventory or a container shown by a mod screen over it.
fn mod_screen_over_inventory(screen: Screen, under: Option<Screen>) -> bool {
    screen == Screen::Mod && under == Some(Screen::Inventory)
}

/// The screen a mod screen gives back when it closes: the one it replaced, or
/// the world.
fn mod_screen_returns_to(under: Option<Screen>) -> Screen {
    under.unwrap_or(Screen::Playing)
}

/// Whether `screen` can be drawn under a mod screen without its input: the
/// pause menu and the inventory or container panel, which draw from state
/// alone. The options page is built of widgets that need their own input.
fn mod_screen_draws_under(screen: Screen) -> bool {
    screen == Screen::Paused || screen == Screen::Inventory
}

#[cfg(test)]
mod tests {
    use super::{
        Screen, mod_screen_draws_under, mod_screen_opens_over, mod_screen_over_inventory,
        mod_screen_returns_to,
    };

    /// A mod's own screen can also replace the options page, the pause menu
    /// and the inventory or a container (they come back), but still not
    /// another mod's screen, an overlay or any other screen of the game's.
    #[test]
    fn a_mod_screen_opens_over_a_screen_it_can_give_back() {
        for covered in [
            Screen::Playing,
            Screen::Options,
            Screen::Paused,
            Screen::Inventory,
        ] {
            assert_eq!(mod_screen_opens_over(covered, false, None, "a"), None);
            assert_eq!(mod_screen_opens_over(covered, true, None, "a"), Some(false));
        }
        for other in [
            Screen::Chat,
            Screen::Creative,
            Screen::Dead,
            Screen::Stats,
            Screen::CommandBlock,
        ] {
            assert_eq!(mod_screen_opens_over(other, false, None, "a"), Some(false));
        }
        assert_eq!(
            mod_screen_opens_over(Screen::Mod, false, Some("a"), "a"),
            Some(true)
        );
        assert_eq!(
            mod_screen_opens_over(Screen::Mod, false, Some("b"), "a"),
            Some(false)
        );
        // A server closing the container closes the mod screen over it; the
        // screen it gives back is the replaced one, or the world.
        assert!(mod_screen_over_inventory(
            Screen::Mod,
            Some(Screen::Inventory)
        ));
        assert!(!mod_screen_over_inventory(
            Screen::Mod,
            Some(Screen::Paused)
        ));
        assert!(!mod_screen_over_inventory(Screen::Mod, None));
        assert!(!mod_screen_over_inventory(
            Screen::Inventory,
            Some(Screen::Inventory)
        ));
        assert!(mod_screen_returns_to(Some(Screen::Options)) == Screen::Options);
        assert!(mod_screen_returns_to(None) == Screen::Playing);
        // Only what draws from state alone is kept on screen.
        assert!(mod_screen_draws_under(Screen::Inventory));
        assert!(mod_screen_draws_under(Screen::Paused));
        assert!(!mod_screen_draws_under(Screen::Options));
        assert!(!mod_screen_draws_under(Screen::Playing));
    }
}
