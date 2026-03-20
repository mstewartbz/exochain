// @exoeth/exochain-wasm — lazy-init wrapper
// Re-exports all WASM bindings with automatic initialization

const wasm = require('./wasm/exochain_wasm.js');

module.exports = wasm;
