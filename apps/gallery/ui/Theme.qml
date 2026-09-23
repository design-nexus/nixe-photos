pragma Singleton

import Quickshell
import Quickshell.Io
import QtQuick
import "js/Palette.js" as Palette

Singleton {
    id: root

    readonly property string stateDir: Quickshell.env("HOME") + "/.local/state/omarchy/current"
    property string themeName: "Catppuccin"
    property bool ready: false

    // Strata-style semantic palette
    readonly property QtObject color: QtObject {
        property color background: "#1e1e2e"
        property color surface: "#242438"
        property color surfaceElevated: "#2a2a40"
        property color foreground: "#cdd6f4"
        property color dimText: "#7f849c"
        property color accent: "#89b4fa"
        property color selection: "#45475a"
        property color muted: "#313244"
        property color border: "#3b3e54"
        property color danger: "#f38ba8"
    }

    // Typography
    readonly property QtObject font: QtObject {
        readonly property string family: "sans-serif"
        readonly property int title: 16
        readonly property int body: 13
        readonly property int bodySmall: 11
        readonly property int caption: 10
    }

    // Geometry & Layout tokens
    readonly property QtObject geometry: QtObject {
        readonly property int radiusSmall: 4
        readonly property int radiusMedium: 8
        readonly property int radiusLarge: 12
        readonly property int pillRadius: 6
        readonly property int sidebarWidth: 220
        readonly property int inspectorWidth: 300
        readonly property int headerHeight: 46
        readonly property int statusBarHeight: 28
    }

    function applyColors(content) {
        if (!content || content.length === 0) return;
        var p = Palette.parse(content);
        if (!p || Object.keys(p).length === 0) return;

        root.color.background = Palette.pick(p, ["background"], "#1e1e2e");
        root.color.surface = Palette.pick(p, ["dark_background", "lighter_background", "surface"], "#242438");
        root.color.surfaceElevated = Palette.pick(p, ["lighter_background", "surface", "selection"], "#2a2a40");
        root.color.foreground = Palette.pick(p, ["foreground", "bright_foreground"], "#cdd6f4");
        root.color.dimText = Palette.pick(p, ["dark_foreground", "dim_text", "muted"], "#7f849c");
        root.color.accent = Palette.pick(p, ["accent", "blue"], "#89b4fa");
        root.color.selection = Palette.pick(p, ["selection", "highlight"], "#45475a");
        root.color.muted = Palette.pick(p, ["muted", "lighter_background"], "#313244");
        root.color.border = Palette.pick(p, ["border", "selection", "muted"], "#3b3e54");
        root.color.danger = Palette.pick(p, ["red", "danger"], "#f38ba8");
        root.ready = true;
    }

    function applyThemeName(name) {
        if (name && name.trim().length > 0) {
            root.themeName = name.trim();
        }
    }

    // Watch colors.toml
    FileView {
        id: colorsFile
        path: root.stateDir + "/theme/colors.toml"
        blockLoading: true
        printErrors: false
        onLoaded: root.applyColors(text())
        Component.onCompleted: root.applyColors(colorsFile.text())
    }

    // Watch theme.name which Omarchy updates in-place on theme switch
    FileView {
        id: themeNameFile
        path: root.stateDir + "/theme.name"
        blockLoading: true
        watchChanges: true
        printErrors: false
        onLoaded: root.applyThemeName(text())
        onFileChanged: {
            reload();
            colorsFile.reload();
            root.applyThemeName(text());
            root.applyColors(colorsFile.text());
        }
        Component.onCompleted: root.applyThemeName(themeNameFile.text())
    }
}
