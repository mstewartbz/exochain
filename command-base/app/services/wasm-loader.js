// Copyright 2026 Exochain Foundation
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at:
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//
// SPDX-License-Identifier: Apache-2.0

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

