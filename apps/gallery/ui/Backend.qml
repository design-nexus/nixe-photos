pragma Singleton

import Quickshell
import Quickshell.Io
import QtQuick

Singleton {
    id: root

    property bool running: child.running
    property var pending: []
    property bool queueing: true

    signal statusReceived(string libraryPath, string themeName, int totalItems)
    signal revealResult(bool ok, string message)
    signal photosUpdated()

    function send(obj) {
        var line = JSON.stringify(obj) + "\n";
        if (root.queueing) {
            root.pending.push(line);
            return;
        }
        if (child.running) {
            child.write(line);
        }
    }

    function ping() {
        root.send({ c: "ping" });
    }

    function getStatus() {
        root.send({ c: "get_status" });
    }

    function scanLibrary(path) {
        root.send({ c: "scan_library", path: path || null });
    }

    function toggleFavorite(id) {
        root.send({ c: "toggle_favorite", id: id });
    }

    function createAlbum(name) {
        if (!name || name.trim().length === 0) return;
        root.send({ c: "create_album", name: name.trim() });
    }

    function addPhotosToAlbum(albumId, photoIds) {
        root.send({ c: "add_photos_to_album", album_id: albumId, photo_ids: photoIds });
    }

    function deleteAlbum(albumId) {
        root.send({ c: "delete_album", album_id: albumId });
    }

    function revealInStrata(path) {
        root.send({ c: "reveal_in_strata", path: path });
    }

    function receive(raw) {
        if (!raw || raw.trim().length === 0) return;
        var msg = null;
        try {
            msg = JSON.parse(raw.trim());
        } catch (e) {
            console.warn("Backend parse error:", e, raw);
            return;
        }

        if (msg.t === "library_scanned") {
            ViewState.updatePhotos(msg.photos, msg.total_items, msg.total_bytes_formatted, msg.library_path);
            if (msg.albums) {
                ViewState.updateAlbums(msg.albums);
            }
            root.photosUpdated();
        } else if (msg.t === "photos") {
            ViewState.updatePhotos(msg.photos, msg.photos.length);
            root.photosUpdated();
        } else if (msg.t === "albums") {
            ViewState.updateAlbums(msg.albums);
        } else if (msg.t === "album_created") {
            var currAlbums = ViewState.albums.slice();
            currAlbums.push(msg.album);
            ViewState.updateAlbums(currAlbums);
            ViewState.showToast("Album '" + msg.album.name + "' created");
        } else if (msg.t === "favorite_toggled") {
            var photos = ViewState.allPhotos;
            for (var i = 0; i < photos.length; i++) {
                if (photos[i].id === msg.id) {
                    photos[i].is_favorite = msg.is_favorite;
                    break;
                }
            }
            ViewState.updatePhotos(photos);
            root.photosUpdated();
        } else if (msg.t === "status") {
            ViewState.libraryPath = msg.library_path || "~/Pictures";
            ViewState.totalItems = msg.total_items || 0;
            if (msg.total_bytes_formatted) ViewState.totalBytesFormatted = msg.total_bytes_formatted;
            root.statusReceived(msg.library_path, msg.theme_name, msg.total_items);
        } else if (msg.t === "reveal_result") {
            root.revealResult(msg.ok, msg.message);
        }
    }

    Process {
        id: child
        command: [Quickshell.env("NEXUS_BIN") || "nexus-photos", "--backend"]
        running: true
        stdinEnabled: true

        stdout: SplitParser {
            splitMarker: "\n"
            onRead: function(data) { root.receive(data); }
        }

        onStarted: {
            root.queueing = false;
            for (var i = 0; i < root.pending.length; i++) {
                child.write(root.pending[i]);
            }
            root.pending = [];
            root.getStatus();
        }

        onRunningChanged: {
            if (root.queueing && !child.running) {
                root.queueing = false;
                root.pending = [];
                console.warn("Backend failed to start");
            }
        }
    }
}
