//! Where two mods inject into the same place: the warnings a build prints (`warn_conflicts`).
use super::*;

/// The build's warnings about the mods' injections (#6490): two mods on one
/// place, mods wrapping one function at its head and its return, and a
/// cancelling injection on a native hook point. Neither stops the build —
/// they say what the player and the author could not otherwise see. The
/// places are the manifest's: whether two selectors pick the same call, or
/// a raw `target` names a hook point's function, only horizon-mixins
/// resolves, so the warnings stay to what the manifest says for sure.
pub(super) fn warn_conflicts(mods: &[Bundle]) -> Vec<String> {
    let mut out = Vec::new();
    // One place (the function, the kind, the selector, the ordinal), the
    // injections of at least two mods on it, ordered as horizon-mixins runs
    // same-place injections: priority descending, mod id ascending,
    // injection id ascending — the tail of its own sort, which the place
    // prefixes already fixed.
    let mut places: BTreeMap<_, Vec<(&str, &Injection)>> = BTreeMap::new();
    for b in mods {
        for i in &b.injections {
            places.entry(i.place()).or_default().push((&b.id, i));
        }
    }
    for (_, list) in places {
        let mut list = list;
        list.sort_by_key(|(mod_id, i)| {
            (
                std::cmp::Reverse(i.priority),
                (*mod_id).to_owned(),
                i.id.clone(),
            )
        });
        let mut names: Vec<&str> = Vec::new();
        for (mod_id, _) in &list {
            if !names.contains(mod_id) {
                names.push(mod_id);
            }
        }
        if names.len() < 2 {
            continue;
        }
        let first = list[0].1;
        let both = if names.len() == 2 { "both" } else { "all" };
        out.push(format!(
            "mods {} {both} inject at {}; {} runs first (priority descending, mod id ascending).",
            names.join(", "),
            first.place_as_text(),
            names[0]
        ));
    }
    // A function's head and its return: two mods wrapping the whole
    // function, which run in that order whatever their priorities. Said
    // without a "runs first".
    let mut functions: BTreeMap<&InjectionSite, Vec<(&str, &Injection)>> = BTreeMap::new();
    for b in mods {
        for i in &b.injections {
            if i.kind.as_deref() == Some("HEAD") || i.kind.as_deref() == Some("RETURN") {
                functions.entry(&i.site).or_default().push((&b.id, i));
            }
        }
    }
    for (site, list) in functions {
        let kinds: std::collections::BTreeSet<&str> =
            list.iter().filter_map(|(_, i)| i.kind.as_deref()).collect();
        let mut names: Vec<&str> = Vec::new();
        for (mod_id, _) in &list {
            if !names.contains(mod_id) {
                names.push(mod_id);
            }
        }
        // Both sides, and more than one mod on the function: a single mod
        // wrapping its own function at both ends is no one's conflict.
        if kinds.len() < 2 || names.len() < 2 {
            continue;
        }
        let sides: Vec<String> = list
            .iter()
            .map(|(mod_id, i)| format!("{mod_id} ({})", i.kind.as_deref().unwrap_or("")))
            .collect();
        out.push(format!(
            "mods {} inject into {}: HEAD runs on entry, RETURN on return, whatever their priorities.",
            sides.join(", "),
            site.as_text()
        ));
    }
    // A cancelling `HEAD` on a hook point: a `Break` skips the later
    // injections at the place and the function's body. A raw `target` is
    // only known to be a hook point through the catalog, which the launcher
    // does not read; a `hook` name is one by definition. Only `HEAD` is
    // said: the interior kinds' cancellation skips less than the body.
    for b in mods {
        for i in &b.injections {
            if i.cancellable
                && i.kind.as_deref() == Some("HEAD")
                && matches!(i.site, InjectionSite::Hook(_))
            {
                out.push(format!(
                    "{}'s `{}` can cancel at {}: a Break skips the later injections and the function's body.",
                    b.id,
                    i.id,
                    i.place_as_text()
                ));
            }
        }
    }
    out
}
