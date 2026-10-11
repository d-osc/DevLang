fn verify_package(args: &[String]) -> Result<i32, String> {
    if args.len() != 4 || args[2] != "--github" {
        return Err("pkg verify ARTIFACT --github OWNER/REPO".into());
    }
    scoped_identity(&format!("@{}", args[3]))?;
    if args[3].split('/').count() != 2 {
        return Err("attestation repository must be OWNER/REPO".into());
    }
    let artifact = dunce::canonicalize(&args[1]).map_err(|e| e.to_string())?;
    let status = Command::new("gh")
        .args(["attestation", "verify"])
        .arg(artifact)
        .args(["--repo", &args[3], "--deny-self-hosted-runners"])
        .status()
        .map_err(|e| format!("attestation verification requires gh: {e}"))?;
    Ok(status.code().unwrap_or(1))
}
fn verify_signed_tag(project: &Path, args: &[String]) -> Result<i32, String> {
    if args.len() != 4 || args[2] != "--key" || args[1].starts_with('-') {
        return Err("pkg verify-tag TAG --key FULL_FINGERPRINT".into());
    }
    let fingerprint = args[3].to_uppercase();
    if ![40, 64].contains(&fingerprint.len()) || !fingerprint.bytes().all(|b| b.is_ascii_hexdigit())
    {
        return Err("use a full signing/primary key fingerprint".into());
    }
    let output = Command::new("git")
        .current_dir(project)
        .args(["verify-tag", "--raw", &args[1]])
        .output()
        .map_err(|e| e.to_string())?;
    let valid = String::from_utf8_lossy(&output.stderr)
        .lines()
        .filter_map(|line| line.strip_prefix("[GNUPG:] VALIDSIG "))
        .any(|line| {
            line.split_whitespace()
                .any(|part| part.eq_ignore_ascii_case(&fingerprint))
        });
    if !output.status.success() || !valid {
        return Err("tag signature is invalid or does not match the pinned key".into());
    }
    println!("verified tag {} with key {fingerprint}", args[1]);
    Ok(0)
}
fn audit_packages(project: &Path, args: &[String]) -> Result<i32, String> {
    mappings(project)?;
    let lock: Lock = read(&lock_path(project))?;
    let mut found = 0;
    let mut uncovered = 0;
    if args.len() == 3 && args[1] == "--database" {
        let bytes = download_bytes(&args[2], 4 * 1024 * 1024)?;
        let database =
            dev_syntax::don::parse(std::str::from_utf8(&bytes).map_err(|e| e.to_string())?)?;
        if database["schema_version"] != 1 {
            return Err("unsupported advisory database".into());
        }
        let advisories = database["advisories"]
            .as_array()
            .ok_or("database needs advisories")?;
        let coverage = database["coverage"]
            .as_array()
            .ok_or("database needs explicit coverage")?;
        for (alias, item) in &lock.packages {
            let identity = item.source.package.as_deref().unwrap_or(alias);
            if !coverage.iter().any(|v| v.as_str() == Some(identity)) {
                uncovered += 1;
                println!("UNASSESSED {identity}");
                continue;
            }
            let m = manifest(&locked_root(project, alias, item)?)?;
            let Some(version) = m.version else {
                uncovered += 1;
                continue;
            };
            let version = semver::Version::parse(version.trim_start_matches('v'))
                .map_err(|e| e.to_string())?;
            for advisory in advisories {
                if advisory["package"].as_str() == Some(identity)
                    && advisory["withdrawn"] != true
                    && VersionRange::parse(
                        advisory["affected"]
                            .as_str()
                            .ok_or("advisory needs affected range")?,
                    )?
                    .matches(&version)
                {
                    found += 1;
                    println!(
                        "{} {identity} {version}",
                        advisory["id"].as_str().ok_or("advisory needs id")?
                    );
                }
            }
        }
    } else if args.len() == 1 {
        if offline_mode() {
            return Err(
                "offline audit requires --database file://... with locally saved advisories".into(),
            );
        }
        use std::io::Read;
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_global(Some(std::time::Duration::from_secs(30)))
            .build()
            .into();
        let mut queries = 0;
        for (alias, item) in &lock.packages {
            if !item
                .source
                .git
                .as_deref()
                .is_some_and(|s| s.starts_with("https://github.com/"))
            {
                uncovered += 1;
                println!("UNASSESSED {alias}: no public GitHub commit");
                continue;
            }
            let commit = item
                .commit
                .as_deref()
                .ok_or("Git dependency has no commit")?;
            let mut page = String::new();
            for count in 0..16 {
                queries += 1;
                if queries > 256 {
                    return Err("OSV audit exceeded its 256-request budget; use a local advisory feed for large graphs".into());
                }
                let mut query = serde_json::json!({"commit":commit});
                if !page.is_empty() {
                    query["page_token"] = page.clone().into();
                }
                let mut response = agent
                    .post("https://api.osv.dev/v1/query")
                    .header("Content-Type", "application/json")
                    .send(serde_json::to_vec(&query).map_err(|e| e.to_string())?)
                    .map_err(|e| e.to_string())?;
                let mut bytes = Vec::new();
                response
                    .body_mut()
                    .as_reader()
                    .take(4 * 1024 * 1024 + 1)
                    .read_to_end(&mut bytes)
                    .map_err(|e| e.to_string())?;
                if bytes.len() > 4 * 1024 * 1024 {
                    return Err("OSV response exceeds 4 MiB".into());
                }
                let result: serde_json::Value =
                    serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
                if let Some(vulns) = result["vulns"].as_array() {
                    for v in vulns {
                        if v["withdrawn"].is_null() {
                            found += 1;
                            println!(
                                "{} {alias} {commit}",
                                v["id"].as_str().ok_or("invalid OSV advisory")?
                            );
                        }
                    }
                }
                page = result["next_page_token"].as_str().unwrap_or("").into();
                if page.is_empty() {
                    break;
                }
                if count == 15 {
                    return Err("OSV audit exceeded pagination budget".into());
                }
            }
        }
    } else {
        return Err("pkg audit [--database URL] [--offline]".into());
    }
    println!("{found} known findings; {uncovered} unassessed dependencies. Database coverage is limited; zero findings is not a safety guarantee.");
    Ok(if found > 0 {
        1
    } else if uncovered > 0 {
        2
    } else {
        0
    })
}
