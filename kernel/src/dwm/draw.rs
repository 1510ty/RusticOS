use core::sync::atomic::Ordering;
use limine::memory_map::{Entry, EntryType};
use crate::{CURRENT_Y, FRAMEBUFFER_BACK, INITIALIZED, SCREEN_HEIGHT, SCREEN_WIDTH};
use crate::vga::{push_command, request_update, update_screen, DrawCommand};
