const { Pool } = require('pg');
const fs = require('fs');
const path = require('path');

const pool = new Pool({
  connectionString: process.env.DATABASE_URL || 'postgresql://livesafe:livesafe_dev_password@localhost:5432/livesafe',
});

async function migrate() {
  console.log('[Migration] Starting database migration...');

  try {
    const schemaPath = path.join(__dirname, 'schema.sql');
    const schema = fs.readFileSync(schemaPath, 'utf8');

    await pool.query(schema);
    console.log('[Migration] Schema applied successfully');

    // Verify tables
    const result = await pool.query(`
      SELECT table_name FROM information_schema.tables
      WHERE table_schema = 'public'
      ORDER BY table_name;
    `);

    console.log('[Migration] Tables created:');
    result.rows.forEach(row => console.log(`  - ${row.table_name}`));

  } catch (err) {
    console.error('[Migration] Error:', err.message);
    process.exit(1);
  } finally {
    await pool.end();
  }
}

migrate();
