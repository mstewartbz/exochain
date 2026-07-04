const { Pool } = require('pg');

async function setup() {
  // Try different connection options to find working one
  const connOptions = [
    { user: 'postgres', host: 'localhost', database: 'postgres', port: 5432 },
    { user: 'node', host: 'localhost', database: 'postgres', port: 5432 },
    { host: 'localhost', database: 'postgres', port: 5432 },
    { host: '/var/run/postgresql', database: 'postgres' },
  ];

  for (const opts of connOptions) {
    try {
      const pool = new Pool(opts);
      const res = await pool.query('SELECT current_user, current_database()');
      console.log('Connected as:', JSON.stringify(res.rows[0]));

      // Create role
      try {
        await pool.query("CREATE ROLE livesafe WITH LOGIN PASSWORD 'livesafe_dev_password' SUPERUSER");
        console.log('Created livesafe role');
      } catch (e) {
        if (e.message.includes('already exists')) console.log('Role livesafe already exists');
        else console.log('Role creation:', e.message);
      }

      // Create database
      try {
        await pool.query('CREATE DATABASE livesafe OWNER livesafe');
        console.log('Created livesafe database');
      } catch (e) {
        if (e.message.includes('already exists')) console.log('Database livesafe already exists');
        else console.log('Database creation:', e.message);
      }

      await pool.end();
      console.log('Setup complete');
      return;
    } catch (e) {
      console.log('Failed with', JSON.stringify(opts), ':', e.message);
    }
  }
  console.log('ERROR: Could not connect to PostgreSQL with any method');
}

setup().then(() => process.exit(0)).catch(e => { console.error(e); process.exit(1); });
