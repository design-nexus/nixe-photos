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
                        { id: "all_photos", title: "All Photos", icon: "🖼" },
                        { id: "timeline", title: "Timeline", icon: "📅" },
                        { id: "favorites", title: "Favorites", icon: "⭐" },
                        { id: "recent", title: "Recently Added", icon: "🕒" },
                        { id: "trash", title: "Trash", icon: "🗑" }
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

                            Text {
                                text: modelData.icon
                                font.pixelSize: 13
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
                                visible: modelData.id === "all_photos" && ViewState.totalItems > 0
                                width: countText.implicitWidth + 8
                                height: 16
                                radius: 8
                                color: Theme.color.muted

                                Text {
                                    id: countText
                                    anchors.centerIn: parent
                                    text: ViewState.totalItems
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
                        { id: "photos", title: "Photos", icon: "📸" },
                        { id: "screenshots", title: "Screenshots", icon: "🖥" },
                        { id: "videos", title: "Videos", icon: "🎞" }
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

                            Text {
                                text: modelData.icon
                                font.pixelSize: 13
                            }

                            Text {
                                text: modelData.title
                                font.family: Theme.font.family
                                font.pixelSize: Theme.font.body
                                font.bold: mediaRow.active
                                color: Theme.color.foreground
                                Layout.fillWidth: true
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

                Text {
                    text: "ALBUMS"
                    font.family: Theme.font.family
                    font.pixelSize: Theme.font.caption
                    font.bold: true
                    color: Theme.color.dimText
                    Layout.leftMargin: 8
                    Layout.bottomMargin: 4
                }

                Rectangle {
                    id: albumRow
                    property bool active: ViewState.currentSection === "album_favorites"
                    Layout.fillWidth: true
                    height: 30
                    radius: Theme.geometry.pillRadius
                    color: albumRow.active ? Theme.color.selection : (albumHover.hovered ? Theme.color.surfaceElevated : "transparent")

                    RowLayout {
                        anchors.fill: parent
                        anchors.leftMargin: 8
                        anchors.rightMargin: 8
                        spacing: 8

                        Text {
                            text: "📁"
                            font.pixelSize: 13
                        }

                        Text {
                            text: "Library Highlights"
                            font.family: Theme.font.family
                            font.pixelSize: Theme.font.body
                            font.bold: albumRow.active
                            color: Theme.color.foreground
                            Layout.fillWidth: true
                        }
                    }

                    HoverHandler { id: albumHover }
                    TapHandler {
                        onTapped: ViewState.setSection("album_favorites", "Library Highlights")
                    }
                }
            }
        }
    }
}
