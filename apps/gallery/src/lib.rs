//! Nexus Photos: the gallery (`nexus-photos`) and the stand-alone viewer
//! (`npv`, Nexus Photo Viewer) share these modules.

#![allow(clippy::new_without_default)]
pub mod actions;
pub mod clamp;
pub mod cmd;
pub mod dialog;
pub mod edit;
pub mod editor;
pub mod export;
pub mod grid;
pub mod inspector;
pub mod library;
pub mod lightbox;
pub mod metadata;
pub mod models;
pub mod paths;
pub mod prefs;
pub mod recent;
pub mod scanner;
pub mod settings_dialog;
pub mod settings_view;
pub mod stage;
pub mod state;
pub mod theme;
pub mod thumbnails;
pub mod widgets;
pub mod window;
