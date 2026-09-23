pragma Singleton

import Quickshell
import QtQuick

Singleton {
    id: root

    property string currentSection: "all_photos"
    property string currentSectionTitle: "All Photos"
    property string viewMode: "days" // "years", "months", "days", "all"
    property real zoomLevel: 0.4     // 0.0 (compact) to 1.0 (large)
    property bool inspectorOpen: true
    property bool sidebarOpen: true
    property string searchQuery: ""
    property bool searchActive: false

    // Real photos array
    property var allPhotos: []
    property var displayedPhotos: []
    property var timelineGroups: []

    // Multi-selection state
    property var selectedIds: []
    readonly property int selectedCount: selectedIds.length

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

    onViewModeChanged: buildTimelineGroups()

    function updatePhotos(photos, total, bytesFormatted, path) {
        root.allPhotos = photos || [];
        root.totalItems = total !== undefined ? total : root.allPhotos.length;
        if (bytesFormatted) root.totalBytesFormatted = bytesFormatted;
        if (path) root.libraryPath = path;

        // Recalculate category counts
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

        if (root.displayedPhotos.length > 0 && (!root.selectedItem || root.selectedIds.length === 0)) {
            root.selectSingle(root.displayedPhotos[0], 0);
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
        buildTimelineGroups();

        // Restore or reset selection
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
                root.selectSingle(root.displayedPhotos[0], 0);
            }
        } else {
            root.selectedItem = null;
            root.selectedIndex = -1;
            root.selectedIds = [];
        }
    }

    function buildTimelineGroups() {
        if (!root.displayedPhotos || root.displayedPhotos.length === 0) {
            root.timelineGroups = [];
            return;
        }

        if (root.viewMode === "all") {
            root.timelineGroups = [{
                id: "all",
                header: root.currentSectionTitle,
                subheader: root.displayedPhotos.length + " Photos",
                items: root.displayedPhotos
            }];
            return;
        }

        var groupsMap = {};
        var groupOrder = [];

        for (var i = 0; i < root.displayedPhotos.length; i++) {
            var p = root.displayedPhotos[i];
            var key = "";
            var title = "";

            if (root.viewMode === "years") {
                key = String(p.year || "2026");
                title = key;
            } else if (root.viewMode === "months") {
                var mName = fullMonthName(p.month);
                key = mName + " " + (p.year || "2026");
                title = key;
            } else {
                // "days"
                key = p.date_display || (p.year + "-" + p.month + "-" + p.day);
                title = key;
            }

            if (!groupsMap[key]) {
                groupsMap[key] = {
                    id: key,
                    header: title,
                    items: []
                };
                groupOrder.push(key);
            }
            groupsMap[key].items.push(p);
        }

        var res = [];
        for (var k = 0; k < groupOrder.length; k++) {
            var g = groupsMap[groupOrder[k]];
            g.subheader = g.items.length + (g.items.length === 1 ? " photo" : " photos");
            res.push(g);
        }
        root.timelineGroups = res;
    }

    function fullMonthName(m) {
        var names = [
            "", "January", "February", "March", "April", "May", "June",
            "July", "August", "September", "October", "November", "December"
        ];
        return names[m] || "September";
    }

    // Selection management
    function isSelected(id) {
        return root.selectedIds.indexOf(id) !== -1;
    }

    function selectSingle(photo, index) {
        root.selectedItem = photo;
        root.selectedIndex = index !== undefined ? index : findIndexById(photo.id);
        root.selectedIds = [photo.id];
    }

    function toggleSelect(photo, index) {
        var idx = root.selectedIds.indexOf(photo.id);
        var copy = root.selectedIds.slice();
        if (idx === -1) {
            copy.push(photo.id);
            root.selectedItem = photo;
            root.selectedIndex = index !== undefined ? index : findIndexById(photo.id);
        } else {
            copy.splice(idx, 1);
            if (root.selectedItem && root.selectedItem.id === photo.id) {
                root.selectedItem = copy.length > 0 ? findPhotoById(copy[copy.length - 1]) : null;
                root.selectedIndex = root.selectedItem ? findIndexById(root.selectedItem.id) : -1;
            }
        }
        root.selectedIds = copy;
    }

    function selectRange(targetIndex) {
        if (targetIndex < 0 || targetIndex >= root.displayedPhotos.length) return;
        var start = root.selectedIndex >= 0 ? root.selectedIndex : 0;
        var min = Math.min(start, targetIndex);
        var max = Math.max(start, targetIndex);

        var copy = root.selectedIds.slice();
        for (var i = min; i <= max; i++) {
            var id = root.displayedPhotos[i].id;
            if (copy.indexOf(id) === -1) {
                copy.push(id);
            }
        }
        root.selectedIds = copy;
        root.selectedItem = root.displayedPhotos[targetIndex];
        root.selectedIndex = targetIndex;
    }

    function selectAll() {
        var allIds = [];
        for (var i = 0; i < root.displayedPhotos.length; i++) {
            allIds.push(root.displayedPhotos[i].id);
        }
        root.selectedIds = allIds;
    }

    function clearSelection() {
        if (root.displayedPhotos.length > 0) {
            root.selectSingle(root.displayedPhotos[0], 0);
        } else {
            root.selectedIds = [];
            root.selectedItem = null;
            root.selectedIndex = -1;
        }
    }

    function navigateGrid(delta) {
        if (root.displayedPhotos.length === 0) return;
        var nextIdx = root.selectedIndex + delta;
        if (nextIdx < 0) nextIdx = 0;
        if (nextIdx >= root.displayedPhotos.length) nextIdx = root.displayedPhotos.length - 1;
        root.selectSingle(root.displayedPhotos[nextIdx], nextIdx);
    }

    function findIndexById(id) {
        for (var i = 0; i < root.displayedPhotos.length; i++) {
            if (root.displayedPhotos[i].id === id) return i;
        }
        return -1;
    }

    function findPhotoById(id) {
        for (var i = 0; i < root.displayedPhotos.length; i++) {
            if (root.displayedPhotos[i].id === id) return root.displayedPhotos[i];
        }
        return null;
    }

    // Lightbox & Rotation State
    property bool lightboxOpen: false
    property var rotationMap: ({})
    property string toastMessage: ""
    property bool toastVisible: false

    function showToast(msg) {
        root.toastMessage = msg;
        root.toastVisible = true;
    }

    function hideToast() {
        root.toastVisible = false;
    }

    function openLightbox(photo, index) {
        if (photo) {
            root.selectSingle(photo, index);
        }
        root.lightboxOpen = true;
    }

    function closeLightbox() {
        root.lightboxOpen = false;
    }

    function nextPhoto() {
        if (root.displayedPhotos.length === 0) return;
        var next = (root.selectedIndex + 1) % root.displayedPhotos.length;
        root.selectSingle(root.displayedPhotos[next], next);
    }

    function prevPhoto() {
        if (root.displayedPhotos.length === 0) return;
        var prev = (root.selectedIndex - 1 + root.displayedPhotos.length) % root.displayedPhotos.length;
        root.selectSingle(root.displayedPhotos[prev], prev);
    }

    function rotateCurrent() {
        if (!root.selectedItem) return;
        var current = getRotation(root.selectedItem.id);
        var nextAngle = (current + 90) % 360;
        var mapCopy = Object.assign({}, root.rotationMap);
        mapCopy[root.selectedItem.id] = nextAngle;
        root.rotationMap = mapCopy;
    }

    function getRotation(id) {
        if (!id) return 0;
        return root.rotationMap[id] || 0;
    }

    function copyCurrentPath() {
        if (!root.selectedItem) return;
        var p = root.selectedItem.path;
        Quickshell.execDetached(["wl-copy", p]);
        showToast("Copied path to clipboard");
    }

    function toggleSelectedFavorite() {
        if (!root.selectedItem) return;
        Backend.toggleFavorite(root.selectedItem.id);
    }

    function toggleInspector() {
        root.inspectorOpen = !root.inspectorOpen;
    }

    function toggleSidebar() {
        root.sidebarOpen = !root.sidebarOpen;
    }
}
