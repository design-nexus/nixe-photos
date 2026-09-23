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

        // Section Title & Breadcrumbs
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
                    text: ViewState.libraryPath + " • Updated just now"
                    font.family: Theme.font.family
                    font.pixelSize: Theme.font.caption
                    color: Theme.color.dimText
                }
            }

            Item { Layout.fillWidth: true }

            // Filter button or sort badge
            Rectangle {
                height: 26
                width: sortText.implicitWidth + 20
                radius: Theme.geometry.pillRadius
                color: Theme.color.surface
                border.color: Theme.color.border
                border.width: 1

                RowLayout {
                    anchors.centerIn: parent
                    spacing: 6
                    Text { text: "↕"; font.pixelSize: 11; color: Theme.color.dimText }
                    Text {
                        id: sortText
                        text: "Newest First"
                        font.family: Theme.font.family
                        font.pixelSize: Theme.font.bodySmall
                        color: Theme.color.foreground
                    }
                }
            }
        }

        // Timeline Date Group Header
        RowLayout {
            Layout.fillWidth: true
            spacing: 8

            Text {
                text: "September 22, 2026"
                font.family: Theme.font.family
                font.pixelSize: Theme.font.body
                font.bold: true
                color: Theme.color.foreground
            }

            Rectangle {
                height: 1
                Layout.fillWidth: true
                color: Theme.color.border
            }

            Text {
                text: "Stage 1 Preview"
                font.family: Theme.font.family
                font.pixelSize: Theme.font.caption
                color: Theme.color.dimText
            }
        }

        // Demo Photos Grid (Scalable with Zoom Slider)
        ScrollView {
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true

            Flow {
                width: parent.width
                spacing: 12

                Repeater {
                    model: 8

                    Rectangle {
                        property real baseCardSize: 140 + ViewState.zoomLevel * 120
                        width: baseCardSize
                        height: baseCardSize
                        radius: Theme.geometry.radiusMedium
                        color: Theme.color.surface
                        border.color: cardHover.hovered ? Theme.color.accent : Theme.color.border
                        border.width: cardHover.hovered ? 2 : 1

                        Behavior on width {
                            NumberAnimation { duration: 100 }
                        }
                        Behavior on height {
                            NumberAnimation { duration: 100 }
                        }

                        ColumnLayout {
                            anchors.centerIn: parent
                            spacing: 8

                            Rectangle {
                                width: 42
                                height: 42
                                radius: 8
                                color: Theme.color.surfaceElevated
                                Layout.alignment: Qt.AlignHCenter

                                Text {
                                    anchors.centerIn: parent
                                    text: ["🌄", "🌇", "🏕", "🏖", "🏙", "🌉", "⛵", "📷"][index % 8]
                                    font.pixelSize: 22
                                }
                            }

                            Text {
                                text: "IMG_" + (1001 + index) + ".JPG"
                                font.family: Theme.font.family
                                font.pixelSize: Theme.font.caption
                                color: Theme.color.dimText
                                Layout.alignment: Qt.AlignHCenter
                            }
                        }

                        // Favorite star badge
                        Rectangle {
                            anchors.top: parent.top
                            anchors.right: parent.right
                            anchors.margins: 6
                            width: 22
                            height: 22
                            radius: 11
                            color: Theme.color.surfaceElevated
                            visible: index % 3 === 0

                            Text {
                                anchors.centerIn: parent
                                text: "★"
                                font.pixelSize: 11
                                color: Theme.color.accent
                            }
                        }

                        HoverHandler { id: cardHover }
                        TapHandler {
                            onTapped: {
                                ViewState.selectedItem = {
                                    name: "IMG_" + (1001 + index) + ".JPG",
                                    index: index
                                };
                            }
                        }
                    }
                }
            }
        }
    }
}
