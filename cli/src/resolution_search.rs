// A bounded chronological search. Selected versions do not replace declared ranges in locks.
#[derive(Default)]
struct ResolutionSearch {
    active: bool,
    key: String,
    choices: Vec<(String, Vec<semver::Version>, usize)>,
    visited: BTreeSet<String>,
}
thread_local! { static RESOLUTION_SEARCH: std::cell::RefCell<ResolutionSearch> = std::cell::RefCell::new(ResolutionSearch::default()); }
fn choose_version(mut versions: Vec<semver::Version>) -> Result<semver::Version, String> {
    versions.sort_by(|a, b| b.cmp_precedence(a));
    versions.dedup();
    RESOLUTION_SEARCH.with(|state| {
        let mut state = state.borrow_mut();
        if !state.active {
            return versions
                .first()
                .cloned()
                .ok_or("no matching version".into());
        }
        let key = state.key.clone();
        state.visited.insert(key.clone());
        if let Some((_, old, index)) = state.choices.iter().find(|(k, _, _)| k == &key) {
            let selected = old.get(*index).ok_or("invalid resolver choice")?;
            if versions.contains(selected) {
                return Ok(selected.clone());
            }
            return Err("selected version no longer satisfies dependency".into());
        }
        let selected = versions.first().cloned().ok_or("no matching version")?;
        state.choices.push((key, versions, 0));
        Ok(selected)
    })
}
fn install_mode(
    project: &Path,
    locked: bool,
    refresh: bool,
    production: bool,
) -> Result<(), String> {
    if locked || offline_mode() {
        return install_attempt(project, locked, refresh, production);
    }
    let budget = match std::env::var("DEVLANG_RESOLUTION_ATTEMPTS") {
        Ok(value) => value
            .parse::<usize>()
            .ok()
            .filter(|n| (1..=65536).contains(n))
            .ok_or("DEVLANG_RESOLUTION_ATTEMPTS must be an integer from 1 to 65536")?,
        Err(std::env::VarError::NotPresent) => 128,
        Err(_) => return Err("DEVLANG_RESOLUTION_ATTEMPTS must be valid text".into()),
    };
    RESOLUTION_SEARCH.with(|s| {
        *s.borrow_mut() = ResolutionSearch {
            active: true,
            ..Default::default()
        }
    });
    let original = std::fs::read(manifest_path(project)).map_err(|e| e.to_string())?;
    let result = (|| {
        for attempt in 0..budget {
            RESOLUTION_SEARCH.with(|s| s.borrow_mut().visited.clear());
            match install_attempt(project, false, refresh || attempt > 0, production) {
                Ok(()) => return Ok(()),
                Err(error) => {
                    if try_auto_peer(project, &error)? {
                        continue;
                    }
                    // Transport, integrity and malformed source failures must not be hidden by a downgrade.
                    if ![
                        "does not satisfy",
                        "requires peer",
                        "requires DevLang CLI",
                        "no Git SemVer tag",
                        "no Store version",
                        "no matching version",
                        "selected version no longer",
                    ]
                    .iter()
                    .any(|message| error.contains(message))
                    {
                        return Err(error);
                    }
                    // A previously cached selection has no search choices yet. Refresh once to enumerate alternatives.
                    if attempt == 0
                        && lock_path(project).exists()
                        && RESOLUTION_SEARCH.with(|s| s.borrow().choices.is_empty())
                    {
                        continue;
                    }
                    let more = RESOLUTION_SEARCH.with(|s| {
                        let mut s = s.borrow_mut();
                        // Choices from a discarded dependency branch cannot constrain
                        // its replacement or consume the next branch's search budget.
                        let visited = s.visited.clone();
                        s.choices.retain(|(key, _, _)| visited.contains(key));
                        while let Some((_, versions, index)) = s.choices.last_mut() {
                            if *index + 1 < versions.len() {
                                *index += 1;
                                return true;
                            }
                            s.choices.pop();
                        }
                        false
                    });
                    if !more {
                        return Err(error);
                    }
                }
            }
        }
        Err(format!("dependency resolution exhausted its {budget}-attempt budget; adjust DEVLANG_RESOLUTION_ATTEMPTS"))
    })();
    RESOLUTION_SEARCH.with(|s| *s.borrow_mut() = ResolutionSearch::default());
    if result.is_err() {
        std::fs::write(manifest_path(project), original).map_err(|e| e.to_string())?;
    }
    result
}
