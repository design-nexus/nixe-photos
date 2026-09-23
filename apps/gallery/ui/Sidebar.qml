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
                        { id: "timeline", title: "Timeline", icon: "calendar" },
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
                                width: countText.implicitWidth + 8
                                height: 16
                                radius: 8
                                color: Theme.color.muted

                                Text {
                                    id: countText
                                    anchors.centerIn: parent
                                    text: parent.sectionCount
                                    font.pixelSize: 9
                                    color: Theme.color.dimText
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
                                width: mediaCountText.implicitWidth + 8
                                height: 16
                                radius: 8
                                color: Theme.color.muted

                                Text {
                                    id: mediaCountText
                                    anchors.centerIn: parent
                                    text: parent.sectionCount
                                    font.pixelSize: 9
                                    color: Theme.color.dimText
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
                        Layout.fillWidth: true
                        height: 30
                        radius: Theme.geometry.pillRadius
                        color: userAlbumRow.active ? Theme.color.selection : (userAlbumHover.hovered ? Theme.color.surfaceElevated : "transparent")

                        RowLayout {
                            anchors.fill: parent
                            anchors.leftMargin: 8
                            anchors.rightMargin: 8
                            spacing: 8

                            NexusIcon {
                                name: "folder"
                                size: 15
                                color: userAlbumRow.active ? Theme.color.accent : (userAlbumHover.hovered ? Theme.color.foreground : Theme.color.dimText)
                            }

                            Text {
                                text: modelData.name
                                font.family: Theme.font.family
                                font.pixelSize: Theme.font.body
                                font.bold: userAlbumRow.active
                                color: Theme.color.foreground
                                elide: Text.ElideRight
                                Layout.fillWidth: true
                            }

                            Rectangle {
                                visible: modelData.photo_ids && modelData.photo_ids.length > 0
                                width: albumCountText.implicitWidth + 8
                                height: 16
                                radius: 8
                                color: Theme.color.muted

                                Text {
                                    id: albumCountText
                                    anchors.centerIn: parent
                                    text: modelData.photo_ids ? modelData.photo_ids.length : 0
                                    font.pixelSize: 9
                                    color: Theme.color.dimText
                                }
                            }
                        }

                        HoverHandler { id: userAlbumHover }
                        TapHandler {
                            onTapped: ViewState.setSection(modelData.id, modelData.name)
                        }
                    }
                }
            }
        }
    }
}
