import QtQuick
import QtQuick.Layouts
import "."

Rectangle {
    id: root
    height: Theme.geometry.statusBarHeight
    color: Theme.color.surface

    Rectangle {
        anchors.top: parent.top
        anchors.left: parent.left
        anchors.right: parent.right
        height: 1
        color: Theme.color.border
    }

    RowLayout {
        anchors.fill: parent
        anchors.leftMargin: 12
        anchors.rightMargin: 12

        // Left: Item counts & storage info
        Text {
            text: ViewState.totalItems + " Photos • " +
                  (ViewState.selectedCount > 0 ? (ViewState.selectedCount + " Selected • ") : "") +
                  ViewState.totalBytesFormatted + " • " + ViewState.libraryPath
            font.family: Theme.font.family
            font.pixelSize: Theme.font.caption
            color: Theme.color.dimText
        }

        Item { Layout.fillWidth: true }

        // Center: Keyboard hints
        Text {
            text: "←/→/↑/↓: Move   •   Ctrl+A: Select All   •   F: Favorite   •   I: Info   •   Ctrl+O: Open in Strata"
            font.family: Theme.font.family
            font.pixelSize: Theme.font.caption
            color: Theme.color.dimText
        }

        Item { Layout.fillWidth: true }

        // Right: Active Omarchy theme
        RowLayout {
            spacing: 6

            Rectangle {
                width: 7
                height: 7
                radius: 3.5
                color: Theme.color.accent
            }

            Text {
                text: "Omarchy Quattro: " + Theme.themeName
                font.family: Theme.font.family
                font.pixelSize: Theme.font.caption
                font.bold: true
                color: Theme.color.dimText
            }
        }
    }
}
