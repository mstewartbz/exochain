'use strict';

function unavailableProxy(error) {
  return new Proxy({}, {
    get(_target, property) {
      if (property === '__exochainWasmUnavailable') return true;
      throw new Error(`exochain_wasm_unavailable: ${error.message}`);
    },
  });
}

function loadExochainWasm() {
  try {
    return require('@exochain/exochain-wasm');
  } catch (packageError) {
    try {
      return require('../../../packages/exochain-wasm/wasm');
    } catch (workspaceError) {
      const error = new Error(`${packageError.message}; ${workspaceError.message}`);
      return unavailableProxy(error);
    }
  }
}

module.exports = {
  loadExochainWasm,
};
