/***********************************************************************************************************************
 * Copyright (c) 2019 by the authors
 *
 * Author: André Borrmann
 * License: Apache License 2.0
 **********************************************************************************************************************/
#![doc(html_root_url = "https://docs.rs/ruspiro-btle/0.1.0")]
#![cfg_attr(not(any(test, doctest)), no_std)]
#![feature(const_fn, drain_filter)]
//! # Bluetooth Low Energy interface
//!
//!
//! # Features
//! - ``ruspiro_pi3``
//!

extern crate alloc;
use alloc::sync::Arc;
use ruspiro_brain;
use ruspiro_console;
use ruspiro_interrupt;
use ruspiro_lock;
use ruspiro_register;
use ruspiro_singleton;
use ruspiro_uart;

pub type SharedTransport = Arc<ruspiro_singleton::Singleton<ruspiro_uart::Uart0>>;

mod hci;
pub use hci::*;
