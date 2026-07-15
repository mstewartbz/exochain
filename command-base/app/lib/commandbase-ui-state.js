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
