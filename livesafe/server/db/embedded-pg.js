/**
 * Custom embedded-postgres manager that handles environments
 * where en_US.UTF-8 locale is not available (falls back to C locale).
 * Wraps the platform-specific @embedded-postgres binaries directly.
 */

var spawn = require('child_process').spawn;
var fs = require('fs');
var path = require('path');
var crypto = require('crypto');
var os = require('os');
var pg = require('pg');

function embeddedPostgresPackageName() {
  var platformMap = {
    darwin: 'darwin',
    linux: 'linux',
    win32: 'windows',
  };
  var platform = platformMap[process.platform];
  if (!platform) {
    throw new Error('Unsupported embedded Postgres platform: ' + process.platform);
  }
  return platform + '-' + process.arch;
}

var BINS_DIR = path.join(
  __dirname,
  '..',
  'node_modules',
  '@embedded-postgres',
  embeddedPostgresPackageName(),
  'native',
  'bin'
);
var INITDB_BIN = path.join(BINS_DIR, 'initdb');
var POSTGRES_BIN = path.join(BINS_DIR, 'postgres');
var PG_CTL_BIN = path.join(BINS_DIR, 'pg_ctl');

function ensureExecutable(filePath) {
  try {
    var stat = fs.statSync(filePath);
    var execBits = 0o111;
    if ((stat.mode & execBits) !== execBits) {
      fs.chmodSync(filePath, stat.mode | execBits);
    }
  } catch (e) {
    console.error('[EmbeddedPG] Could not set executable:', filePath, e.message);
  }
}

function EmbeddedPG(options) {
  this.options = Object.assign({
    databaseDir: path.join(process.cwd(), 'data', 'db'),
    user: 'postgres',
    password: 'password',
    port: 5432,
    persistent: true,
  }, options);
  this.process = null;
}

/**
 * Initialize the PostgreSQL data directory
 */
EmbeddedPG.prototype.initialise = function() {
  var self = this;
  return new Promise(function(resolve, reject) {
    var dataDir = self.options.databaseDir;

    // Check if already initialized
    if (fs.existsSync(path.join(dataDir, 'PG_VERSION'))) {
      console.log('[EmbeddedPG] Data directory already initialized');
      resolve();
      return;
    }

    fs.mkdirSync(dataDir, { recursive: true });

    ensureExecutable(INITDB_BIN);

    // Write password file
    var randomId = crypto.randomBytes(6).toString('hex');
    var pwFile = path.join(os.tmpdir(), 'pg-password-' + randomId);
    fs.writeFileSync(pwFile, self.options.password + '\n');

    console.log('[EmbeddedPG] Initializing cluster at', dataDir);

    var args = [
      '--pgdata=' + dataDir,
      '--auth=password',
      '--username=' + self.options.user,
      '--pwfile=' + pwFile,
    ];

    var p = spawn(INITDB_BIN, args, {
      env: { LC_MESSAGES: 'C', LC_ALL: 'C' }
    });

    var output = '';
    p.stdout.on('data', function(chunk) {
      output += chunk.toString();
    });
    p.stderr.on('data', function(chunk) {
      output += chunk.toString();
    });

    p.on('exit', function(code) {
      // Clean up password file
      try { fs.unlinkSync(pwFile); } catch(e) {}

      if (code === 0) {
        console.log('[EmbeddedPG] Cluster initialized successfully');
        resolve();
      } else {
        console.error('[EmbeddedPG] initdb output:', output);
        reject(new Error('initdb exited with code ' + code));
      }
    });
  });
};

/**
 * Start the PostgreSQL server
 */
EmbeddedPG.prototype.start = function() {
  var self = this;
  return new Promise(function(resolve, reject) {
    ensureExecutable(POSTGRES_BIN);

    console.log('[EmbeddedPG] Starting PostgreSQL on port', self.options.port);

    var dataDir = self.options.databaseDir;

    // Update postgresql.conf to set the port
    var confPath = path.join(dataDir, 'postgresql.conf');
    if (fs.existsSync(confPath)) {
      var conf = fs.readFileSync(confPath, 'utf8');
      if (!conf.includes('port = ' + self.options.port)) {
        conf += '\nport = ' + self.options.port + '\n';
        conf += "listen_addresses = 'localhost'\n";
        fs.writeFileSync(confPath, conf);
      }
    }

    self.process = spawn(POSTGRES_BIN, [
      '-D', dataDir,
      '-p', self.options.port.toString(),
    ], {
      env: { LC_MESSAGES: 'C', LC_ALL: 'C' }
    });

    var started = false;
    var timeoutId = setTimeout(function() {
      if (!started) {
        reject(new Error('PostgreSQL failed to start within 15 seconds'));
      }
    }, 15000);

    self.process.stderr.on('data', function(chunk) {
      var msg = chunk.toString();
      console.log('[EmbeddedPG]', msg.trim());
      if (msg.includes('database system is ready to accept connections')) {
        started = true;
        clearTimeout(timeoutId);
        resolve();
      }
    });

    self.process.stdout.on('data', function(chunk) {
      console.log('[EmbeddedPG]', chunk.toString().trim());
    });

    self.process.on('close', function(code) {
      clearTimeout(timeoutId);
      if (!started) {
        reject(new Error('PostgreSQL process exited with code ' + code));
      }
    });

    self.process.on('error', function(err) {
      clearTimeout(timeoutId);
      reject(err);
    });
  });
};

/**
 * Create a database
 */
EmbeddedPG.prototype.createDatabase = function(dbName) {
  var self = this;
  return new Promise(function(resolve, reject) {
    var client = new pg.Client({
      host: 'localhost',
      port: self.options.port,
      user: self.options.user,
      password: self.options.password,
      database: 'postgres',
    });

    client.connect()
      .then(function() {
        return client.query("SELECT 1 FROM pg_database WHERE datname = $1", [dbName]);
      })
      .then(function(result) {
        if (result.rows.length === 0) {
          return client.query('CREATE DATABASE ' + dbName);
        }
        console.log('[EmbeddedPG] Database ' + dbName + ' already exists');
      })
      .then(function() {
        return client.end();
      })
      .then(resolve)
      .catch(function(err) {
        client.end().catch(function() {});
        // If database already exists, that's fine
        if (err.message && err.message.includes('already exists')) {
          resolve();
        } else {
          reject(err);
        }
      });
  });
};

/**
 * Stop the PostgreSQL server
 */
EmbeddedPG.prototype.stop = function() {
  var self = this;
  return new Promise(function(resolve) {
    if (self.process) {
      console.log('[EmbeddedPG] Stopping PostgreSQL...');
      self.process.on('close', function() {
        console.log('[EmbeddedPG] PostgreSQL stopped');
        resolve();
      });
      self.process.kill('SIGTERM');
      // Force kill after 5 seconds
      setTimeout(function() {
        if (self.process) {
          try { self.process.kill('SIGKILL'); } catch(e) {}
        }
        resolve();
      }, 5000);
    } else {
      resolve();
    }
  });
};

module.exports = EmbeddedPG;
