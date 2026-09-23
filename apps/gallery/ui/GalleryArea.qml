import QtQuick
import QtQuick.Layouts
import QtQuick.Controls
import "."

Rectangle {
    id: root
    color: Theme.color.background

    readonly property real baseCardSize: 110 + ViewState.zoomLevel * 150
    readonly property real availableGridWidth: galleryScroll && galleryScroll.width > 0 ? galleryScroll.width : (root.width - 36)

    onAvailableGridWidthChanged: updateColumns()
    onBaseCardSizeChanged: updateColumns()

    function updateColumns() {
        var spacing = 12;
        var cols = Math.max(1, Math.floor((availableGridWidth + spacing) / (baseCardSize + spacing)));
        ViewState.gridColumns = cols;
    }

    Component.onCompleted: updateColumns()

    function ensureVisible(item) {
        if (!item || !galleryScroll || !galleryScroll.visible) return;
        var flick = galleryScroll.contentItem;
        if (!flick) return;

        var pos = item.mapToItem(galleryScroll, 0, 0);
        var margin = 24;

        if (pos.y < margin) {
            flick.contentY = Math.max(0, flick.contentY + pos.y - margin);
        } else if (pos.y + item.height > galleryScroll.height - margin) {
            var maxScroll = Math.max(0, flick.contentHeight - galleryScroll.height);
            flick.contentY = Math.min(maxScroll, flick.contentY + (pos.y + item.height - galleryScroll.height + margin));
        }
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 18
        spacing: 16

        // Section Title & Action Row
        RowLayout {
            Layout.fillWidth: true

            ColumnLayout {
                spacing: 2
                Text {
                    text: ViewState.currentSectionTitle
                    font.family: Theme.font.family
                    font.pixelSize: 20
                    font.bold: true
                    color: Theme.color.foreground
                }

                Text {
                    text: ViewState.displayedPhotos.length + " photos" +
                          (ViewState.selectedCount > 1 ? (" • " + ViewState.selectedCount + " selected") : "") +
                          " • " + ViewState.libraryPath
                    font.family: Theme.font.family
                    font.pixelSize: Theme.font.caption
                    color: Theme.color.dimText
                }
            }

            Item { Layout.fillWidth: true }

            // Album Management Actions (When viewing an album)
            RowLayout {
                spacing: 8
                visible: ViewState.currentSection.indexOf("album_") === 0

                // Rename Album Button
                Rectangle {
                    height: 26
                    width: renameText.implicitWidth + 28
                    radius: Theme.geometry.pillRadius
                    color: renameHover.hovered ? Theme.color.selection : Theme.color.surface
                    border.color: Theme.color.border
                    border.width: 1

                    RowLayout {
                        anchors.centerIn: parent
                        spacing: 6
                        NexusIcon { name: "edit"; size: 12; color: Theme.color.foreground }
                        Text {
                            id: renameText
                            text: "Rename Album"
                            font.family: Theme.font.family
                            font.pixelSize: Theme.font.bodySmall
                            color: Theme.color.foreground
                        }
                    }
                    HoverHandler { id: renameHover }
                    TapHandler {
                        onTapped: ViewState.openRenameAlbumDialog(ViewState.currentSection, ViewState.currentSectionTitle)
                    }
                }

                // Delete Album Button
                Rectangle {
                    height: 26
                    width: delAlbText.implicitWidth + 28
                    radius: Theme.geometry.pillRadius
                    color: delAlbHover.hovered ? Theme.color.danger : Theme.color.surface
                    border.color: Theme.color.border
                    border.width: 1

                    RowLayout {
                        anchors.centerIn: parent
                        spacing: 6
                        NexusIcon {
                            name: "trash"
                            size: 12
                            color: delAlbHover.hovered ? "#ffffff" : Theme.color.danger
                        }
                        Text {
                            id: delAlbText
                            text: "Delete Album"
                            font.family: Theme.font.family
                            font.pixelSize: Theme.font.bodySmall
                            color: delAlbHover.hovered ? "#ffffff" : Theme.color.danger
                        }
                    }
                    HoverHandler { id: delAlbHover }
                    TapHandler {
                        onTapped: {
                            var albId = ViewState.currentSection;
                            var albTitle = ViewState.currentSectionTitle;
                            Backend.deleteAlbum(albId);
                            ViewState.showToast("Album '" + albTitle + "' deleted");
                        }
                    }
                }
            }

            // Select All / Clear Selection Pills
            RowLayout {
                spacing: 8
                visible: ViewState.displayedPhotos.length > 0

                Rectangle {
                    visible: ViewState.selectedCount > 1
                    height: 26
                    width: clearSelText.implicitWidth + 16
                    radius: Theme.geometry.pillRadius
                    color: clearSelHover.hovered ? Theme.color.selection : Theme.color.surface
                    border.color: Theme.color.border
                    border.width: 1

                    Text {
                        id: clearSelText
                        anchors.centerIn: parent
                        text: "Deselect All"
                        font.family: Theme.font.family
                        font.pixelSize: Theme.font.bodySmall
                        color: Theme.color.dimText
                    }

                    HoverHandler { id: clearSelHover }
                    TapHandler { onTapped: ViewState.clearSelection() }
                }

                Rectangle {
                    height: 26
                    width: selAllText.implicitWidth + 16
                    radius: Theme.geometry.pillRadius
                    color: selAllHover.hovered ? Theme.color.selection : Theme.color.surface
                    border.color: Theme.color.border
                    border.width: 1

                    Text {
                        id: selAllText
                        anchors.centerIn: parent
                        text: "Select All"
                        font.family: Theme.font.family
                        font.pixelSize: Theme.font.bodySmall
                        color: Theme.color.foreground
                    }

                    HoverHandler { id: selAllHover }
                    TapHandler { onTapped: ViewState.selectAll() }
                }
            }

            // Rescan Library Button
            Rectangle {
                height: 26
                width: rescanText.implicitWidth + 26
                radius: Theme.geometry.pillRadius
                color: rescanHover.hovered ? Theme.color.selection : Theme.color.surface
                border.color: Theme.color.border
                border.width: 1

                RowLayout {
                    anchors.centerIn: parent
                    spacing: 6
                    NexusIcon {
                        name: "rotate"
                        size: 11
                        color: Theme.color.foreground
                    }
                    Text {
                        id: rescanText
                        text: "Rescan"
                        font.family: Theme.font.family
                        font.pixelSize: Theme.font.bodySmall
                        color: Theme.color.foreground
                    }
                }

                HoverHandler { id: rescanHover }
                TapHandler {
                    onTapped: Backend.scanLibrary()
                }
            }
        }

        // Empty State
        Rectangle {
            visible: ViewState.displayedPhotos.length === 0
            Layout.fillWidth: true
            Layout.fillHeight: true
            color: "transparent"

            ColumnLayout {
                anchors.centerIn: parent
                spacing: 14

                // Wireframe Camera Emblem
                Rectangle {
                    Layout.alignment: Qt.AlignHCenter
                    width: 76
                    height: 76
                    radius: 20
                    color: Theme.color.surfaceElevated
                    border.color: Theme.color.border
                    border.width: 1

                    NexusIcon {
                        anchors.centerIn: parent
                        name: ViewState.currentSection.indexOf("album_") === 0 ? "folder" :
                              (ViewState.currentSection === "videos" ? "film" : "camera")
                        size: 38
                        strokeWidth: 1.5
                        color: Theme.color.dimText
                    }
                }

                Text {
                    text: ViewState.currentSection.indexOf("album_") === 0 ? "This album is empty" :
                          (ViewState.currentSection === "videos" ? "No videos found" :
                          (ViewState.currentSection === "favorites" ? "No favorites yet" :
                          (ViewState.currentSection === "screenshots" ? "No screenshots found" : "No photos found")))
                    font.family: Theme.font.family
                    font.pixelSize: Theme.font.title
                    font.bold: true
                    color: Theme.color.foreground
                    Layout.alignment: Qt.AlignHCenter
                }

                Text {
                    text: ViewState.currentSection.indexOf("album_") === 0 ? "Select photos from All Photos and add them to this album" :
                          (ViewState.currentSection === "videos" ? "Add video files (.mp4, .mov, .mkv) to " + ViewState.libraryPath :
                          (ViewState.currentSection === "favorites" ? "Press F or click the star to favorite any photo" :
                          "Add media to " + ViewState.libraryPath + " or click Rescan"))
                    font.family: Theme.font.family
                    font.pixelSize: Theme.font.bodySmall
                    color: Theme.color.dimText
                    Layout.alignment: Qt.AlignHCenter
                }
            }
        }

        // Grouped Timeline Grid
        ScrollView {
            id: galleryScroll
            visible: ViewState.displayedPhotos.length > 0
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true

            ColumnLayout {
                width: parent.width
                spacing: 20

                Repeater {
                    model: ViewState.timelineGroups

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 10

                        // Sticky Date Section Header
                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 10

                            Text {
                                text: modelData.header
                                font.family: Theme.font.family
                                font.pixelSize: 15
                                font.bold: true
                                color: Theme.color.foreground
                            }

                            Text {
                                text: modelData.subheader
                                font.family: Theme.font.family
                                font.pixelSize: Theme.font.caption
                                color: Theme.color.dimText
                            }

                            Rectangle {
                                height: 1
                                Layout.fillWidth: true
                                color: Theme.color.border
                            }
                        }

                        // Flow of Photos in this Timeline Group
                        Flow {
                            Layout.fillWidth: true
                            spacing: 12

                            Repeater {
                                model: modelData.items

                                Rectangle {
                                    id: card
                                    property var photoData: modelData
                                    property bool isSelected: ViewState.isSelected(modelData.id)
                                    width: root.baseCardSize
                                    height: root.baseCardSize + 24
                                    radius: Theme.geometry.radiusMedium
                                    color: Theme.color.surface
                                    border.color: isSelected ? Theme.color.accent : (cardHover.hovered ? Theme.color.selection : Theme.color.border)
                                    border.width: isSelected ? 2 : 1
                                    clip: true

                                    onIsSelectedChanged: {
                                        if (isSelected) {
                                            root.ensureVisible(card);
                                        }
                                    }

                                    Behavior on width {
                                        NumberAnimation { duration: 80 }
                                    }
                                    Behavior on height {
                                        NumberAnimation { duration: 80 }
                                    }

                                    ColumnLayout {
                                        anchors.fill: parent
                                        spacing: 0

                                        // Thumbnail
                                        Rectangle {
                                            Layout.fillWidth: true
                                            Layout.fillHeight: true
                                            radius: Theme.geometry.radiusMedium - 2
                                            color: Theme.color.background
                                            clip: true

                                            Image {
                                                id: thumbImg
                                                anchors.fill: parent
                                                source: "file://" + (photoData.thumbnail_path || photoData.path)
                                                fillMode: Image.PreserveAspectCrop
                                                rotation: ViewState.getRotation(photoData.id)
                                                asynchronous: true
                                                mipmap: true
                                            }

                                            // Fallback icon
                                            NexusIcon {
                                                anchors.centerIn: parent
                                                visible: thumbImg.status !== Image.Ready
                                                name: photoData.media_type === "videos" ? "film" : "camera"
                                                size: 24
                                                color: Theme.color.dimText
                                            }

                                            // Video indicator badge on thumbnail
                                            Rectangle {
                                                anchors.bottom: parent.bottom
                                                anchors.left: parent.left
                                                anchors.margins: 6
                                                visible: photoData.media_type === "videos"
                                                height: 18
                                                width: vidBadgeRow.implicitWidth + 8
                                                radius: 4
                                                color: Qt.rgba(0, 0, 0, 0.65)

                                                RowLayout {
                                                    id: vidBadgeRow
                                                    anchors.centerIn: parent
                                                    spacing: 3

                                                    NexusIcon {
                                                        name: "film"
                                                        size: 10
                                                        color: "#ffffff"
                                                    }

                                                    Text {
                                                        text: photoData.format || "VIDEO"
                                                        font.family: Theme.font.family
                                                        font.pixelSize: 8
                                                        font.bold: true
                                                        color: "#ffffff"
                                                    }
                                                }
                                            }

                                            // Selection overlay tint
                                            Rectangle {
                                                anchors.fill: parent
                                                color: Theme.color.accent
                                                opacity: card.isSelected ? 0.16 : 0.0
                                            }
                                        }

                                        // Caption
                                        Rectangle {
                                            Layout.fillWidth: true
                                            height: 24
                                            color: Theme.color.surface

                                            RowLayout {
                                                anchors.fill: parent
                                                anchors.leftMargin: 8
                                                anchors.rightMargin: 6
                                                spacing: 4

                                                Text {
                                                    text: photoData.file_name
                                                    font.family: Theme.font.family
                                                    font.pixelSize: Theme.font.caption
                                                    font.bold: card.isSelected
                                                    color: card.isSelected ? Theme.color.accent : Theme.color.foreground
                                                    elide: Text.ElideMiddle
                                                    Layout.fillWidth: true
                                                }

                                                Text {
                                                    text: photoData.format
                                                    font.family: Theme.font.family
                                                    font.pixelSize: 8
                                                    color: Theme.color.dimText
                                                }
                                            }
                                        }
                                    }

                                    // Card Click & Selection Area (z: 1)
                                    MouseArea {
                                        anchors.fill: parent
                                        z: 1
                                        acceptedButtons: Qt.LeftButton
                                        onClicked: function(mouse) {
                                            if (mouse.modifiers & Qt.ControlModifier) {
                                                ViewState.toggleSelect(photoData);
                                            } else if (mouse.modifiers & Qt.ShiftModifier) {
                                                var gIdx = ViewState.findIndexById(photoData.id);
                                                ViewState.selectRange(gIdx);
                                            } else {
                                                var gIdx = ViewState.findIndexById(photoData.id);
                                                ViewState.selectSingle(photoData, gIdx);
                                            }
                                        }
                                        onDoubleClicked: function() {
                                            var gIdx = ViewState.findIndexById(photoData.id);
                                            ViewState.selectSingle(photoData, gIdx);
                                            ViewState.openLightbox();
                                        }
                                    }

                                    // Top-Left Multi-Selection Checkmark Badge (z: 10)
                                    Rectangle {
                                        z: 10
                                        anchors.top: parent.top
                                        anchors.left: parent.left
                                        anchors.margins: 6
                                        width: 24
                                        height: 24
                                        radius: 12
                                        color: card.isSelected ? Theme.color.accent : (cardHover.hovered ? Qt.rgba(0, 0, 0, 0.5) : "transparent")
                                        border.color: card.isSelected ? Theme.color.accent : (cardHover.hovered ? "#ffffff" : "transparent")
                                        border.width: 1.5
                                        visible: card.isSelected || cardHover.hovered || ViewState.selectedCount > 1

                                        NexusIcon {
                                            anchors.centerIn: parent
                                            visible: card.isSelected
                                            name: "check"
                                            size: 11
                                            color: Theme.color.background
                                        }

                                        MouseArea {
                                            anchors.fill: parent
                                            cursorShape: Qt.PointingHandCursor
                                            acceptedButtons: Qt.LeftButton
                                            onClicked: function(mouse) {
                                                mouse.accepted = true;
                                                ViewState.toggleSelect(photoData);
                                            }
                                        }
                                    }

                                    // Top-Right Favorite Star Button (z: 10)
                                    Rectangle {
                                        id: favBtn
                                        z: 10
                                        anchors.top: parent.top
                                        anchors.right: parent.right
                                        anchors.margins: 6
                                        width: 26
                                        height: 26
                                        radius: 13
                                        property bool isFav: ViewState.isFavorite(photoData.id)
                                        color: isFav ? Theme.color.accent : (favMouse.containsMouse ? Theme.color.surfaceElevated : Qt.rgba(0, 0, 0, 0.5))
                                        visible: isFav || cardHover.hovered

                                        NexusIcon {
                                            anchors.centerIn: parent
                                            name: "star"
                                            size: 13
                                            filled: favBtn.isFav
                                            color: favBtn.isFav ? Theme.color.background : (favMouse.containsMouse ? Theme.color.accent : Theme.color.foreground)
                                        }

                                        MouseArea {
                                            id: favMouse
                                            anchors.fill: parent
                                            hoverEnabled: true
                                            cursorShape: Qt.PointingHandCursor
                                            acceptedButtons: Qt.LeftButton
                                            onClicked: function(mouse) {
                                                mouse.accepted = true;
                                                Backend.toggleFavorite(photoData.id);
                                            }
                                        }
                                    }

                                    HoverHandler { id: cardHover }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
