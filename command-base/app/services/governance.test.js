'use strict';

const assert = require('node:assert/strict');
const { readFileSync } = require('node:fs');
const { join } = require('node:path');
const test = require('node:test');

const source = readFileSync(join(__dirname, 'governance.js'), 'utf8');

function functionSource(name) {
  const start = source.indexOf(`function ${name}`);
  assert.notEqual(start, -1, `${name} source must be present`);
  const nextFunction = source.indexOf('\n  function ', start + 1);
  return nextFunction === -1 ? source.slice(start) : source.slice(start, nextFunction);
}

test('assignAdjudicationStage uses deterministic adjudicator ordering', () => {
  const body = functionSource('assignAdjudicationStage');

  assert.equal(
    /ORDER\s+BY\s+RANDOM\s*\(/i.test(body),
    false,
    'adjudicator selection must not use SQLite ORDER BY RANDOM()',
  );
  assert.match(
    body,
    /challenge_adjudication_stages[\s\S]+adjudicator_id\s*=\s*team_members\.id/i,
    'adjudicator ordering must account for existing adjudication load',
  );
  assert.match(
    body,
    /ORDER\s+BY[\s\S]+ASC[\s\S]+id\s+ASC/i,
    'adjudicator ordering must use a stable id tie-breaker',
  );
});
