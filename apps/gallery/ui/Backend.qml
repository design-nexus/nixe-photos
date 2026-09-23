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

        if (msg.t === "status") {
            ViewState.libraryPath = msg.library_path || "~/Pictures";
            ViewState.totalItems = msg.total_items || 0;
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
