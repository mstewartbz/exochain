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

(function(global) {
  'use strict';

  var cache = Object.assign({}, global.__COMMAND_BASE_DURABLE_STATE__ || {});
  var pending = Object.create(null);
  var endpoint = '/api/dagdb/commandbase/ui-state';

  function send(key, value) {
    pending[key] = value;
    var body = JSON.stringify({ key: key, value: value });
    fetch(endpoint, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: body,
      credentials: 'same-origin'
    }).then(function(response) {
      if (!response.ok) throw new Error('durable-state write failed: ' + response.status);
      delete pending[key];
    }).catch(function(error) {
      console.error('[dagdb-durable-state] failed to persist ' + key + ': ' + error.message);
    });
  }

  global.commandBaseDurableState = {
    getItem: function(key) {
      return Object.prototype.hasOwnProperty.call(cache, key) ? cache[key] : null;
    },
    setItem: function(key, value) {
      var stringValue = String(value);
      cache[key] = stringValue;
      send(key, stringValue);
    },
    removeItem: function(key) {
      delete cache[key];
      send(key, null);
    },
    pendingKeys: function() {
      return Object.keys(pending).sort();
    }
  };
})(window);
