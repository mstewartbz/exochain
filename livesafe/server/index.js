const express = require('express');
const cors = require('cors');
const helmet = require('helmet');
const rateLimit = require('express-rate-limit');
const { Pool } = require('pg');
const path = require('path');
const fs = require('fs');

// Load environment variables - override=true ensures .env PORT takes precedence over env
require('dotenv').config({ path: path.join(__dirname, '..', '.env'), override: true });

// Fail-closed secret validation. These have no source fallback (rotated
// 2026-07-04): JWT_SECRET signs every session token, and the two encryption
// keys protect subscriber medical data at rest. Refuse to start without them
// rather than silently running on a weak default. Must run before any route
// module is required, since routes read these at load time.
for (const requiredSecret of ['JWT_SECRET', 'CREDENTIAL_ENCRYPTION_KEY', 'RECORD_ENCRYPTION_SECRET']) {
  if (!process.env[requiredSecret] || process.env[requiredSecret].trim() === '') {
    console.error(`FATAL: required secret ${requiredSecret} is not set; refusing to start`);
    process.exit(1);
  }
}

const { sendAiHelpStatusResponse } = require('./utils/ai-help-status');
const {
  sendAiHelpUsageSummaryStatusResponse,
} = require('./utils/ai-help-usage-summary-status');
const {
  sendAiHelpSessionTranscriptStatusResponse,
} = require('./utils/ai-help-session-transcript-status');
const {
  sendAiHelpUnansweredTopicStatusResponse,
} = require('./utils/ai-help-unanswered-topic-status');
const { sendFeedbackBoardStatusResponse } = require('./utils/feedback-board-status');
const {
  sendFeedbackCodeHintsStatusResponse,
} = require('./utils/feedback-code-hints-status');
const { sendHealthStatusResponse } = require('./utils/health-status');
const { sendTrustStatusResponse } = require('./utils/trust-status');
const {
  runtimeExochainConnectivityStatus,
} = require('./utils/exochain-connectivity-status');
const { sendError, isDatabaseError } = require('./utils/errorHandler');
const { registerStatusRouteContracts } = require('./utils/status-route-contracts');

const app = express();
const PORT = process.env.PORT || 3001;

// Global pool reference (set during startup)
let pool;
let embeddedPg;

// ── Database initialization ──────────────────────────────────
async function initDatabase() {
  const dbUrl = process.env.DATABASE_URL || 'postgresql://livesafe:livesafe_dev_password@localhost:5432/livesafe';

  // First, try connecting to an existing PostgreSQL instance
  const sslConfig = dbUrl.includes('supabase') || dbUrl.includes('sslmode') ? { rejectUnauthorized: false } : false;
  const testPool = new Pool({ connectionString: dbUrl, connectionTimeoutMillis: 10000, ssl: sslConfig });
  try {
    await testPool.query('SELECT 1');
    await testPool.end();
    console.log('[DB] Found existing PostgreSQL instance');
    return new Pool({ connectionString: dbUrl, ssl: sslConfig });
  } catch (err) {
    await testPool.end().catch(() => {});
    console.log('[DB] External PostgreSQL connection failed:', err.message);
    console.log('[DB] DATABASE_URL prefix:', dbUrl.substring(0, 40) + '...');
    
    // In production, don't fall back to embedded PG — just crash clearly
    if (process.env.NODE_ENV === 'production') {
      throw new Error('[DB] Production mode — refusing to fall back to embedded PG. Fix DATABASE_URL. Error: ' + err.message);
    }
    console.log('[DB] Dev mode — falling back to embedded-postgres...');
  }

  // Fall back to custom embedded-postgres wrapper (handles locale issues)
  const EmbeddedPG = require('./db/embedded-pg');
  const dataDir = path.join(__dirname, '..', 'data', 'db');

  embeddedPg = new EmbeddedPG({
    databaseDir: dataDir,
    user: 'livesafe',
    password: 'livesafe_dev_password',
    port: 5432,
    persistent: true,
  });

  // Initialize if needed
  await embeddedPg.initialise();

  // Start PostgreSQL
  await embeddedPg.start();

  // Create the livesafe database if it doesn't exist
  await embeddedPg.createDatabase('livesafe');
  console.log('[DB] Embedded PostgreSQL ready');

  const embeddedUrl = `postgresql://livesafe:livesafe_dev_password@localhost:5432/livesafe`;
  return new Pool({ connectionString: embeddedUrl });
}

