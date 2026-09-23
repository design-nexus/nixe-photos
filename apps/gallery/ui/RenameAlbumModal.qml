import QtQuick
import QtQuick.Layouts
import QtQuick.Controls
import "."

Rectangle {
    id: root
    visible: ViewState.renameModalOpen
    color: Qt.rgba(0, 0, 0, 0.65)
    z: 320

    onVisibleChanged: {
        if (visible) {
            albumInput.text = ViewState.renameInitialName;
            albumInput.forceActiveFocus();
            albumInput.selectAll();
        }
    }

    TapHandler {
        onTapped: ViewState.closeRenameAlbumDialog()
    }

    Rectangle {
        id: dialogCard
        width: 360
        height: 155
        anchors.centerIn: parent
        radius: Theme.geometry.radiusLarge
        color: Theme.color.surface
        border.color: Theme.color.border
        border.width: 1

        TapHandler {
            // Absorb clicks on the card
        }

        ColumnLayout {
            anchors.fill: parent
            anchors.margins: 16
            spacing: 12

            RowLayout {
                Layout.fillWidth: true

                Text {
                    text: "Rename Album"
                    font.family: Theme.font.family
                    font.pixelSize: Theme.font.title
                    font.bold: true
                    color: Theme.color.foreground
                    Layout.fillWidth: true
                }

                Rectangle {
                    width: 24
                    height: 24
                    radius: 12
                    color: closeHover.hovered ? Theme.color.selection : "transparent"

                    NexusIcon {
                        anchors.centerIn: parent
                        name: "close"
                        size: 11
                        color: closeHover.hovered ? Theme.color.foreground : Theme.color.dimText
                    }

                    HoverHandler { id: closeHover }
                    TapHandler { onTapped: ViewState.closeRenameAlbumDialog() }
                }
            }

            Rectangle {
                Layout.fillWidth: true
                height: 34
                radius: Theme.geometry.radiusSmall
                color: Theme.color.background
                border.color: albumInput.activeFocus ? Theme.color.accent : Theme.color.border
                border.width: 1

                TextInput {
                    id: albumInput
                    anchors.fill: parent
                    anchors.leftMargin: 10
                    anchors.rightMargin: 10
                    verticalAlignment: TextInput.AlignVCenter
                    font.family: Theme.font.family
                    font.pixelSize: Theme.font.body
                    color: Theme.color.foreground
                    selectByMouse: true

                    Keys.onReturnPressed: root.save()
                    Keys.onEnterPressed: root.save()
                    Keys.onEscapePressed: ViewState.closeRenameAlbumDialog()
                }
            }

            RowLayout {
                Layout.fillWidth: true
                Layout.topMargin: 2
                spacing: 8

                Item { Layout.fillWidth: true }

                Rectangle {
                    width: cancelText.implicitWidth + 24
                    height: 28
                    radius: Theme.geometry.pillRadius
                    color: cancelHover.hovered ? Theme.color.selection : Theme.color.surfaceElevated
                    border.color: Theme.color.border
                    border.width: 1

                    Text {
                        id: cancelText
                        anchors.centerIn: parent
                        text: "Cancel"
                        font.family: Theme.font.family
                        font.pixelSize: Theme.font.bodySmall
                        color: Theme.color.foreground
                    }

                    HoverHandler { id: cancelHover }
                    TapHandler { onTapped: ViewState.closeRenameAlbumDialog() }
                }

                Rectangle {
                    width: saveText.implicitWidth + 24
                    height: 28
                    radius: Theme.geometry.pillRadius
                    color: Theme.color.accent

                    Text {
                        id: saveText
                        anchors.centerIn: parent
                        text: "Save"
                        font.family: Theme.font.family
                        font.pixelSize: Theme.font.bodySmall
                        font.bold: true
                        color: Theme.color.background
                    }

                    HoverHandler { id: saveHover }
                    TapHandler { onTapped: root.save() }
                }
            }
        }
    }

    function save() {
        var newName = albumInput.text.trim();
        if (newName.length > 0 && newName !== ViewState.renameInitialName) {
            Backend.renameAlbum(ViewState.renameTargetId, newName);
            ViewState.showToast("Album renamed to '" + newName + "'");
        }
        ViewState.closeRenameAlbumDialog();
    }
}
