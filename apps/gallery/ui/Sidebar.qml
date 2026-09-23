import QtQuick
import QtQuick.Layouts
import QtQuick.Controls
import "."

Rectangle {
    id: root
    width: ViewState.sidebarOpen ? Theme.geometry.sidebarWidth : 0
    visible: width > 0
    color: Theme.color.surface
    clip: true

    Behavior on width {
        NumberAnimation { duration: 150; easing.type: Easing.OutQuad }
    }

    Rectangle {
        anchors.top: parent.top
        anchors.bottom: parent.bottom
        anchors.right: parent.right
        width: 1
        color: Theme.color.border
    }

    ScrollView {
        anchors.fill: parent
        anchors.rightMargin: 1
        contentWidth: availableWidth

        ColumnLayout {
            width: parent.width
            spacing: 16
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.top: parent.top
            anchors.margins: 10

            // Section: LIBRARY
            ColumnLayout {
                Layout.fillWidth: true
                spacing: 2

                Text {
                    text: "LIBRARY"
                    font.family: Theme.font.family
                    font.pixelSize: Theme.font.caption
                    font.bold: true
                    color: Theme.color.dimText
                    Layout.leftMargin: 8
                    Layout.bottomMargin: 4
                }

                Repeater {
                    model: [
                        { id: "all_photos", title: "All Photos", icon: "image" },
                        { id: "favorites", title: "Favorites", icon: "star" },
                        { id: "recent", title: "Recently Added", icon: "clock" },
                        { id: "trash", title: "Trash", icon: "trash" }
                    ]

                    Rectangle {
                        id: libRow
                        property bool active: ViewState.currentSection === modelData.id
                        Layout.fillWidth: true
                        height: 30
                        radius: Theme.geometry.pillRadius
                        color: libRow.active ? Theme.color.selection : (itemHover.hovered ? Theme.color.surfaceElevated : "transparent")

                        RowLayout {
                            anchors.fill: parent
                            anchors.leftMargin: 8
                            anchors.rightMargin: 8
                            spacing: 8

                            NexusIcon {
                                name: modelData.icon
                                size: 15
                                filled: modelData.id === "favorites" && libRow.active
                                color: libRow.active ? Theme.color.accent : (itemHover.hovered ? Theme.color.foreground : Theme.color.dimText)
                            }

                            Text {
                                text: modelData.title
                                font.family: Theme.font.family
                                font.pixelSize: Theme.font.body
                                font.bold: libRow.active
                                color: Theme.color.foreground
                                Layout.fillWidth: true
                            }

                            Rectangle {
                                property int sectionCount: {
                                    if (modelData.id === "all_photos") return ViewState.totalItems;
                                    if (modelData.id === "favorites") return ViewState.favoriteCount;
                                    return 0;
                                }
                                visible: sectionCount > 0
                                width: Math.max(18, countText.implicitWidth + 10)
                                height: 18
                                radius: 9
                                color: Theme.color.surfaceElevated
                                border.color: Theme.color.border
                                border.width: 1

                                Text {
                                    id: countText
                                    anchors.centerIn: parent
                                    text: parent.sectionCount
                                    font.family: Theme.font.family
                                    font.pixelSize: 10
                                    font.bold: true
                                    color: Theme.color.foreground
                                }
                            }
                        }

                        HoverHandler { id: itemHover }
                        TapHandler {
                            onTapped: ViewState.setSection(modelData.id, modelData.title)
                        }
                    }
                }
            }

            // Section: MEDIA TYPES
            ColumnLayout {
                Layout.fillWidth: true
                spacing: 2

                Text {
                    text: "MEDIA TYPES"
                    font.family: Theme.font.family
                    font.pixelSize: Theme.font.caption
                    font.bold: true
                    color: Theme.color.dimText
                    Layout.leftMargin: 8
                    Layout.bottomMargin: 4
                }

                Repeater {
                    model: [
                        { id: "photos", title: "Photos", icon: "camera" },
                        { id: "screenshots", title: "Screenshots", icon: "monitor" },
                        { id: "videos", title: "Videos", icon: "film" }
                    ]

                    Rectangle {
                        id: mediaRow
                        property bool active: ViewState.currentSection === modelData.id
                        Layout.fillWidth: true
                        height: 30
                        radius: Theme.geometry.pillRadius
                        color: mediaRow.active ? Theme.color.selection : (itemHover.hovered ? Theme.color.surfaceElevated : "transparent")

                        RowLayout {
                            anchors.fill: parent
                            anchors.leftMargin: 8
                            anchors.rightMargin: 8
                            spacing: 8

                            NexusIcon {
                                name: modelData.icon
                                size: 15
                                color: mediaRow.active ? Theme.color.accent : (itemHover.hovered ? Theme.color.foreground : Theme.color.dimText)
                            }

                            Text {
                                text: modelData.title
                                font.family: Theme.font.family
                                font.pixelSize: Theme.font.body
                                font.bold: mediaRow.active
                                color: Theme.color.foreground
                                Layout.fillWidth: true
                            }

                            Rectangle {
                                property int sectionCount: {
                                    if (modelData.id === "screenshots") return ViewState.screenshotCount;
                                    if (modelData.id === "photos") return ViewState.photoCount;
                                    if (modelData.id === "videos") return ViewState.videoCount;
                                    return 0;
                                }
                                visible: sectionCount > 0
                                width: Math.max(18, mediaCountText.implicitWidth + 10)
                                height: 18
                                radius: 9
                                color: Theme.color.surfaceElevated
                                border.color: Theme.color.border
                                border.width: 1

                                Text {
                                    id: mediaCountText
                                    anchors.centerIn: parent
                                    text: parent.sectionCount
                                    font.family: Theme.font.family
                                    font.pixelSize: 10
                                    font.bold: true
                                    color: Theme.color.foreground
                                }
                            }
                        }

                        HoverHandler { id: itemHover }
                        TapHandler {
                            onTapped: ViewState.setSection(modelData.id, modelData.title)
                        }
                    }
                }
            }

            // Section: ALBUMS
            ColumnLayout {
                Layout.fillWidth: true
                spacing: 2

                RowLayout {
                    Layout.fillWidth: true
                    Layout.leftMargin: 8
                    Layout.rightMargin: 8
                    Layout.bottomMargin: 4

                    Text {
                        text: "ALBUMS"
                        font.family: Theme.font.family
                        font.pixelSize: Theme.font.caption
                        font.bold: true
                        color: Theme.color.dimText
                        Layout.fillWidth: true
                    }

                    Rectangle {
                        width: 18
                        height: 18
                        radius: 9
                        color: addAlbumHover.hovered ? Theme.color.selection : "transparent"

                        NexusIcon {
                            anchors.centerIn: parent
                            name: "plus"
                            size: 11
                            color: addAlbumHover.hovered ? Theme.color.foreground : Theme.color.dimText
                        }

                        HoverHandler { id: addAlbumHover }
                        TapHandler {
                            onTapped: {
                                var num = ViewState.albums.length + 1;
                                Backend.createAlbum("New Album " + num);
                            }
                        }
                    }
                }

                Repeater {
                    model: ViewState.albums

                    Rectangle {
                        id: userAlbumRow
                        property bool active: ViewState.currentSection === modelData.id
                        property bool isEditing: false
                        Layout.fillWidth: true
                        height: 30
                        radius: Theme.geometry.pillRadius
                        color: userAlbumRow.active ? Theme.color.selection : (userAlbumHover.hovered ? Theme.color.surfaceElevated : "transparent")

                        RowLayout {
                            anchors.fill: parent
                            anchors.leftMargin: 8
                            anchors.rightMargin: 6
                            spacing: 6

                            NexusIcon {
                                name: "folder"
                                size: 14
                                color: userAlbumRow.active ? Theme.color.accent : (userAlbumHover.hovered ? Theme.color.foreground : Theme.color.dimText)
                            }

                            // View Mode: Display album name
                            Text {
                                visible: !userAlbumRow.isEditing
                                text: modelData.name
                                font.family: Theme.font.family
                                font.pixelSize: Theme.font.body
                                font.bold: userAlbumRow.active
                                color: Theme.color.foreground
                                elide: Text.ElideRight
                                Layout.fillWidth: true
                            }

                            // Edit Mode: Inline TextInput
                            Rectangle {
                                visible: userAlbumRow.isEditing
                                Layout.fillWidth: true
                                height: 22
                                radius: 4
                                color: Theme.color.background
                                border.color: Theme.color.accent
                                border.width: 1

                                TextInput {
                                    id: inlineInput
                                    anchors.fill: parent
                                    anchors.leftMargin: 4
                                    anchors.rightMargin: 4
                                    verticalAlignment: TextInput.AlignVCenter
                                    text: modelData.name
                                    font.family: Theme.font.family
                                    font.pixelSize: Theme.font.bodySmall
                                    color: Theme.color.foreground
                                    selectByMouse: true

                                    Connections {
                                        target: userAlbumRow
                                        function onIsEditingChanged() {
                                            if (userAlbumRow.isEditing) {
                                                inlineInput.text = modelData.name;
                                                inlineInput.forceActiveFocus();
                                                inlineInput.selectAll();
                                            }
                                        }
                                    }

                                    Keys.onReturnPressed: commitEdit()
                                    Keys.onEnterPressed: commitEdit()
                                    Keys.onEscapePressed: {
                                        inlineInput.text = modelData.name;
                                        userAlbumRow.isEditing = false;
                                    }
                                    onEditingFinished: commitEdit()

                                    function commitEdit() {
                                        if (!userAlbumRow.isEditing) return;
                                        userAlbumRow.isEditing = false;
                                        var newName = text.trim();
                                        if (newName.length > 0 && newName !== modelData.name) {
                                            Backend.renameAlbum(modelData.id, newName);
                                            ViewState.showToast("Album renamed to '" + newName + "'");
                                        }
                                    }
                                }
                            }

                            // Count Badge (when not editing and not hovering)
                            Rectangle {
                                visible: !userAlbumRow.isEditing && !userAlbumHover.hovered && modelData.photo_ids && modelData.photo_ids.length > 0
                                width: Math.max(18, albumCountText.implicitWidth + 10)
                                height: 18
                                radius: 9
                                color: Theme.color.surfaceElevated
                                border.color: Theme.color.border
                                border.width: 1

                                Text {
                                    id: albumCountText
                                    anchors.centerIn: parent
                                    text: modelData.photo_ids ? modelData.photo_ids.length : 0
                                    font.family: Theme.font.family
                                    font.pixelSize: 10
                                    font.bold: true
                                    color: Theme.color.foreground
                                }
                            }

                            // Action Buttons (visible when hovered and not editing)
                            RowLayout {
                                visible: !userAlbumRow.isEditing && userAlbumHover.hovered
                                spacing: 2

                                // Edit button
                                Rectangle {
                                    width: 20
                                    height: 20
                                    radius: 10
                                    color: editAlbumHover.hovered ? Theme.color.selection : "transparent"

                                    NexusIcon {
                                        anchors.centerIn: parent
                                        name: "edit"
                                        size: 11
                                        color: editAlbumHover.hovered ? Theme.color.foreground : Theme.color.dimText
                                    }

                                    HoverHandler { id: editAlbumHover }
                                    TapHandler {
                                        onTapped: {
                                            userAlbumRow.isEditing = true;
                                        }
                                    }
                                }

                                // Delete button
                                Rectangle {
                                    width: 20
                                    height: 20
                                    radius: 10
                                    color: delAlbumHover.hovered ? Theme.color.danger : "transparent"

                                    NexusIcon {
                                        anchors.centerIn: parent
                                        name: "trash"
                                        size: 11
                                        color: delAlbumHover.hovered ? "#ffffff" : Theme.color.dimText
                                    }

                                    HoverHandler { id: delAlbumHover }
                                    TapHandler {
                                        onTapped: {
                                            Backend.deleteAlbum(modelData.id);
                                            ViewState.showToast("Album '" + modelData.name + "' deleted");
                                        }
                                    }
                                }
                            }
                        }

                        HoverHandler { id: userAlbumHover }
                        TapHandler {
                            enabled: !userAlbumRow.isEditing
                            onTapped: ViewState.setSection(modelData.id, modelData.name)
                            onDoubleTapped: {
                                userAlbumRow.isEditing = true;
                            }
                        }
                    }
                }
            }
        }
    }
}
