//! What voyager-sky asks Hyprland through hyprctl.

fn hyprctl(args: &[&str]) -> Option<serde_json::Value> {
    let out = std::process::Command::new("hyprctl")
        .args(args)
        .output()
        .ok()?;
    serde_json::from_slice(&out.stdout).ok()
}

/// The cursor, in global logical pixels.
pub fn cursor_pos() -> Option<(f64, f64)> {
    let v = hyprctl(&["-j", "cursorpos"])?;
    Some((v["x"].as_f64()?, v["y"].as_f64()?))
}

/// The biggest connected monitor, in pixels, rotation included.
pub fn largest_output() -> Option<(u32, u32)> {
    let mons = hyprctl(&["-j", "monitors"])?;
    mons.as_array()?
        .iter()
        .filter_map(|m| {
            let (w, h) = (m["width"].as_u64()? as u32, m["height"].as_u64()? as u32);
            Some(if m["transform"].as_u64().unwrap_or(0) % 2 == 1 {
                (h, w)
            } else {
                (w, h)
            })
        })
        .max_by_key(|&(w, h)| w as u64 * h as u64)
}
