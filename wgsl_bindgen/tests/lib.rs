// Integration test library
// This file organizes all integration tests into logical modules

// Core functionality tests - basic bindgen features
mod core;

// Issue-specific regression tests
mod issues;

// Feature-specific tests (shader_defs, shared bind groups, etc.)
mod features;

// Large-scale integration tests (bevy, etc.)
mod integration;

// Utility tests (dependency tree, etc.)
mod utils;
