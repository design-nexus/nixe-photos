import QtQuick
import QtQuick.Layouts
import QtQuick.Controls
import "."

Rectangle {
    id: root
    visible: ViewState.searchActive
    color: Qt.rgba(0, 0, 0, 0.6)
    z: 250

    property var searchResults: []
    property int selectedIndex: 0

    onVisibleChanged: {
        if (visible) {
            searchInput.text = "";
            root.selectedIndex = 0;
            updateSearch("");
            searchInput.forceActiveFocus();
        }
    }

    function updateSearch(q) {
        var query = (q || "").trim().toLowerCase();
        if (query.length === 0) {
            root.searchResults = ViewState.allPhotos.slice(0, 15);
            root.selectedIndex = 0;
            return;
        }

        var matches = [];
        for (var i = 0; i < ViewState.allPhotos.length; i++) {
            var p = ViewState.allPhotos[i];
            var fn = (p.file_name || "").toLowerCase();
            var cam = (p.camera_model || "").toLowerCase();
            var lens = (p.lens_model || "").toLowerCase();
            var date = (p.date_display || "").toLowerCase();
            var fmt = (p.format || "").toLowerCase();

            if (fn.indexOf(query) !== -1 ||
                cam.indexOf(query) !== -1 ||
                lens.indexOf(query) !== -1 ||
                date.indexOf(query) !== -1 ||
                fmt.indexOf(query) !== -1) {
                matches.push(p);
            }
        }
        root.searchResults = matches;
        root.selectedIndex = 0;
    }

    function selectCurrent() {
        if (root.searchResults.length > 0 && root.selectedIndex >= 0 && root.selectedIndex < root.searchResults.length) {
            var item = root.searchResults[root.selectedIndex];
            var gIdx = ViewState.findIndexById(item.id);
            ViewState.selectSingle(item, gIdx);
            ViewState.searchActive = false;
        }
    }

    // Modal Background click to close
    MouseArea {
        anchors.fill: parent
        onClicked: ViewState.searchActive = false
    }

    // Floating Search Dialog Card
    Rectangle {
        width: Math.min(560, parent.width - 40)
        height: Math.min(460, parent.height - 80)
        anchors.centerIn: parent
        radius: Theme.geometry.radiusLarge
        color: Theme.color.surface
        border.color: Theme.color.border
        border.width: 1
        clip: true

        // Stop clicks inside card from closing modal
        MouseArea {
            anchors.fill: parent
        }

        ColumnLayout {
            anchors.fill: parent
            spacing: 0

            // Search Header Bar
            Rectangle {
                Layout.fillWidth: true
                height: 52
                color: Theme.color.surfaceElevated

                RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: 16
                    anchors.rightMargin: 16
                    spacing: 12

                    NexusIcon {
                        name: "search"
                        size: 14
                        color: Theme.color.dimText
                    }

                    TextField {
                        id: searchInput
                        Layout.fillWidth: true
                        placeholderText: "Search photos by name, camera, date, format..."
                        placeholderTextColor: Theme.color.dimText
                        color: Theme.color.foreground
                        font.family: Theme.font.family
                        font.pixelSize: Theme.font.body
                        background: Rectangle { color: "transparent" }

                        onTextChanged: root.updateSearch(text)

                        Keys.onPressed: function(event) {
                            if (event.key === Qt.Key_Escape) {
                                ViewState.searchActive = false;
                                event.accepted = true;
                            } else if (event.key === Qt.Key_Down) {
                                if (root.selectedIndex < root.searchResults.length - 1) {
                                    root.selectedIndex++;
                                }
                                event.accepted = true;
                            } else if (event.key === Qt.Key_Up) {
                                if (root.selectedIndex > 0) {
                                    root.selectedIndex--;
                                }
                                event.accepted = true;
                            } else if (event.key === Qt.Key_Return || event.key === Qt.Key_Enter) {
                                root.selectCurrent();
                                event.accepted = true;
                            }
                        }
                    }

                    Rectangle {
                        width: 38
                        height: 20
                        radius: 4
                        color: Theme.color.surface
                        border.color: Theme.color.border
                        border.width: 1

                        Text {
                            anchors.centerIn: parent
                            text: "Esc"
                            font.family: "monospace"
                            font.pixelSize: 10
                            color: Theme.color.dimText
                        }

                        TapHandler { onTapped: ViewState.searchActive = false }
                    }
                }

                Rectangle {
                    anchors.bottom: parent.bottom
                    anchors.left: parent.left
                    anchors.right: parent.right
                    height: 1
                    color: Theme.color.border
                }
            }

            // Results List
            ScrollView {
                Layout.fillWidth: true
                Layout.fillHeight: true
                clip: true

                ListView {
                    id: resultsList
                    width: parent.width
                    model: root.searchResults
                    currentIndex: root.selectedIndex

                    delegate: Rectangle {
                        property bool isHighlight: root.selectedIndex === index
                        width: resultsList.width
                        height: 48
                        color: isHighlight ? Theme.color.selection : (itemHover.hovered ? Theme.color.surfaceElevated : "transparent")

                        RowLayout {
                            anchors.fill: parent
                            anchors.leftMargin: 14
                            anchors.rightMargin: 14
                            spacing: 12

                            // Thumbnail Mini
                            Rectangle {
                                width: 34
                                height: 34
                                radius: Theme.geometry.radiusSmall
                                color: Theme.color.background
                                clip: true

                                Image {
                                    anchors.fill: parent
                                    source: "file://" + (modelData.thumbnail_path || modelData.path)
                                    fillMode: Image.PreserveAspectCrop
                                }
                            }

                            // Details
                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: 2

                                Text {
                                    text: modelData.file_name
                                    font.family: Theme.font.family
                                    font.pixelSize: Theme.font.bodySmall
                                    font.bold: isHighlight
                                    color: isHighlight ? Theme.color.accent : Theme.color.foreground
                                    elide: Text.ElideMiddle
                                    Layout.fillWidth: true
                                }

                                Text {
                                    text: modelData.date_display + (modelData.camera_model ? (" • " + modelData.camera_model) : "")
                                    font.family: Theme.font.family
                                    font.pixelSize: Theme.font.caption
                                    color: Theme.color.dimText
                                }
                            }

                            // Format badge
                            Rectangle {
                                height: 18
                                width: fmtBadgeText.implicitWidth + 8
                                radius: 4
                                color: Theme.color.muted

                                Text {
                                    id: fmtBadgeText
                                    anchors.centerIn: parent
                                    text: modelData.format
                                    font.pixelSize: 8
                                    color: Theme.color.dimText
                                }
                            }
                        }

                        HoverHandler { id: itemHover }
                        TapHandler {
                            onTapped: {
                                root.selectedIndex = index;
                                root.selectCurrent();
                            }
                        }
                    }
                }
            }

            // Footer
            Rectangle {
                Layout.fillWidth: true
                height: 32
                color: Theme.color.surfaceElevated

                Rectangle {
                    anchors.top: parent.top
                    anchors.left: parent.left
                    anchors.right: parent.right
                    height: 1
                    color: Theme.color.border
                }

                RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: 14
                    anchors.rightMargin: 14

                    Text {
                        text: root.searchResults.length + " matching items"
                        font.family: Theme.font.family
                        font.pixelSize: Theme.font.caption
                        color: Theme.color.dimText
                    }

                    Item { Layout.fillWidth: true }

                    Text {
                        text: "↑/↓ to navigate   •   Enter to select   •   Esc to close"
                        font.family: Theme.font.family
                        font.pixelSize: Theme.font.caption
                        color: Theme.color.dimText
                    }
                }
            }
        }
    }
}
