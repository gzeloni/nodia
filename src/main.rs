// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023-2025 Gustavo Zeloni <gustavo@gzeloni.dev>

//! Command-line entry point for the `nodia` executable.

mod cli;

fn main() {
    std::process::exit(cli::run(std::env::args().collect()));
}
