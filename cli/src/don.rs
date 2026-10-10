pub fn command(args: &[String]) -> Result<i32, String> {
    let [mode, file] = args else {
        return Err("usage: d don check|fmt|to-json|from-json FILE".into());
    };
    if !["check", "fmt", "to-json", "from-json"].contains(&mode.as_str()) {
        return Err("unknown DON command".into());
    }
    let file_path = std::path::Path::new(file);
    if std::fs::metadata(file_path)
        .map_err(|e| e.to_string())?
        .len()
        > 8 * 1024 * 1024
    {
        return Err("data input exceeds 8 MiB".into());
    }
    let text = std::fs::read_to_string(file_path).map_err(|e| format!("{file}: {e}"))?;
    let value = if mode == "from-json" {
        serde_json::from_str(&text).map_err(|e| format!("{file}: {e}"))?
    } else {
        dev_syntax::don::parse(&text).map_err(|e| format!("{file}: {e}"))?
    };
    match mode.as_str() {
        "check" => println!("valid DON: {file}"),
        "to-json" => println!(
            "{}",
            serde_json::to_string_pretty(&value).map_err(|e| e.to_string())?
        ),
        _ => println!("{}", dev_syntax::don::stringify(&value)?),
    }
    Ok(0)
}
