import QtQuick
import QtQuick.Layouts
import QtQuick.Controls
import "."

Rectangle {
    id: root
    visible: ViewState.lightboxOpen
    color: Qt.rgba(Theme.color.background.r, Theme.color.background.g, Theme.color.background.b, 0.96)
    z: 200

    FocusScope {
        anchors.fill: parent
        focus: root.visible

        Keys.onPressed: function(event) {
            if (event.key === Qt.Key_Escape || event.key === Qt.Key_Space) {
                ViewState.closeLightbox();
                event.accepted = true;
            } else if (event.key === Qt.Key_Left) {
                ViewState.prevPhoto();
                event.accepted = true;
            } else if (event.key === Qt.Key_Right) {
                ViewState.nextPhoto();
                event.accepted = true;
            } else if (event.key === Qt.Key_R) {
                ViewState.rotateCurrent();
                event.accepted = true;
            } else if (event.key === Qt.Key_F || event.key === Qt.Key_Period) {
                ViewState.toggleSelectedFavorite();
                event.accepted = true;
            } else if (event.key === Qt.Key_O && (event.modifiers & Qt.ControlModifier)) {
                if (ViewState.selectedItem) {
                    Backend.revealInStrata(ViewState.selectedItem.path);
                }
                event.accepted = true;
            }
        }

        // Top Floating Chrome Bar
        Rectangle {
            id: topBar
            anchors.top: parent.top
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.margins: 14
            height: 44
            radius: Theme.geometry.radiusLarge
            color: Theme.color.surface
            border.color: Theme.color.border
            border.width: 1
            z: 210

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 12
                anchors.rightMargin: 12
                spacing: 12

                // Back / Close Button
                Rectangle {
                    width: 32
                    height: 32
                    radius: Theme.geometry.pillRadius
                    color: backHover.hovered ? Theme.color.selection : Theme.color.surfaceElevated

                    NexusIcon {
                        anchors.centerIn: parent
                        name: "arrow-left"
                        size: 14
                        color: Theme.color.foreground
                    }

                    HoverHandler { id: backHover }
                    TapHandler { onTapped: ViewState.closeLightbox() }

                    // Hover Tooltip
                    Rectangle {
                        visible: backHover.hovered
                        anchors.top: parent.bottom
                        anchors.topMargin: 8
                        anchors.left: parent.left
                        width: backTipText.implicitWidth + 14
                        height: 24
                        radius: 4
                        color: Theme.color.surfaceElevated
                        border.color: Theme.color.border
                        border.width: 1
                        z: 999

                        Text {
                            id: backTipText
                            anchors.centerIn: parent
                            text: "Back to Gallery (Esc)"
                            font.family: Theme.font.family
                            font.pixelSize: Theme.font.caption
                            color: Theme.color.foreground
                        }
                    }
                }

                // File name & date
                ColumnLayout {
                    spacing: 1
                    Text {
                        text: ViewState.selectedItem ? ViewState.selectedItem.file_name : ""
                        font.family: Theme.font.family
                        font.pixelSize: Theme.font.body
                        font.bold: true
                        color: Theme.color.foreground
                        elide: Text.ElideMiddle
                        Layout.maximumWidth: 320
                    }

                    Text {
                        text: ViewState.selectedItem ? (ViewState.selectedItem.date_display + " • " + ViewState.selectedItem.width + " × " + ViewState.selectedItem.height) : ""
                        font.family: Theme.font.family
                        font.pixelSize: Theme.font.caption
                        color: Theme.color.dimText
                    }
                }

                Item { Layout.fillWidth: true }

                // Center position indicator
                Rectangle {
                    height: 24
                    width: counterText.implicitWidth + 16
                    radius: 12
                    color: Theme.color.background
                    border.color: Theme.color.border
                    border.width: 1

                    Text {
                        id: counterText
                        anchors.centerIn: parent
                        text: (ViewState.selectedIndex + 1) + " of " + ViewState.displayedPhotos.length
                        font.family: Theme.font.family
                        font.pixelSize: Theme.font.caption
                        color: Theme.color.dimText
                    }
                }

                Item { Layout.fillWidth: true }

                // Action Buttons
                RowLayout {
                    spacing: 8

                    // Rotate 90 deg
                    Rectangle {
                        height: 32
                        width: 32
                        radius: Theme.geometry.pillRadius
                        color: rotHover.hovered ? Theme.color.selection : Theme.color.surfaceElevated

                        NexusIcon {
                            anchors.centerIn: parent
                            name: "rotate"
                            size: 14
                            color: Theme.color.foreground
                        }

                        HoverHandler { id: rotHover }
                        TapHandler { onTapped: ViewState.rotateCurrent() }

                        // Hover Tooltip
                        Rectangle {
                            visible: rotHover.hovered
                            anchors.top: parent.bottom
                            anchors.topMargin: 8
                            anchors.horizontalCenter: parent.horizontalCenter
                            width: rotTipText.implicitWidth + 14
                            height: 24
                            radius: 4
                            color: Theme.color.surfaceElevated
                            border.color: Theme.color.border
                            border.width: 1
                            z: 999

                            Text {
                                id: rotTipText
                                anchors.centerIn: parent
                                text: "Rotate 90° (R)"
                                font.family: Theme.font.family
                                font.pixelSize: Theme.font.caption
                                color: Theme.color.foreground
                            }
                        }
                    }

                    // Favorite Button
                    Rectangle {
                        id: favModalBtn
                        height: 32
                        width: 32
                        radius: Theme.geometry.pillRadius
                        property bool isFav: ViewState.selectedItem ? ViewState.isFavorite(ViewState.selectedItem.id) : false
                        color: isFav ? Theme.color.accent : (favHover.hovered ? Theme.color.selection : Theme.color.surfaceElevated)

                        NexusIcon {
                            anchors.centerIn: parent
                            name: "star"
                            size: 14
                            filled: favModalBtn.isFav
                            color: favModalBtn.isFav ? Theme.color.background : Theme.color.foreground
                        }

                        HoverHandler { id: favHover }
                        TapHandler { onTapped: ViewState.toggleSelectedFavorite() }

                        // Hover Tooltip
                        Rectangle {
                            visible: favHover.hovered
                            anchors.top: parent.bottom
                            anchors.topMargin: 8
                            anchors.horizontalCenter: parent.horizontalCenter
                            width: favTipText.implicitWidth + 14
                            height: 24
                            radius: 4
                            color: Theme.color.surfaceElevated
                            border.color: Theme.color.border
                            border.width: 1
                            z: 999

                            Text {
                                id: favTipText
                                anchors.centerIn: parent
                                text: favModalBtn.isFav ? "Remove from Favorites (F)" : "Add to Favorites (F)"
                                font.family: Theme.font.family
                                font.pixelSize: Theme.font.caption
                                color: Theme.color.foreground
                            }
                        }
                    }

                    // Reveal in Strata / File Manager
                    Rectangle {
                        height: 32
                        width: 32
                        radius: Theme.geometry.pillRadius
                        color: revHover.hovered ? Theme.color.selection : Theme.color.surfaceElevated

                        NexusIcon {
                            anchors.centerIn: parent
                            name: "folder"
                            size: 14
                            color: revHover.hovered ? Theme.color.accent : Theme.color.foreground
                        }

                        HoverHandler { id: revHover }
                        TapHandler {
                            onTapped: {
                                if (ViewState.selectedItem) {
                                    Backend.revealInStrata(ViewState.selectedItem.path);
                                }
                            }
                        }

                        // Hover Tooltip
                        Rectangle {
                            visible: revHover.hovered
                            anchors.top: parent.bottom
                            anchors.topMargin: 8
                            anchors.horizontalCenter: parent.horizontalCenter
                            width: revTipText.implicitWidth + 14
                            height: 24
                            radius: 4
                            color: Theme.color.surfaceElevated
                            border.color: Theme.color.border
                            border.width: 1
                            z: 999

                            Text {
                                id: revTipText
                                anchors.centerIn: parent
                                text: "Reveal in File Manager (Ctrl+O)"
                                font.family: Theme.font.family
                                font.pixelSize: Theme.font.caption
                                color: Theme.color.foreground
                            }
                        }
                    }

                    // Close Button
                    Rectangle {
                        height: 32
                        width: 32
                        radius: Theme.geometry.pillRadius
                        color: closeBtnHover.hovered ? Theme.color.danger : Theme.color.surfaceElevated

                        NexusIcon {
                            anchors.centerIn: parent
                            name: "close"
                            size: 12
                            color: closeBtnHover.hovered ? "#ffffff" : Theme.color.foreground
                        }

                        HoverHandler { id: closeBtnHover }
                        TapHandler { onTapped: ViewState.closeLightbox() }

                        // Hover Tooltip
                        Rectangle {
                            visible: closeBtnHover.hovered
                            anchors.top: parent.bottom
                            anchors.topMargin: 8
                            anchors.right: parent.right
                            width: closeTipText.implicitWidth + 14
                            height: 24
                            radius: 4
                            color: Theme.color.surfaceElevated
                            border.color: Theme.color.border
                            border.width: 1
                            z: 999

                            Text {
                                id: closeTipText
                                anchors.centerIn: parent
                                text: "Close Lightbox (Esc)"
                                font.family: Theme.font.family
                                font.pixelSize: Theme.font.caption
                                color: Theme.color.foreground
                            }
                        }
                    }
                }
            }
        }

        // Center High-Res Photo Container
        Item {
            anchors.fill: parent
            anchors.topMargin: 70
            anchors.bottomMargin: 95
            anchors.leftMargin: 60
            anchors.rightMargin: 60

            Image {
                id: mainImage
                anchors.fill: parent
                source: ViewState.selectedItem ? ("file://" + ViewState.selectedItem.path) : ""
                fillMode: Image.PreserveAspectFit
                rotation: ViewState.getRotation(ViewState.selectedItem ? ViewState.selectedItem.id : "")
                asynchronous: true
                mipmap: true

                Behavior on rotation {
                    NumberAnimation { duration: 150; easing.type: Easing.OutQuad }
                }
            }

            // Loading indicator
            Text {
                anchors.centerIn: parent
                visible: mainImage.status === Image.Loading
                text: "Loading high-resolution preview..."
                font.family: Theme.font.family
                font.pixelSize: Theme.font.body
                color: Theme.color.dimText
            }
        }

        // Left Navigation Chevron Button
        Rectangle {
            anchors.left: parent.left
            anchors.verticalCenter: parent.verticalCenter
            anchors.leftMargin: 16
            width: 40
            height: 54
            radius: Theme.geometry.radiusMedium
            color: prevHover.hovered ? Theme.color.selection : Qt.rgba(0, 0, 0, 0.45)
            border.color: Theme.color.border
            border.width: 1
            z: 210

            NexusIcon {
                anchors.centerIn: parent
                name: "arrow-left"
                size: 18
                color: Theme.color.foreground
            }

            HoverHandler { id: prevHover }
            TapHandler { onTapped: ViewState.prevPhoto() }
        }

        // Right Navigation Chevron Button
        Rectangle {
            anchors.right: parent.right
            anchors.verticalCenter: parent.verticalCenter
            anchors.rightMargin: 16
            width: 40
            height: 54
            radius: Theme.geometry.radiusMedium
            color: nextHover.hovered ? Theme.color.selection : Qt.rgba(0, 0, 0, 0.45)
            border.color: Theme.color.border
            border.width: 1
            z: 210

            NexusIcon {
                anchors.centerIn: parent
                name: "arrow-right"
                size: 18
                color: Theme.color.foreground
            }

            HoverHandler { id: nextHover }
            TapHandler { onTapped: ViewState.nextPhoto() }
        }

        // Bottom Filmstrip Scrubber
        Rectangle {
            id: filmstripBar
            anchors.bottom: parent.bottom
            anchors.horizontalCenter: parent.horizontalCenter
            anchors.bottomMargin: 14
            height: 70
            width: Math.min(parent.width - 40, (ViewState.displayedPhotos.length * 56) + 32)
            radius: Theme.geometry.radiusLarge
            color: Theme.color.surface
            border.color: Theme.color.border
            border.width: 1
            z: 210
            clip: true

            ListView {
                id: stripList
                anchors.fill: parent
                anchors.margins: 6
                orientation: ListView.Horizontal
                spacing: 6
                model: ViewState.displayedPhotos
                currentIndex: ViewState.selectedIndex

                delegate: Rectangle {
                    property bool isCur: ViewState.selectedIndex === index
                    width: 50
                    height: 58
                    radius: Theme.geometry.radiusSmall
                    color: Theme.color.background
                    border.color: isCur ? Theme.color.accent : (stripThumbHover.hovered ? Theme.color.selection : "transparent")
                    border.width: isCur ? 2 : 1
                    clip: true

                    Image {
                        anchors.fill: parent
                        anchors.margins: 2
                        source: "file://" + (modelData.thumbnail_path || modelData.path)
                        fillMode: Image.PreserveAspectCrop
                        rotation: ViewState.getRotation(modelData.id)
                        asynchronous: true
                    }

                    HoverHandler { id: stripThumbHover }
                    TapHandler {
                        onTapped: {
                            ViewState.selectSingle(modelData, index);
                        }
                    }
                }
            }
        }
    }
}
