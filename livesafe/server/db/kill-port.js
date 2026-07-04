const { execSync } = require('child_process');

const ports = [3000, 3001, 3002];

for (const port of ports) {
  try {
    const pids = execSync(`lsof -ti :${port} 2>/dev/null`).toString().trim().split('\n').filter(Boolean);
    for (const pid of pids) {
      console.log(`Killing PID ${pid} on port ${port}`);
      try {
        process.kill(parseInt(pid), 'SIGKILL');
      } catch (e) {
        console.log(`  Could not kill ${pid}: ${e.message}`);
      }
    }
  } catch (e) {
    console.log(`Port ${port}: no process found`);
  }
}

console.log('Done. Waiting 2s...');
setTimeout(() => {
  for (const port of ports) {
    try {
      const pids = execSync(`lsof -ti :${port} 2>/dev/null`).toString().trim();
      console.log(`Port ${port} still has: ${pids}`);
    } catch (e) {
      console.log(`Port ${port}: free`);
    }
  }
}, 2000);
