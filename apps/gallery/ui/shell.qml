//@ pragma AppId io.github.nexus.photos
//@ pragma ShellId nexus-photos

import Quickshell
import QtQuick
import QtQuick.Layouts
import QtQuick.Controls
import "."

ShellRoot {
    id: root

    FloatingWindow {
        id: appWindow
        title: "Nexus Photos"
        implicitWidth: 1180
        implicitHeight: 740
        color: Theme.color.background

        FocusScope {
            anchors.fill: parent
            focus: true

            Keys.onPressed: function(event) {
                if (event.key === Qt.Key_I && !(event.modifiers & Qt.ControlModifier)) {
                    ViewState.toggleInspector();
                    event.accepted = true;
                } else if (event.key === Qt.Key_O && (event.modifiers & Qt.ControlModifier)) {
                    var target = ViewState.selectedItem ? ViewState.selectedItem.path : ViewState.libraryPath;
                    Backend.revealInStrata(target);
                    event.accepted = true;
                } else if (event.key === Qt.Key_K && (event.modifiers & Qt.ControlModifier)) {
                    ViewState.searchActive = !ViewState.searchActive;
                    event.accepted = true;
                } else if (event.key === Qt.Key_A && (event.modifiers & Qt.ControlModifier)) {
                    ViewState.selectAll();
                    event.accepted = true;
                } else if (event.key === Qt.Key_Escape) {
                    ViewState.clearSelection();
                    event.accepted = true;
                } else if (event.key === Qt.Key_Period || (event.key === Qt.Key_F && !(event.modifiers & Qt.ControlModifier))) {
                    ViewState.toggleSelectedFavorite();
                    event.accepted = true;
                } else if (event.key === Qt.Key_Left) {
                    ViewState.navigateGrid(-1);
                    event.accepted = true;
                } else if (event.key === Qt.Key_Right) {
                    ViewState.navigateGrid(1);
                    event.accepted = true;
                } else if (event.key === Qt.Key_Up) {
                    ViewState.navigateGrid(-4);
                    event.accepted = true;
                } else if (event.key === Qt.Key_Down) {
                    ViewState.navigateGrid(4);
                    event.accepted = true;
                }
            }

            ColumnLayout {
                anchors.fill: parent
                spacing: 0

                Header {
                    Layout.fillWidth: true
                }

                RowLayout {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    spacing: 0

                    Sidebar {
                        Layout.fillHeight: true
                    }

                    GalleryArea {
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                    }

                    InspectorDrawer {
                        Layout.fillHeight: true
                    }
                }

                StatusBar {
                    Layout.fillWidth: true
                }
            }
        }
    }
}
