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
use ruspiro_brain as brain;
use ruspiro_core::*;
use ruspiro_lock as lock;
use ruspiro_singleton as singleton;
pub use ruspiro_uart as uart;

//pub type SharedTransport = Arc<ruspiro_singleton::Singleton<ruspiro_uart::Uart0>>;

pub mod hci;
mod hctl;

//mod hci;
//pub use hci::*;
