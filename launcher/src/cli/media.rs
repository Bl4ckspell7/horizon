//! `screenshots` and `multiplayer`: the commands over an instance's screenshots and server list.
use super::*;

pub(super) fn screenshots(action: ScreenshotsCmd, as_json: bool) -> Result<(), String> {
    use horizon_launcher::screenshots;
    let home = |instance: Option<String>| -> Result<PathBuf, String> {
        Ok(match instance {
            Some(name) => instances::find(&name)?.home,
            None => instances::selected().home,
        })
    };
    match action {
        ScreenshotsCmd::List { instance } => {
            let home = home(instance)?;
            let shots = screenshots::list(&home);
            if as_json {
                json(
                    shots
                        .iter()
                        .map(|s| {
                            let millis = s
                                .taken
                                .duration_since(std::time::UNIX_EPOCH)
                                .map_or(0, |d| d.as_millis() as u64);
                            serde_json::json!({ "name": s.name, "path": s.path, "taken_ms": millis, "size": s.size })
                        })
                        .collect(),
                );
            } else if shots.is_empty() {
                println!("No screenshots in {}", screenshots::folder(&home).display());
            }
            for s in shots.iter().filter(|_| !as_json) {
                println!("{}", s.path.display());
            }
        }
        ScreenshotsCmd::Delete {
            screenshot,
            instance,
        } => {
            let shot = screenshots::find(&home(instance)?, &screenshot)?;
            screenshots::delete(&shot)?;
            println!("{} is in the trash.", shot.name);
        }
    }
    Ok(())
}

pub(super) fn multiplayer(action: MultiplayerCmd, as_json: bool) -> Result<(), String> {
    let home = |instance: Option<String>| -> Result<PathBuf, String> {
        Ok(match instance {
            Some(name) => instances::find(&name)?.home,
            None => instances::selected().home,
        })
    };
    match action {
        MultiplayerCmd::List { instance } => {
            let home = home(instance)?;
            let servers = server_list::List::load(&home).servers();
            if as_json {
                json(
                    servers
                        .iter()
                        .map(|s| serde_json::json!({ "name": s.name, "address": s.ip, "icon": s.icon.is_some() }))
                        .collect(),
                );
            } else if servers.is_empty() {
                println!("No servers in {}", home.join("servers.dat").display());
            }
            for (i, s) in servers.iter().enumerate().filter(|_| !as_json) {
                println!("{}. {} ({})", i + 1, s.name, s.ip);
            }
        }
        MultiplayerCmd::Add {
            address,
            name,
            instance,
        } => {
            let home = home(instance)?;
            let mut list = server_list::List::load(&home);
            list.add(name.as_deref().unwrap_or(""), &address)?;
            list.save(&home)?;
            println!("Added {}.", address.trim());
        }
        MultiplayerCmd::Remove { server, instance } => {
            let home = home(instance)?;
            let mut list = server_list::List::load(&home);
            let (i, _) = server_list::find(&list, &server)?;
            let gone = list.remove(i)?;
            list.save(&home)?;
            println!("Removed {} ({}).", gone.name, gone.ip);
        }
    }
    Ok(())
}
