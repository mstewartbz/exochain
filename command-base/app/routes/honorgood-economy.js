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

const { ExochainEconomyClient } = require('../services/honorgood-economy');

function sendError(res, error) {
  res.status(502).json({
    error: error.message,
    settlement_authority: 'EXOCHAIN',
    local_simulation: false,
  });
}

module.exports = function registerHonorGoodEconomyRoutes(app, _db, _deps = {}) {
  const client = new ExochainEconomyClient();

  app.get('/api/honorgood/status', (_req, res) => {
    res.json(client.status());
  });

  app.post('/api/honorgood/missions', async (req, res) => {
    try {
      res.json(await client.createMission(req.body));
    } catch (error) {
      sendError(res, error);
    }
  });

  app.post('/api/honorgood/contribution-receipts', async (req, res) => {
    try {
      res.json(await client.createContributionReceipt(req.body));
    } catch (error) {
      sendError(res, error);
    }
  });

  app.post('/api/honorgood/legacy-receipts', async (req, res) => {
    try {
      res.json(await client.createLegacyReceipt(req.body));
    } catch (error) {
      sendError(res, error);
    }
  });

  app.get('/api/honorgood/legacy-receipts/:id', async (req, res) => {
    try {
      res.json(await client.getLegacyReceipt(req.params.id));
    } catch (error) {
      sendError(res, error);
    }
  });

  app.get('/api/honorgood/upstream/:id', async (req, res) => {
    try {
      res.json(await client.getLegacyReceipt(req.params.id));
    } catch (error) {
      sendError(res, error);
    }
  });

  app.post('/api/honorgood/rulesets', async (req, res) => {
    try {
      res.json(await client.createRuleset(req.body));
    } catch (error) {
      sendError(res, error);
    }
  });

  app.post('/api/honorgood/mission-settlements', async (req, res) => {
    try {
      res.json(await client.createMissionSettlement(req.body));
    } catch (error) {
      sendError(res, error);
    }
  });

  app.get('/api/honorgood/mission-settlements/:id', async (req, res) => {
    try {
      res.json(await client.getMissionSettlement(req.params.id));
    } catch (error) {
      sendError(res, error);
    }
  });
};
