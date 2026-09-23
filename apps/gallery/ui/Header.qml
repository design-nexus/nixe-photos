import QtQuick
import QtQuick.Layouts
import QtQuick.Controls
import "."

Rectangle {
    id: root
    height: Theme.geometry.headerHeight
    color: Theme.color.surface

    Rectangle {
        anchors.bottom: parent.bottom
        anchors.left: parent.left
        anchors.right: parent.right
        height: 1
        color: Theme.color.border
    }

    RowLayout {
        anchors.fill: parent
        anchors.leftMargin: 12
        anchors.rightMargin: 12
        spacing: 12

        // Left: Sidebar toggle & Title
        RowLayout {
            spacing: 8
            Layout.alignment: Qt.AlignVCenter

            Rectangle {
                width: 28
                height: 28
                radius: Theme.geometry.pillRadius
                color: sidebarHover.hovered ? Theme.color.selection : "transparent"

                Text {
                    anchors.centerIn: parent
                    text: "≡"
                    font.pixelSize: 16
                    font.bold: true
                    color: Theme.color.foreground
                }

                HoverHandler { id: sidebarHover }
                TapHandler { onTapped: ViewState.toggleSidebar() }
            }

            Rectangle {
                width: 10
                height: 10
                radius: 5
                color: Theme.color.accent
            }

            Text {
                text: "Nexus Photos"
                font.family: Theme.font.family
                font.pixelSize: Theme.font.title
                font.bold: true
                color: Theme.color.foreground
            }
        }

        Item { Layout.fillWidth: true }

        // Center: Segmented Timeline Mode Switcher
        Rectangle {
            height: 28
            width: modeRow.implicitWidth + 8
            radius: Theme.geometry.pillRadius
            color: Theme.color.background
            border.color: Theme.color.border
            border.width: 1
            Layout.alignment: Qt.AlignVCenter

            RowLayout {
                id: modeRow
                anchors.centerIn: parent
                spacing: 2

                Repeater {
                    model: [
                        { id: "years", label: "Years" },
                        { id: "months", label: "Months" },
                        { id: "days", label: "Days" },
                        { id: "all", label: "All Photos" }
                    ]

                    Rectangle {
                        property bool active: ViewState.viewMode === modelData.id
                        width: labelText.implicitWidth + 16
                        height: 22
                        radius: Theme.geometry.radiusSmall
                        color: active ? Theme.color.selection : (segHover.hovered ? Theme.color.surfaceElevated : "transparent")

                        Text {
                            id: labelText
                            anchors.centerIn: parent
                            text: modelData.label
                            font.family: Theme.font.family
                            font.pixelSize: Theme.font.bodySmall
                            font.bold: active
                            color: active ? Theme.color.accent : Theme.color.dimText
                        }

                        HoverHandler { id: segHover }
                        TapHandler { onTapped: ViewState.viewMode = modelData.id }
                    }
                }
            }
        }

        Item { Layout.fillWidth: true }

        // Right: Search, Zoom, Info toggle & Theme
        RowLayout {
            spacing: 10
            Layout.alignment: Qt.AlignVCenter

            // Quick Search Pill
            Rectangle {
                width: 130
                height: 26
                radius: Theme.geometry.pillRadius
                color: Theme.color.background
                border.color: searchHover.hovered ? Theme.color.accent : Theme.color.border
                border.width: 1

                RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: 8
                    anchors.rightMargin: 6
                    spacing: 4

                    Text {
                        text: "🔍"
                        font.pixelSize: 10
                        color: Theme.color.dimText
                    }

                    Text {
                        text: "Search"
                        font.family: Theme.font.family
                        font.pixelSize: Theme.font.bodySmall
                        color: Theme.color.dimText
                        Layout.fillWidth: true
                    }

                    Rectangle {
                        width: 38
                        height: 16
                        radius: 3
                        color: Theme.color.surfaceElevated

                        Text {
                            anchors.centerIn: parent
                            text: "Ctrl+K"
                            font.family: "monospace"
                            font.pixelSize: 9
                            color: Theme.color.dimText
                        }
                    }
                }

                HoverHandler { id: searchHover }
                TapHandler { onTapped: ViewState.searchActive = true }
            }

            // Thumbnail Zoom control
            RowLayout {
                spacing: 4

                Text {
                    text: "🖽"
                    font.pixelSize: 11
                    color: Theme.color.dimText
                }

                Slider {
                    id: zoomSlider
                    from: 0.0
                    to: 1.0
                    value: ViewState.zoomLevel
                    onMoved: ViewState.zoomLevel = value
                    implicitWidth: 70
                    implicitHeight: 20

                    background: Rectangle {
                        x: zoomSlider.leftPadding
                        y: zoomSlider.topPadding + zoomSlider.availableHeight / 2 - height / 2
                        implicitWidth: 70
                        implicitHeight: 3
                        width: zoomSlider.availableWidth
                        height: implicitHeight
                        radius: 2
                        color: Theme.color.muted

                        Rectangle {
                            width: zoomSlider.visualPosition * parent.width
                            height: parent.height
                            color: Theme.color.accent
                            radius: 2
                        }
                    }

                    handle: Rectangle {
                        x: zoomSlider.leftPadding + zoomSlider.visualPosition * (zoomSlider.availableWidth - width)
                        y: zoomSlider.topPadding + zoomSlider.availableHeight / 2 - height / 2
                        implicitWidth: 10
                        implicitHeight: 10
                        radius: 5
                        color: zoomSlider.pressed ? Theme.color.accent : Theme.color.foreground
                    }
                }

                Text {
                    text: "🖽"
                    font.pixelSize: 15
                    color: Theme.color.dimText
                }
            }

            // Inspector Toggle Button
            Rectangle {
                width: 28
                height: 28
                radius: Theme.geometry.pillRadius
                color: ViewState.inspectorOpen ? Theme.color.selection : (infoHover.hovered ? Theme.color.surfaceElevated : "transparent")
                border.color: ViewState.inspectorOpen ? Theme.color.accent : "transparent"
                border.width: 1

                Text {
                    anchors.centerIn: parent
                    text: "ℹ"
                    font.pixelSize: 14
                    font.bold: true
                    color: ViewState.inspectorOpen ? Theme.color.accent : Theme.color.foreground
                }

                HoverHandler { id: infoHover }
                TapHandler { onTapped: ViewState.toggleInspector() }
            }

            // Theme Badge
            Rectangle {
                height: 22
                width: themeBadgeText.implicitWidth + 14
                radius: Theme.geometry.pillRadius
                color: Theme.color.surfaceElevated
                border.color: Theme.color.border
                border.width: 1

                RowLayout {
                    anchors.centerIn: parent
                    spacing: 4

                    Rectangle {
                        width: 6
                        height: 6
                        radius: 3
                        color: Theme.color.accent
                    }

                    Text {
                        id: themeBadgeText
                        text: Theme.themeName
                        font.family: Theme.font.family
                        font.pixelSize: Theme.font.caption
                        color: Theme.color.dimText
                    }
                }
            }
        }
    }
}
