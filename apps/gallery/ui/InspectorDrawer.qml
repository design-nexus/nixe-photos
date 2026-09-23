import QtQuick
import QtQuick.Layouts
import QtQuick.Controls
import "."

Rectangle {
    id: root
    width: ViewState.inspectorOpen ? Theme.geometry.inspectorWidth : 0
    visible: width > 0
    color: Theme.color.surface
    clip: true

    Behavior on width {
        NumberAnimation { duration: 150; easing.type: Easing.OutQuad }
    }

    Rectangle {
        anchors.top: parent.top
        anchors.bottom: parent.bottom
        anchors.left: parent.left
        width: 1
        color: Theme.color.border
    }

    ScrollView {
        anchors.fill: parent
        contentWidth: availableWidth
        clip: true

        ColumnLayout {
            width: parent.width - 24
            anchors.horizontalCenter: parent.horizontalCenter
            anchors.top: parent.top
            anchors.margins: 12
            spacing: 14

            // Drawer Header
            RowLayout {
                Layout.fillWidth: true

                Text {
                    text: "Photo Info"
                    font.family: Theme.font.family
                    font.pixelSize: Theme.font.title
                    font.bold: true
                    color: Theme.color.foreground
                    Layout.fillWidth: true
                }

                Rectangle {
                    width: 22
                    height: 22
                    radius: 11
                    color: closeHover.hovered ? Theme.color.selection : "transparent"

                    Text {
                        anchors.centerIn: parent
                        text: "✕"
                        font.pixelSize: 11
                        color: Theme.color.dimText
                    }

                    HoverHandler { id: closeHover }
                    TapHandler { onTapped: ViewState.toggleInspector() }
                }
            }

            // High-res Image Preview Box (Clickable for Fullscreen Lightbox)
            Rectangle {
                Layout.fillWidth: true
                height: 160
                radius: Theme.geometry.radiusMedium
                color: Theme.color.background
                border.color: prevBoxHover.hovered ? Theme.color.accent : Theme.color.border
                border.width: 1
                clip: true

                Image {
                    id: previewImg
                    anchors.fill: parent
                    anchors.margins: 4
                    visible: ViewState.selectedItem !== null
                    source: ViewState.selectedItem ? "file://" + ViewState.selectedItem.path : ""
                    fillMode: Image.PreserveAspectFit
                    rotation: ViewState.getRotation(ViewState.selectedItem ? ViewState.selectedItem.id : "")
                    asynchronous: true

                    Behavior on rotation {
                        NumberAnimation { duration: 150 }
                    }
                }

                // Hover overlay hint
                Rectangle {
                    anchors.fill: parent
                    color: Qt.rgba(0, 0, 0, 0.4)
                    visible: prevBoxHover.hovered && ViewState.selectedItem !== null

                    RowLayout {
                        anchors.centerIn: parent
                        spacing: 6
                        Text { text: "🔍"; font.pixelSize: 12 }
                        Text {
                            text: "Fullscreen (Space)"
                            font.family: Theme.font.family
                            font.pixelSize: Theme.font.bodySmall
                            font.bold: true
                            color: "#ffffff"
                        }
                    }
                }

                ColumnLayout {
                    anchors.centerIn: parent
                    visible: ViewState.selectedItem === null
                    spacing: 6

                    Text {
                        text: "📷"
                        font.pixelSize: 32
                        Layout.alignment: Qt.AlignHCenter
                    }

                    Text {
                        text: "Select a photo to inspect"
                        font.family: Theme.font.family
                        font.pixelSize: Theme.font.bodySmall
                        color: Theme.color.dimText
                        Layout.alignment: Qt.AlignHCenter
                    }
                }

                HoverHandler { id: prevBoxHover }
                TapHandler {
                    onTapped: {
                        if (ViewState.selectedItem) {
                            ViewState.openLightbox();
                        }
                    }
                }
            }

            // macOS Photos Exposure Histogram Representation
            ColumnLayout {
                Layout.fillWidth: true
                spacing: 6
                visible: ViewState.selectedItem !== null

                Text {
                    text: "HISTOGRAM"
                    font.family: Theme.font.family
                    font.pixelSize: Theme.font.caption
                    font.bold: true
                    color: Theme.color.dimText
                }

                Rectangle {
                    Layout.fillWidth: true
                    height: 52
                    radius: Theme.geometry.radiusMedium
                    color: Theme.color.background
                    border.color: Theme.color.border
                    border.width: 1

                    RowLayout {
                        anchors.fill: parent
                        anchors.margins: 6
                        spacing: 2

                        Repeater {
                            model: 24

                            Rectangle {
                                Layout.fillHeight: true
                                Layout.fillWidth: true
                                radius: 1

                                property real barHeight: {
                                    // Generate realistic histogram distribution based on index
                                    var x = index / 23.0;
                                    var curve = Math.sin(x * Math.PI) * 0.7 + Math.sin(x * 3 * Math.PI) * 0.2 + 0.15;
                                    return Math.max(0.1, Math.min(1.0, curve));
                                }

                                height: parent.height * barHeight
                                Layout.alignment: Qt.AlignBottom
                                color: {
                                    if (index < 6) return Qt.rgba(0.9, 0.4, 0.4, 0.75); // Shadows (Red/Warm)
                                    if (index < 18) return Qt.rgba(0.4, 0.8, 0.5, 0.75); // Midtones (Green)
                                    return Qt.rgba(0.4, 0.6, 0.95, 0.75); // Highlights (Blue)
                                }
                            }
                        }
                    }
                }
            }

            // File Details Section
            ColumnLayout {
                Layout.fillWidth: true
                spacing: 8

                Text {
                    text: "FILE DETAILS"
                    font.family: Theme.font.family
                    font.pixelSize: Theme.font.caption
                    font.bold: true
                    color: Theme.color.dimText
                }

                Rectangle {
                    Layout.fillWidth: true
                    height: fileCol.implicitHeight + 16
                    radius: Theme.geometry.radiusMedium
                    color: Theme.color.background
                    border.color: Theme.color.border
                    border.width: 1

                    ColumnLayout {
                        id: fileCol
                        anchors.fill: parent
                        anchors.margins: 10
                        spacing: 6

                        RowLayout {
                            Layout.fillWidth: true
                            Text { text: "File name"; font.pixelSize: Theme.font.bodySmall; color: Theme.color.dimText }
                            Text {
                                text: ViewState.selectedItem ? ViewState.selectedItem.file_name : "-"
                                font.pixelSize: Theme.font.bodySmall
                                font.bold: true
                                color: Theme.color.foreground
                                elide: Text.ElideMiddle
                                Layout.fillWidth: true
                                horizontalAlignment: Text.AlignRight
                            }
                        }

                        RowLayout {
                            Layout.fillWidth: true
                            Text { text: "Dimensions"; font.pixelSize: Theme.font.bodySmall; color: Theme.color.dimText; Layout.fillWidth: true }
                            Text {
                                text: ViewState.selectedItem ? (ViewState.selectedItem.width + " × " + ViewState.selectedItem.height) : "-"
                                font.pixelSize: Theme.font.bodySmall
                                color: Theme.color.foreground
                            }
                        }

                        RowLayout {
                            Layout.fillWidth: true
                            Text { text: "File size"; font.pixelSize: Theme.font.bodySmall; color: Theme.color.dimText; Layout.fillWidth: true }
                            Text {
                                text: ViewState.selectedItem ? ViewState.selectedItem.file_size_formatted : "-"
                                font.pixelSize: Theme.font.bodySmall
                                color: Theme.color.foreground
                            }
                        }

                        RowLayout {
                            Layout.fillWidth: true
                            Text { text: "Format"; font.pixelSize: Theme.font.bodySmall; color: Theme.color.dimText; Layout.fillWidth: true }
                            Text {
                                text: ViewState.selectedItem ? ViewState.selectedItem.format : "-"
                                font.pixelSize: Theme.font.bodySmall
                                color: Theme.color.foreground
                            }
                        }

                        RowLayout {
                            Layout.fillWidth: true
                            Text { text: "Date"; font.pixelSize: Theme.font.bodySmall; color: Theme.color.dimText; Layout.fillWidth: true }
                            Text {
                                text: ViewState.selectedItem ? ViewState.selectedItem.date_display : "-"
                                font.pixelSize: Theme.font.bodySmall
                                color: Theme.color.foreground
                            }
                        }
                    }
                }
            }

            // Camera & Optics (EXIF)
            ColumnLayout {
                Layout.fillWidth: true
                spacing: 8

                Text {
                    text: "CAMERA & OPTICS"
                    font.family: Theme.font.family
                    font.pixelSize: Theme.font.caption
                    font.bold: true
                    color: Theme.color.dimText
                }

                Rectangle {
                    Layout.fillWidth: true
                    height: exifCol.implicitHeight + 16
                    radius: Theme.geometry.radiusMedium
                    color: Theme.color.background
                    border.color: Theme.color.border
                    border.width: 1

                    ColumnLayout {
                        id: exifCol
                        anchors.fill: parent
                        anchors.margins: 10
                        spacing: 6

                        RowLayout {
                            Layout.fillWidth: true
                            Text { text: "Device"; font.pixelSize: Theme.font.bodySmall; color: Theme.color.dimText }
                            Text {
                                text: (ViewState.selectedItem && ViewState.selectedItem.camera_model) ?
                                      ViewState.selectedItem.camera_model : "Digital Screenshot / Screen Capture"
                                font.pixelSize: Theme.font.bodySmall
                                font.bold: true
                                color: Theme.color.foreground
                                elide: Text.ElideRight
                                Layout.fillWidth: true
                                horizontalAlignment: Text.AlignRight
                            }
                        }

                        RowLayout {
                            visible: ViewState.selectedItem && ViewState.selectedItem.lens_model !== null
                            Layout.fillWidth: true
                            Text { text: "Lens"; font.pixelSize: Theme.font.bodySmall; color: Theme.color.dimText }
                            Text {
                                text: ViewState.selectedItem && ViewState.selectedItem.lens_model ? ViewState.selectedItem.lens_model : ""
                                font.pixelSize: Theme.font.bodySmall
                                color: Theme.color.foreground
                                elide: Text.ElideRight
                                Layout.fillWidth: true
                                horizontalAlignment: Text.AlignRight
                            }
                        }

                        // Exposure badges row
                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 4

                            Rectangle {
                                visible: ViewState.selectedItem && ViewState.selectedItem.focal_length !== null
                                height: 20
                                width: exp1.implicitWidth + 8
                                radius: 4
                                color: Theme.color.surfaceElevated
                                Text { id: exp1; anchors.centerIn: parent; text: ViewState.selectedItem && ViewState.selectedItem.focal_length ? ViewState.selectedItem.focal_length : ""; font.pixelSize: 9; color: Theme.color.dimText }
                            }
                            Rectangle {
                                visible: ViewState.selectedItem && ViewState.selectedItem.aperture !== null
                                height: 20
                                width: exp2.implicitWidth + 8
                                radius: 4
                                color: Theme.color.surfaceElevated
                                Text { id: exp2; anchors.centerIn: parent; text: ViewState.selectedItem && ViewState.selectedItem.aperture ? ViewState.selectedItem.aperture : ""; font.pixelSize: 9; color: Theme.color.dimText }
                            }
                            Rectangle {
                                visible: ViewState.selectedItem && ViewState.selectedItem.shutter_speed !== null
                                height: 20
                                width: exp3.implicitWidth + 8
                                radius: 4
                                color: Theme.color.surfaceElevated
                                Text { id: exp3; anchors.centerIn: parent; text: ViewState.selectedItem && ViewState.selectedItem.shutter_speed ? ViewState.selectedItem.shutter_speed : ""; font.pixelSize: 9; color: Theme.color.dimText }
                            }
                            Rectangle {
                                visible: ViewState.selectedItem && ViewState.selectedItem.iso !== null
                                height: 20
                                width: exp4.implicitWidth + 8
                                radius: 4
                                color: Theme.color.surfaceElevated
                                Text { id: exp4; anchors.centerIn: parent; text: ViewState.selectedItem && ViewState.selectedItem.iso ? ViewState.selectedItem.iso : ""; font.pixelSize: 9; color: Theme.color.dimText }
                            }

                            Text {
                                visible: !ViewState.selectedItem || (!ViewState.selectedItem.focal_length && !ViewState.selectedItem.aperture && !ViewState.selectedItem.shutter_speed)
                                text: "No optical exposure metadata"
                                font.pixelSize: Theme.font.caption
                                color: Theme.color.dimText
                            }
                        }
                    }
                }
            }

            // Actions: Rotate, Copy Path, Reveal in Strata & Favorite
            ColumnLayout {
                Layout.fillWidth: true
                spacing: 8

                // Rotate & Copy Path Row
                RowLayout {
                    Layout.fillWidth: true
                    spacing: 8

                    Rectangle {
                        Layout.fillWidth: true
                        height: 30
                        radius: Theme.geometry.pillRadius
                        color: rotHover.hovered ? Theme.color.selection : Theme.color.surfaceElevated
                        border.color: Theme.color.border
                        border.width: 1

                        RowLayout {
                            anchors.centerIn: parent
                            spacing: 4
                            Text { text: "↻"; font.pixelSize: 12 }
                            Text { text: "Rotate (R)"; font.pixelSize: Theme.font.bodySmall; color: Theme.color.foreground }
                        }

                        HoverHandler { id: rotHover }
                        TapHandler { onTapped: ViewState.rotateCurrent() }
                    }

                    Rectangle {
                        Layout.fillWidth: true
                        height: 30
                        radius: Theme.geometry.pillRadius
                        color: copyHover.hovered ? Theme.color.selection : Theme.color.surfaceElevated
                        border.color: Theme.color.border
                        border.width: 1

                        RowLayout {
                            anchors.centerIn: parent
                            spacing: 4
                            Text { text: "📋"; font.pixelSize: 11 }
                            Text { text: "Copy Path"; font.pixelSize: Theme.font.bodySmall; color: Theme.color.foreground }
                        }

                        HoverHandler { id: copyHover }
                        TapHandler { onTapped: ViewState.copyCurrentPath() }
                    }
                }

                // Reveal in Strata Button
                Rectangle {
                    Layout.fillWidth: true
                    height: 32
                    radius: Theme.geometry.pillRadius
                    color: revealHover.hovered ? Theme.color.selection : Theme.color.surfaceElevated
                    border.color: Theme.color.border
                    border.width: 1

                    RowLayout {
                        anchors.centerIn: parent
                        spacing: 6

                        Text { text: "📂"; font.pixelSize: 12 }
                        Text {
                            text: "Reveal in Strata (Ctrl+O)"
                            font.family: Theme.font.family
                            font.pixelSize: Theme.font.bodySmall
                            font.bold: true
                            color: Theme.color.foreground
                        }
                    }

                    HoverHandler { id: revealHover }
                    TapHandler {
                        onTapped: {
                            if (ViewState.selectedItem) {
                                Backend.revealInStrata(ViewState.selectedItem.path);
                            } else {
                                Backend.revealInStrata(ViewState.libraryPath);
                            }
                        }
                    }
                }

                // Favorite Button
                Rectangle {
                    Layout.fillWidth: true
                    height: 32
                    radius: Theme.geometry.pillRadius
                    color: favHover.hovered ? Theme.color.selection : Theme.color.surfaceElevated
                    border.color: Theme.color.border
                    border.width: 1

                    RowLayout {
                        anchors.centerIn: parent
                        spacing: 6

                        Text {
                            text: ViewState.selectedItem && ViewState.selectedItem.is_favorite ? "★" : "☆"
                            font.pixelSize: 13
                            color: ViewState.selectedItem && ViewState.selectedItem.is_favorite ? Theme.color.accent : Theme.color.dimText
                        }
                        Text {
                            text: ViewState.selectedItem && ViewState.selectedItem.is_favorite ? "Remove from Favorites" : "Add to Favorites (F)"
                            font.family: Theme.font.family
                            font.pixelSize: Theme.font.bodySmall
                            font.bold: true
                            color: Theme.color.foreground
                        }
                    }

                    HoverHandler { id: favHover }
                    TapHandler {
                        onTapped: {
                            if (ViewState.selectedItem) {
                                Backend.toggleFavorite(ViewState.selectedItem.id);
                            }
                        }
                    }
                }
            }
        }
    }
}
