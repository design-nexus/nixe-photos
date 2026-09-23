// Palette.js — Omarchy theme colors.toml parser

function parse(body) {
    var found = {};
    var lines = String(body || "").split("\n");
    for (var i = 0; i < lines.length; i++) {
        var line = lines[i].trim();
        if (!line || line.indexOf("#") === 0) continue;
        var kv = line.match(/^\s*([A-Za-z0-9_-]+)\s*=\s*["']?(#[0-9A-Fa-f]{6})/);
        if (kv) {
            found[kv[1]] = kv[2];
        }
    }
    return found;
}

function pick(found, keys, fallback) {
    for (var i = 0; i < keys.length; i++) {
        if (found && found[keys[i]]) {
            return found[keys[i]];
        }
    }
    return fallback;
}