// ── Apply schema ─────────────────────────────────────────────
async function applySchema(pool) {
  const schemaPath = path.join(__dirname, 'db', 'schema.sql');
  if (fs.existsSync(schemaPath)) {
    try {
      const schema = fs.readFileSync(schemaPath, 'utf8');
      await pool.query(schema);
      console.log('[DB] Schema applied successfully');
    } catch (err) {
      // Schema might already be applied
      console.log('[DB] Schema note:', err.message);
    }
  }
}

// ── Main startup ─────────────────────────────────────────────
async function startServer() {
  try {
    // Initialize database
    pool = await initDatabase();

    // Test connection
    const res = await pool.query('SELECT NOW()');
    console.log('[DB] PostgreSQL connected successfully at', res.rows[0].now);

    // Apply schema
    await applySchema(pool);

    // Helper: detect PostgreSQL connection/availability errors (Feature #194)
    function isDbConnectionError(err) {
      if (!err) return false;
      const dbErrCodes = ['ECONNREFUSED', '57P01', '08006', '08001', '08004', '3D000', '53300'];
      if (dbErrCodes.includes(err.code)) return true;
      const dbErrMessages = [
        'Connection refused', 'ECONNREFUSED', 'connection refused',
        'the database system is', 'Connection terminated',
        'cannot connect to server', 'pool is draining', 'Client was closed',
        'read ECONNRESET', 'getaddrinfo', 'connect ETIMEDOUT',
      ];
      return dbErrMessages.some((msg) => err.message && err.message.includes(msg));
    }

    // Wrap pool with SQL query logging and DB error detection (Feature #194)
    const dbWithLogging = {
      query: async (...args) => {
        const queryText = typeof args[0] === 'string' ? args[0] : args[0].text;
        console.log(`[SQL] ${queryText.substring(0, 200)}`);
        const start = Date.now();
        try {
          const result = await pool.query(...args);
          const duration = Date.now() - start;
          console.log(`[SQL] Completed in ${duration}ms — ${result.rowCount} row(s)`);
          return result;
        } catch (err) {
          const duration = Date.now() - start;
          console.error(`[SQL] Failed in ${duration}ms — ${err.message}`);
          if (isDbConnectionError(err)) {
            const serviceError = new Error(
              'The database is temporarily unavailable. Please try again in a moment.'
            );
            serviceError.status = 503;
            serviceError.code = 'DB_UNAVAILABLE';
            serviceError.isDbError = true;
            serviceError.originalCode = err.code;
            throw serviceError;
          }
          throw err;
        }
      },
      connect: () => pool.connect(),
      end: () => pool.end(),
      on: (...args) => pool.on(...args),
    };

    // Make pool available to routes (with logging)
    app.locals.db = dbWithLogging;

    // Trust Fly.io / reverse proxy headers
    app.set('trust proxy', 1);

    // Middleware
    app.use(helmet({ contentSecurityPolicy: false }));
    app.use(cors({
      origin: true, // allow all origins in production (Fly.io + custom domains)
      credentials: true,
    }));

    // ── Correlation ID middleware (Feature #402) ──────────────────
    // MUST be registered BEFORE express.json() so that even malformed-JSON
    // parse errors (thrown inside body-parser before routes run) carry a
    // correlation ID in the response header and body.
    app.use((req, res, next) => {
      const { v4: uuidv4 } = require('uuid');
      // Honour incoming header (X-Correlation-ID / X-Request-ID) so clients can
      // propagate their own trace IDs; otherwise generate a fresh one.
      const correlationId =
        req.headers['x-correlation-id'] ||
        req.headers['x-request-id'] ||
        uuidv4();
      req.correlationId = correlationId;
      res.setHeader('X-Correlation-ID', correlationId);

      // Patch res.json so every error response automatically carries correlationId
      const originalJson = res.json.bind(res);
      res.json = function (body) {
        if (
          res.statusCode >= 400 &&
          body &&
          typeof body === 'object' &&
          !Array.isArray(body) &&
          !body.correlationId
        ) {
          body = { ...body, correlationId };
        }
        return originalJson(body);
      };

      next();
    });

    app.use(express.json({ limit: '10mb' }));
    app.use(express.urlencoded({ extended: true }));

    // Rate limiting for auth endpoints
    const authLimiter = rateLimit({
      windowMs: 15 * 60 * 1000, // 15 minutes
      max: 50,
      message: { error: 'Too many requests, please try again later.' },
    });
    app.use('/api/auth', authLimiter);

    // Request logging — includes correlationId for log traceability (Feature #402)
    app.use((req, res, next) => {
      console.log(`[API] ${req.method} ${req.path} [correlationId=${req.correlationId}]`);
      next();
    });

    registerStatusRouteContracts(app, {
      async sendHealthResponse(_req, res) {
        return sendHealthStatusResponse(_req, res, {
          pool,
          exochainConnected: runtimeExochainConnectivityStatus.getConnected(),
          version: '1.0.0',
          uptime: process.uptime(),
        });
      },
      sendTrustStatusResponse(req, res) {
        sendTrustStatusResponse(req, res, {
          exochainConnected: runtimeExochainConnectivityStatus.getConnected(),
          version: '1.0.0',
          uptimeSeconds: process.uptime(),
        });
      },
      sendAiHelpStatusResponse(req, res) {
        sendAiHelpStatusResponse(req, res, {
          helpAiEnabled: process.env.LIVESAFE_HELP_AI_ENABLED,
          feedbackWritesEnabled: process.env.LIVESAFE_FEEDBACK_WRITES_ENABLED,
          helpAiMandatedReporterEnabled:
            process.env.LIVESAFE_HELP_AI_MANDATED_REPORTER_ENABLED,
          feedbackAgentDispatchEnabled:
            process.env.LIVESAFE_FEEDBACK_AGENT_DISPATCH_ENABLED,
          feedbackScreenshotsEnabled:
            process.env.LIVESAFE_FEEDBACK_SCREENSHOTS_ENABLED,
          feedbackCodeHintsEnabled:
            process.env.LIVESAFE_FEEDBACK_CODE_HINTS_ENABLED,
          feedbackAgentTriggerStatuses:
            process.env.LIVESAFE_FEEDBACK_AGENT_TRIGGER_STATUSES,
          helpAiSessionTtlHours: process.env.LIVESAFE_HELP_AI_SESSION_TTL_HOURS,
          helpAiReportIntervalMinutes:
            process.env.LIVESAFE_HELP_AI_REPORT_INTERVAL_MINUTES,
          helpAiUnansweredThreshold:
            process.env.LIVESAFE_HELP_AI_UNANSWERED_THRESHOLD,
        });
      },
      sendAiHelpUsageSummaryStatusResponse,
      sendAiHelpSessionTranscriptStatusResponse,
      sendAiHelpUnansweredTopicStatusResponse,
      sendFeedbackBoardStatusResponse,
      sendFeedbackCodeHintsStatusResponse,
    });

    // Slow response test endpoint — used to verify client-side timeout handling
    app.get('/api/test/slow', async (req, res) => {
      const delay = Math.min(parseInt(req.query.delay) || 5000, 60000); // max 60s
      console.log(`[Test] Simulating slow response with ${delay}ms delay`);
      await new Promise((resolve) => setTimeout(resolve, delay));
      res.json({ status: 'ok', message: 'Slow response completed', delay_ms: delay });
    });

    // Fast response test endpoint — used to verify connectivity after timeout
    app.get('/api/test/ping', async (req, res) => {
      res.json({ status: 'ok', message: 'pong', timestamp: new Date().toISOString() });
    });

    // DB error simulation endpoint — used to verify Feature #194 (graceful DB error handling)
    app.get('/api/test/db-error', async (req, res) => {
      const err = new Error('Connection refused: could not connect to server (ECONNREFUSED)');
      err.code = 'ECONNREFUSED';
      err.isDbError = true;
      err.status = 503;
      return res.status(503).json({
        error: 'The database is temporarily unavailable. Please try again in a moment.',
        code: 'DB_UNAVAILABLE',
      });
    });

    // Import routes
    try {
      const subscriberRoutes = require('./routes/subscribers');
      const paceRoutes = require('./routes/pace');
      const cardRoutes = require('./routes/card');
      const scanRoutes = require('./routes/scan');
      const recordsRoutes = require('./routes/records');
      const consentRoutes = require('./routes/consent');
      const alertsRoutes = require('./routes/alerts');
      const auditRoutes = require('./routes/audit');
      const odentityRoutes = require('./routes/odentity');
      const authRoutes = require('./routes/auth');
      const credentialsRoutes = require('./routes/credentials');
      const notificationsRoutes = require('./routes/notifications');
      const researchRoutes = require('./routes/research');
      const adminRoutes = require('./routes/admin');
      const devicesRoutes = require('./routes/devices');
      const marketplaceRoutes = require('./routes/marketplace');

      app.use('/api/auth', authRoutes);
      app.use('/api/subscribers', subscriberRoutes);
      app.use('/api/pace', paceRoutes);
      app.use('/api/card', cardRoutes);
      app.use('/api/scan', scanRoutes);
      app.use('/api/records', recordsRoutes);
      app.use('/api/consent', consentRoutes);
      app.use('/api/alerts', alertsRoutes);
      app.use('/api/audit', auditRoutes);
      app.use('/api/odentity', odentityRoutes);
      app.use('/api/credentials', credentialsRoutes);
      app.use('/api/notifications', notificationsRoutes);
      app.use('/api/research', researchRoutes);
      app.use('/api/admin', adminRoutes);
      app.use('/api/devices', devicesRoutes);
      app.use('/api/marketplace', marketplaceRoutes);
    } catch (err) {
      console.log('[API] Some routes not yet implemented:', err.message);
    }

    // 404 handler — correlationId is injected automatically by the res.json patch above
    app.use('/api/*', (req, res) => {
      res.status(404).json({ error: 'Endpoint not found' });
    });

    // Serve built React frontend (SPA catch-all)
    const clientDist = path.join(__dirname, '..', 'client', 'dist');
    app.use(express.static(clientDist));
    app.get('*', (req, res) => {
      res.sendFile(path.join(clientDist, 'index.html'));
    });

    // Error handler — detects DB connection errors and returns 503 with user-friendly messages
    app.use((err, req, res, next) => {
      // Use the correlationId assigned by the request middleware, or generate one as fallback
      const correlationId = req.correlationId || require('uuid').v4();
      console.error(`[API] Error [correlationId=${correlationId}]: ${err.message}`);

      if (isDatabaseError(err)) {
        console.error('[API] Database connectivity error — returning 503');
      }
      return sendError(res, err, undefined, { correlationId });
    });

    // EXOCHAIN Phase 2: non-blocking gateway health checks. The first probe is
    // delayed because Railway's private network is not routable for the first
    // few seconds after container start; the interval keeps exochain_connected
    // truthful across gateway restarts instead of freezing the boot-time result.
    const refreshExochainConnectivity = () =>
      runtimeExochainConnectivityStatus.refresh().then(({ connected, probe_state: probeState }) => {
        if (probeState === 'not-called') {
          console.log('[EXOCHAIN] Gateway health check skipped: runtime adapter is not verified');
          return;
        }

        console.log(`[EXOCHAIN] Gateway health check: ${connected ? 'connected' : 'unreachable (will retry)'}`);
      }).catch(() => {
        console.log('[EXOCHAIN] Gateway health check: unavailable (fail-closed)');
      });
    setTimeout(refreshExochainConnectivity, 5000).unref();
    setInterval(refreshExochainConnectivity, 60000).unref();

    // Start server
    const server = require('http').createServer(app);
    server.listen(PORT, '0.0.0.0', () => {
      console.log(`[Server] LiveSafe API running on http://localhost:${PORT}`);
      console.log(`[Server] Health check: http://localhost:${PORT}/api/health`);
    });

    // Graceful shutdown
    process.on('SIGTERM', async () => {
      console.log('[Server] SIGTERM received, shutting down...');
      server.close();
      if (pool) await pool.end();
      if (embeddedPg) await embeddedPg.stop();
      process.exit(0);
    });

    process.on('SIGINT', async () => {
      console.log('[Server] SIGINT received, shutting down...');
      server.close();
      if (pool) await pool.end();
      if (embeddedPg) await embeddedPg.stop();
      process.exit(0);
    });

  } catch (err) {
    console.error('[Server] Failed to start:', err);
    process.exit(1);
  }
}

startServer();

module.exports = app;
