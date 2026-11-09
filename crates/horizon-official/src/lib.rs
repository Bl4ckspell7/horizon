//! The official game: the platform's game (`horizon-game`) with Horizon's own in-game parts
//! plugged in through the platform's hooks (docs/layers.md, "How a Horizon crate plugs in").
//! The release builds ship its executable under the game's name, `horizon-client`
//! (`game-build.yml`); horizon-client keeps its own plain executable.
//!
//! Each Horizon crate that runs in the game gives a `&'static` value implementing
//! `horizon_platform::hooks::Hooks`, and [`register`] adds it. Nothing here depends on an engine
//! crate.
#![forbid(unsafe_code)]

use horizon_platform::{
    hooks::{self, Hooks},
    lang,
    pages::{self, Page},
};

/// Every Horizon part in the game, in the order its hooks run.
const PARTS: [&dyn Hooks; 2] = [
    // The anonymous "I'm playing" ping.
    &horizon_heartbeat::HOOKS,
    // Play with Friends: `friend:` addresses join through its tunnel, and the open world is
    // shared behind an invite code.
    &horizon_friends::game::HOOKS,
];

/// The lang files with Horizon's parts' text (their `assets/lang`).
const LANG: [&lang::Files; 1] = [horizon_friends::game::LANG];

/// The pages Horizon's parts add to the game's menus.
const PAGES: [&dyn Page; 2] = [
    // Play with Friends, what the pause menu's Friends icon opens.
    &horizon_friends::game::PAGE,
    // Join a Friend, what the title screen's Friends icon opens.
    &horizon_friends::game::JOIN,
];

/// Registers Horizon's in-game parts with the game. The executable calls it before it runs the
/// game, and so does the launcher's generated executable for native mods.
pub fn register() {
    for part in PARTS {
        if let Err(e) = hooks::register(part) {
            // The executable registered after starting the game: a bug, not a player's problem.
            log::warn!("Horizon: an in-game part did not register: {e}");
        }
    }
    for page in PAGES {
        if let Err(e) = pages::register(page) {
            log::warn!("Horizon: a page did not register: {e}");
        }
    }
    for files in LANG {
        if let Err(e) = lang::register(files) {
            log::warn!("Horizon: a part's text did not register: {e}");
        }
    }
}
