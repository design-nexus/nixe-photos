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
                if (event.key === Qt.Key_Space) {
                    if (ViewState.lightboxOpen) {
                        ViewState.closeLightbox();
                    } else if (ViewState.selectedItem) {
                        ViewState.openLightbox();
                    }
                    event.accepted = true;
                } else if (event.key === Qt.Key_Return || event.key === Qt.Key_Enter) {
                    if (!ViewState.lightboxOpen && ViewState.selectedItem) {
                        ViewState.openLightbox();
                        event.accepted = true;
                    }
                } else if (event.key === Qt.Key_Escape) {
                    if (ViewState.searchActive) {
                        ViewState.searchActive = false;
                    } else if (ViewState.lightboxOpen) {
                        ViewState.closeLightbox();
                    } else {
                        ViewState.clearSelection();
                    }
                    event.accepted = true;
                } else if (event.key === Qt.Key_I && !(event.modifiers & Qt.ControlModifier)) {
                    ViewState.toggleInspector();
                    event.accepted = true;
                } else if (event.key === Qt.Key_R && !(event.modifiers & Qt.ControlModifier)) {
                    ViewState.rotateCurrent();
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
                } else if (event.key === Qt.Key_Period || (event.key === Qt.Key_F && !(event.modifiers & Qt.ControlModifier))) {
                    ViewState.toggleSelectedFavorite();
                    event.accepted = true;
                } else if (event.key === Qt.Key_Left) {
                    if (ViewState.lightboxOpen) {
                        ViewState.prevPhoto();
                    } else {
                        ViewState.navigateGrid(-1);
                    }
                    event.accepted = true;
                } else if (event.key === Qt.Key_Right) {
                    if (ViewState.lightboxOpen) {
                        ViewState.nextPhoto();
                    } else {
                        ViewState.navigateGrid(1);
                    }
                    event.accepted = true;
                } else if (event.key === Qt.Key_Up) {
                    if (!ViewState.lightboxOpen) {
                        ViewState.navigateGrid(-4);
                    }
                    event.accepted = true;
                } else if (event.key === Qt.Key_Down) {
                    if (!ViewState.lightboxOpen) {
                        ViewState.navigateGrid(4);
                    }
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

            // Fullscreen Lightbox Modal Overlay
            LightboxModal {
                anchors.fill: parent
            }

            // Command Palette Search Modal
            SearchModal {
                anchors.fill: parent
            }

            // Floating Toast Notification
            Rectangle {
                anchors.bottom: parent.bottom
                anchors.horizontalCenter: parent.horizontalCenter
                anchors.bottomMargin: 40
                height: 32
                width: toastText.implicitWidth + 28
                radius: Theme.geometry.pillRadius
                color: Theme.color.surfaceElevated
                border.color: Theme.color.accent
                border.width: 1
                visible: ViewState.toastVisible
                z: 350

                RowLayout {
                    anchors.centerIn: parent
                    spacing: 6

                    NexusIcon { name: "check"; size: 12; color: Theme.color.accent }
                    Text {
                        id: toastText
                        text: ViewState.toastMessage
                        font.family: Theme.font.family
                        font.pixelSize: Theme.font.bodySmall
                        color: Theme.color.foreground
                    }
                }

                Timer {
                    interval: 2200
                    running: ViewState.toastVisible
                    onTriggered: ViewState.hideToast()
                }
            }
        }
    }
}
