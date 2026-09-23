pragma Singleton

import Quickshell
import QtQuick

Singleton {
    id: root

    property string currentSection: "all_photos"
    property string currentSectionTitle: "All Photos"
    property string viewMode: "all" // "years", "months", "days", "all"
    property real zoomLevel: 0.4    // 0.0 (compact) to 1.0 (large)
    property bool inspectorOpen: true
    property bool sidebarOpen: true
    property string searchQuery: ""
    property bool searchActive: false

    // Selected photo info for inspector
    property var selectedItem: null
    property int selectedIndex: -1
    property int totalItems: 0
    property string libraryPath: "~/Pictures"

    function setSection(id, title) {
        root.currentSection = id;
        root.currentSectionTitle = title;
    }

    function toggleInspector() {
        root.inspectorOpen = !root.inspectorOpen;
    }

    function toggleSidebar() {
        root.sidebarOpen = !root.sidebarOpen;
    }
}
