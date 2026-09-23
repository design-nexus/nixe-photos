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
                    text: ViewState.displayedPhotos.length + " photos" +
                          (ViewState.selectedCount > 1 ? (" • " + ViewState.selectedCount + " selected") : "") +
                          " • " + ViewState.libraryPath
                    font.family: Theme.font.family
                    font.pixelSize: Theme.font.caption
                    color: Theme.color.dimText
                }
            }

            Item { Layout.fillWidth: true }

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
                spacing: 10

                Text {
                    text: "📷"
                    font.pixelSize: 48
                    Layout.alignment: Qt.AlignHCenter
                }

                Text {
                    text: "No photos found"
                    font.family: Theme.font.family
                    font.pixelSize: Theme.font.title
                    font.bold: true
                    color: Theme.color.foreground
                    Layout.alignment: Qt.AlignHCenter
                }

                Text {
                    text: "Add images to " + ViewState.libraryPath + " or click Rescan."
                    font.family: Theme.font.family
                    font.pixelSize: Theme.font.bodySmall
                    color: Theme.color.dimText
                    Layout.alignment: Qt.AlignHCenter
                }
            }
        }

        // Grouped Timeline Grid
        ScrollView {
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
                                    property real baseCardSize: 110 + ViewState.zoomLevel * 150
                                    width: baseCardSize
                                    height: baseCardSize + 24
                                    radius: Theme.geometry.radiusMedium
                                    color: Theme.color.surface
                                    border.color: isSelected ? Theme.color.accent : (cardHover.hovered ? Theme.color.selection : Theme.color.border)
                                    border.width: isSelected ? 2 : 1
                                    clip: true

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
                                            Text {
                                                anchors.centerIn: parent
                                                visible: thumbImg.status !== Image.Ready
                                                text: "📷"
                                                font.pixelSize: 24
                                                color: Theme.color.dimText
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

                                    // Top-Left Multi-Selection Checkmark Badge
                                    Rectangle {
                                        anchors.top: parent.top
                                        anchors.left: parent.left
                                        anchors.margins: 6
                                        width: 22
                                        height: 22
                                        radius: 11
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

                                        TapHandler {
                                            onTapped: ViewState.toggleSelect(photoData)
                                        }
                                    }

                                    // Top-Right Favorite Star Button
                                    Rectangle {
                                        anchors.top: parent.top
                                        anchors.right: parent.right
                                        anchors.margins: 6
                                        width: 22
                                        height: 22
                                        radius: 11
                                        color: photoData.is_favorite ? Theme.color.accent : (starHover.hovered ? Theme.color.surfaceElevated : Qt.rgba(0, 0, 0, 0.4))
                                        visible: photoData.is_favorite || cardHover.hovered

                                        NexusIcon {
                                            anchors.centerIn: parent
                                            name: "star"
                                            size: 11
                                            filled: photoData.is_favorite
                                            color: photoData.is_favorite ? Theme.color.background : (starHover.hovered ? Theme.color.accent : Theme.color.foreground)
                                        }

                                        HoverHandler { id: starHover }
                                        TapHandler {
                                            onTapped: Backend.toggleFavorite(photoData.id)
                                        }
                                    }

                                    HoverHandler { id: cardHover }
                                    MouseArea {
                                        anchors.fill: parent
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
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
