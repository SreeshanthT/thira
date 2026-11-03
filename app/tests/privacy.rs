//! Thira makes no network calls (PRD privacy). The webview is the only part
//! that could reach the network on its own, so its content security policy
//! must allow no remote sources, and no window may hold a network plugin's
//! permissions. Network crates are banned separately in deny.toml.

use std::fs;
use std::path::Path;

use serde_json::Value;

fn read_json(path: &Path) -> Value {
    let text = fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// Sources allowed in any CSP directive: nothing that names a remote origin.
const LOCAL_SOURCES: [&str; 4] = ["'self'", "'none'", "'unsafe-inline'", "data:"];

#[test]
fn csp_allows_no_remote_sources() {
    let config = read_json(&Path::new(env!("CARGO_MANIFEST_DIR")).join("tauri.conf.json"));
    let csp = config["app"]["security"]["csp"]
        .as_str()
        .expect("app.security.csp must be set");

    let directives: Vec<Vec<&str>> = csp
        .split(';')
        .map(|d| d.split_whitespace().collect::<Vec<_>>())
        .filter(|d| !d.is_empty())
        .collect();
    // Without default-src, fetch/connect would be unrestricted.
    assert!(
        directives.iter().any(|d| d == &["default-src", "'self'"]),
        "CSP needs `default-src 'self'`: {csp}"
    );
    for directive in &directives {
        for source in &directive[1..] {
            assert!(
                LOCAL_SOURCES.contains(source),
                "CSP directive `{}` allows `{source}`, which may reach the network: {csp}",
                directive[0]
            );
        }
    }
}

/// Permission prefixes of Tauri plugins that make network requests.
const NETWORK_PLUGINS: [&str; 4] = ["http:", "updater:", "websocket:", "upload:"];

#[test]
fn no_window_has_network_permissions() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("capabilities");
    let mut checked = 0;
    for entry in fs::read_dir(&dir).expect("capabilities directory") {
        let path = entry.expect("entry").path();
        if path.extension().is_none_or(|ext| ext != "json") {
            continue;
        }
        let capability = read_json(&path);
        let permissions = capability["permissions"]
            .as_array()
            .unwrap_or_else(|| panic!("{}: no permissions list", path.display()));
        for permission in permissions {
            // Either "plugin:permission" or { "identifier": ..., ... }.
            let id = permission
                .as_str()
                .or_else(|| permission["identifier"].as_str())
                .unwrap_or_default();
            assert!(
                !NETWORK_PLUGINS.iter().any(|p| id.starts_with(p)),
                "{} grants network permission `{id}`",
                path.display()
            );
        }
        checked += 1;
    }
    assert!(checked > 0, "no capability files in {}", dir.display());
}
