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

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 14
        spacing: 14

        // Drawer Header
        RowLayout {
            Layout.fillWidth: true

            Text {
                text: "Info"
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

        // Preview Box Placeholder
        Rectangle {
            Layout.fillWidth: true
            height: 140
            radius: Theme.geometry.radiusMedium
            color: Theme.color.background
            border.color: Theme.color.border
            border.width: 1

            ColumnLayout {
                anchors.centerIn: parent
                spacing: 6

                Text {
                    text: "📷"
                    font.pixelSize: 32
                    Layout.alignment: Qt.AlignHCenter
                }

                Text {
                    text: ViewState.selectedItem ? ViewState.selectedItem.name : "Select a photo"
                    font.family: Theme.font.family
                    font.pixelSize: Theme.font.bodySmall
                    color: Theme.color.dimText
                    Layout.alignment: Qt.AlignHCenter
                }
            }
        }

        // Metadata section
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
                        Text { text: "Library"; font.pixelSize: Theme.font.bodySmall; color: Theme.color.dimText; Layout.fillWidth: true }
                        Text { text: ViewState.libraryPath; font.pixelSize: Theme.font.bodySmall; font.bold: true; color: Theme.color.foreground; elide: Text.ElideMiddle; Layout.maximumWidth: 150 }
                    }

                    RowLayout {
                        Layout.fillWidth: true
                        Text { text: "Dimensions"; font.pixelSize: Theme.font.bodySmall; color: Theme.color.dimText; Layout.fillWidth: true }
                        Text { text: "3840 × 2160"; font.pixelSize: Theme.font.bodySmall; color: Theme.color.foreground }
                    }

                    RowLayout {
                        Layout.fillWidth: true
                        Text { text: "File size"; font.pixelSize: Theme.font.bodySmall; color: Theme.color.dimText; Layout.fillWidth: true }
                        Text { text: "4.2 MB"; font.pixelSize: Theme.font.bodySmall; color: Theme.color.foreground }
                    }

                    RowLayout {
                        Layout.fillWidth: true
                        Text { text: "Format"; font.pixelSize: Theme.font.bodySmall; color: Theme.color.dimText; Layout.fillWidth: true }
                        Text { text: "JPEG Image"; font.pixelSize: Theme.font.bodySmall; color: Theme.color.foreground }
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
                        Text { text: "Camera"; font.pixelSize: Theme.font.bodySmall; color: Theme.color.dimText; Layout.fillWidth: true }
                        Text { text: "Sony Alpha 7 IV"; font.pixelSize: Theme.font.bodySmall; font.bold: true; color: Theme.color.foreground }
                    }

                    RowLayout {
                        Layout.fillWidth: true
                        Text { text: "Lens"; font.pixelSize: Theme.font.bodySmall; color: Theme.color.dimText; Layout.fillWidth: true }
                        Text { text: "FE 24-70mm F2.8 GM"; font.pixelSize: Theme.font.bodySmall; color: Theme.color.foreground }
                    }

                    // Exposure pill tags
                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 4

                        Rectangle {
                            height: 20
                            width: exp1.implicitWidth + 8
                            radius: 4
                            color: Theme.color.surfaceElevated
                            Text { id: exp1; anchors.centerIn: parent; text: "35mm"; font.pixelSize: 9; color: Theme.color.dimText }
                        }
                        Rectangle {
                            height: 20
                            width: exp2.implicitWidth + 8
                            radius: 4
                            color: Theme.color.surfaceElevated
                            Text { id: exp2; anchors.centerIn: parent; text: "f/2.8"; font.pixelSize: 9; color: Theme.color.dimText }
                        }
                        Rectangle {
                            height: 20
                            width: exp3.implicitWidth + 8
                            radius: 4
                            color: Theme.color.surfaceElevated
                            Text { id: exp3; anchors.centerIn: parent; text: "1/250s"; font.pixelSize: 9; color: Theme.color.dimText }
                        }
                        Rectangle {
                            height: 20
                            width: exp4.implicitWidth + 8
                            radius: 4
                            color: Theme.color.surfaceElevated
                            Text { id: exp4; anchors.centerIn: parent; text: "ISO 100"; font.pixelSize: 9; color: Theme.color.dimText }
                        }
                    }
                }
            }
        }

        Item { Layout.fillHeight: true }

        // Actions
        ColumnLayout {
            Layout.fillWidth: true
            spacing: 8

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
                        text: "Reveal in Strata"
                        font.family: Theme.font.family
                        font.pixelSize: Theme.font.bodySmall
                        font.bold: true
                        color: Theme.color.foreground
                    }
                }

                HoverHandler { id: revealHover }
                TapHandler {
                    onTapped: {
                        Backend.revealInStrata(ViewState.libraryPath);
                    }
                }
            }
        }
    }
}
