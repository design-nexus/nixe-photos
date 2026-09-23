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
                    Backend.revealInStrata(ViewState.libraryPath);
                    event.accepted = true;
                } else if (event.key === Qt.Key_K && (event.modifiers & Qt.ControlModifier)) {
                    ViewState.searchActive = !ViewState.searchActive;
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
