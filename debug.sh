#!/bin/bash
cargo build
rust-lldb -s ./init.lldb 
