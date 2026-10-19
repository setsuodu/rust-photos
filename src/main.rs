#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
// Body restored at build time from compressed chunks (see build.rs)
include!(concat!(env!("OUT_DIR"), "/restored_main.rs"));
