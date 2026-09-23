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

    // Real photos array
    property var allPhotos: []
    property var displayedPhotos: []

    // Counts
    property int totalItems: 0
    property int favoriteCount: 0
    property int screenshotCount: 0
    property int photoCount: 0
    property string totalBytesFormatted: "0 B"
    property string libraryPath: "~/Pictures"

    // Selected photo info for inspector
    property var selectedItem: null
    property int selectedIndex: -1

    function updatePhotos(photos, total, bytesFormatted, path) {
        root.allPhotos = photos || [];
        root.totalItems = total !== undefined ? total : root.allPhotos.length;
        if (bytesFormatted) root.totalBytesFormatted = bytesFormatted;
        if (path) root.libraryPath = path;

        // Recalculate counts
        var favs = 0;
        var screens = 0;
        var regPhotos = 0;
        for (var i = 0; i < root.allPhotos.length; i++) {
            var p = root.allPhotos[i];
            if (p.is_favorite) favs++;
            if (p.media_type === "screenshots") screens++;
            else regPhotos++;
        }
        root.favoriteCount = favs;
        root.screenshotCount = screens;
        root.photoCount = regPhotos;

        filterSection();

        if (!root.selectedItem && root.displayedPhotos.length > 0) {
            root.selectPhoto(root.displayedPhotos[0], 0);
        }
    }

    function setSection(id, title) {
        root.currentSection = id;
        root.currentSectionTitle = title;
        filterSection();
    }

    function filterSection() {
        var res = [];
        for (var i = 0; i < root.allPhotos.length; i++) {
            var item = root.allPhotos[i];
            if (root.currentSection === "favorites") {
                if (item.is_favorite) res.push(item);
            } else if (root.currentSection === "screenshots") {
                if (item.media_type === "screenshots") res.push(item);
            } else if (root.currentSection === "photos") {
                if (item.media_type === "photos") res.push(item);
            } else {
                res.push(item);
            }
        }
        root.displayedPhotos = res;

        // If selection is not in displayed, reset or select first
        if (root.displayedPhotos.length > 0) {
            var found = false;
            if (root.selectedItem) {
                for (var j = 0; j < root.displayedPhotos.length; j++) {
                    if (root.displayedPhotos[j].id === root.selectedItem.id) {
                        found = true;
                        root.selectedIndex = j;
                        break;
                    }
                }
            }
            if (!found) {
                root.selectPhoto(root.displayedPhotos[0], 0);
            }
        } else {
            root.selectedItem = null;
            root.selectedIndex = -1;
        }
    }

    function selectPhoto(photo, index) {
        root.selectedItem = photo;
        root.selectedIndex = index !== undefined ? index : -1;
    }

    function toggleInspector() {
        root.inspectorOpen = !root.inspectorOpen;
    }

    function toggleSidebar() {
        root.sidebarOpen = !root.sidebarOpen;
    }
}
