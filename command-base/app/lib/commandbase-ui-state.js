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

function mountCommandBaseUiStateRoutes(app, db) {
  const state = Object.create(null);

  app.get('/api/dagdb/commandbase/ui-state', (_req, res) => {
    res.json({ state });
  });

  app.post('/api/dagdb/commandbase/ui-state', (req, res) => {
    const key = typeof req.body?.key === 'string' ? req.body.key : '';
    if (!/^[A-Za-z0-9_.:-]{1,128}$/.test(key)) {
      res.status(400).json({ error: 'invalid durable state key' });
      return;
    }
    if (req.body.value === null || req.body.value === undefined) {
      delete state[key];
    } else {
      state[key] = String(req.body.value);
    }
    if (db && typeof db.recordDurableState === 'function') {
      db.recordDurableState(key, state[key] ?? '');
    }
    res.json({ ok: true, key });
  });
}

module.exports = {
  mountCommandBaseUiStateRoutes,
};
