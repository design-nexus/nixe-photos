import QtQuick
import QtQuick.Layouts
import QtQuick.Controls
import "."

Rectangle {
    id: root
    color: Theme.color.background

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
                    text: ViewState.displayedPhotos.length + " photos • " + ViewState.libraryPath
                    font.family: Theme.font.family
                    font.pixelSize: Theme.font.caption
                    color: Theme.color.dimText
                }
            }

            Item { Layout.fillWidth: true }

            // Rescan Library Button
            Rectangle {
                height: 26
                width: rescanText.implicitWidth + 24
                radius: Theme.geometry.pillRadius
                color: rescanHover.hovered ? Theme.color.selection : Theme.color.surface
                border.color: Theme.color.border
                border.width: 1

                RowLayout {
                    anchors.centerIn: parent
                    spacing: 6
                    Text { text: "🔄"; font.pixelSize: 10 }
                    Text {
                        id: rescanText
                        text: "Rescan Library"
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

        // Empty State when no photos
        Rectangle {
            visible: ViewState.displayedPhotos.length === 0
            Layout.fillWidth: true
            Layout.fillHeight: true
            color: "transparent"

            ColumnLayout {
                anchors.centerIn: parent
                spacing: 10

                Text {
                    text: "📷"
                    font.pixelSize: 48
                    Layout.alignment: Qt.AlignHCenter
                }

                Text {
                    text: "No photos found in this view"
                    font.family: Theme.font.family
                    font.pixelSize: Theme.font.title
                    font.bold: true
                    color: Theme.color.foreground
                    Layout.alignment: Qt.AlignHCenter
                }

                Text {
                    text: "Add images to " + ViewState.libraryPath + " or switch sections in the sidebar."
                    font.family: Theme.font.family
                    font.pixelSize: Theme.font.bodySmall
                    color: Theme.color.dimText
                    Layout.alignment: Qt.AlignHCenter
                }
            }
        }

        // Real Photos Grid
        ScrollView {
            visible: ViewState.displayedPhotos.length > 0
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true

            Flow {
                width: parent.width
                spacing: 12

                Repeater {
                    model: ViewState.displayedPhotos

                    Rectangle {
                        id: card
                        property real baseCardSize: 130 + ViewState.zoomLevel * 140
                        property bool isSelected: ViewState.selectedItem && ViewState.selectedItem.id === modelData.id
                        width: baseCardSize
                        height: baseCardSize + 24
                        radius: Theme.geometry.radiusMedium
                        color: Theme.color.surface
                        border.color: isSelected ? Theme.color.accent : (cardHover.hovered ? Theme.color.selection : Theme.color.border)
                        border.width: isSelected ? 2 : 1
                        clip: true

                        Behavior on width {
                            NumberAnimation { duration: 100 }
                        }
                        Behavior on height {
                            NumberAnimation { duration: 100 }
                        }

                        ColumnLayout {
                            anchors.fill: parent
                            spacing: 0

                            // Thumbnail Image Container
                            Rectangle {
                                Layout.fillWidth: true
                                Layout.fillHeight: true
                                radius: Theme.geometry.radiusMedium - 2
                                color: Theme.color.background
                                clip: true

                                Image {
                                    id: thumbImg
                                    anchors.fill: parent
                                    source: "file://" + (modelData.thumbnail_path || modelData.path)
                                    fillMode: Image.PreserveAspectCrop
                                    asynchronous: true
                                    mipmap: true
                                }

                                // Fallback icon if image is still loading
                                Text {
                                    anchors.centerIn: parent
                                    visible: thumbImg.status !== Image.Ready
                                    text: "📷"
                                    font.pixelSize: 24
                                    color: Theme.color.dimText
                                }

                                // Selection glow overlay
                                Rectangle {
                                    anchors.fill: parent
                                    color: Theme.color.accent
                                    opacity: card.isSelected ? 0.12 : 0.0
                                }
                            }

                            // Card Caption Row
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
                                        text: modelData.file_name
                                        font.family: Theme.font.family
                                        font.pixelSize: Theme.font.caption
                                        font.bold: card.isSelected
                                        color: card.isSelected ? Theme.color.accent : Theme.color.foreground
                                        elide: Text.ElideMiddle
                                        Layout.fillWidth: true
                                    }

                                    // Media format tag
                                    Text {
                                        text: modelData.format
                                        font.family: Theme.font.family
                                        font.pixelSize: 8
                                        color: Theme.color.dimText
                                    }
                                }
                            }
                        }

                        // Favorite star button
                        Rectangle {
                            anchors.top: parent.top
                            anchors.right: parent.right
                            anchors.margins: 6
                            width: 24
                            height: 24
                            radius: 12
                            color: modelData.is_favorite ? Theme.color.accent : (starHover.hovered ? Theme.color.surfaceElevated : Qt.rgba(0, 0, 0, 0.4))
                            visible: modelData.is_favorite || cardHover.hovered

                            Text {
                                anchors.centerIn: parent
                                text: "★"
                                font.pixelSize: 12
                                color: modelData.is_favorite ? Theme.color.background : (starHover.hovered ? Theme.color.accent : "#ffffff")
                            }

                            HoverHandler { id: starHover }
                            TapHandler {
                                onTapped: Backend.toggleFavorite(modelData.id)
                            }
                        }

                        HoverHandler { id: cardHover }
                        TapHandler {
                            onTapped: {
                                ViewState.selectPhoto(modelData, index);
                            }
                        }
                    }
                }
            }
        }
    }
}
