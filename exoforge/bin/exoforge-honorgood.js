#!/usr/bin/env node

import { readFile } from 'node:fs/promises';

import {
  EXOCHAIN_SETTLEMENT_AUTHORITY,
  ExoForgeHonorGoodClient,
  generateLegacyReceiptProposal,
} from '../lib/honorgood.js';

function printJson(value) {
  console.log(JSON.stringify(value, null, 2));
}

function printUsage() {
  console.log(`Usage: exoforge-honorgood <command> [options]

Commands:
  status                 Show EXOCHAIN economy adapter status
  propose-legacy         Generate an unratified upstream LegacyReceipt proposal
  submit-legacy          Submit a complete LegacyReceipt payload to EXOCHAIN core

Options:
  --json                 Output JSON
  --upstream <name>      Upstream project name for propose-legacy
  --receiving <system>   Receiving system for propose-legacy
  --license <name>       License reference for propose-legacy
  --source-uri <uri>     Source URI for propose-legacy
  --materiality <tier>   Materiality tier for propose-legacy
  --basis <basis:bp>     Proposed basis line; may be repeated
  --file <path>          JSON payload file for submit-legacy
  -h, --help             Show this help`);
}

function parseArgs(argv) {
  const result = {
    command: argv[2] || 'status',
    json: false,
    basis: [],
  };
  for (let i = 3; i < argv.length; i += 1) {
    const arg = argv[i];
    switch (arg) {
      case '--json':
        result.json = true;
        break;
      case '--upstream':
        result.upstreamProject = argv[++i];
        break;
      case '--receiving':
        result.receivingSystem = argv[++i];
        break;
      case '--license':
        result.license = argv[++i];
        break;
      case '--source-uri':
        result.sourceUri = argv[++i];
        break;
      case '--materiality':
        result.materialityTier = argv[++i];
        break;
      case '--basis':
        result.basis.push(argv[++i]);
        break;
      case '--file':
        result.file = argv[++i];
        break;
      case '-h':
      case '--help':
        result.help = true;
        break;
      default:
        throw new Error(`unknown option: ${arg}`);
    }
  }
  return result;
}

function parseBasis(lines) {
  return lines.map((line) => {
    const [basis, bp] = String(line || '').split(':');
    if (!basis || !bp) {
      throw new Error(`invalid --basis value: ${line}`);
    }
    const shareBp = Number.parseInt(bp, 10);
    if (!Number.isInteger(shareBp) || shareBp < 0 || shareBp > 10_000) {
      throw new Error(`invalid basis points for ${basis}: ${bp}`);
    }
    return { basis, share_bp: shareBp };
  });
}

async function main() {
  const args = parseArgs(process.argv);
  if (args.help) {
    printUsage();
    return;
  }

  const client = new ExoForgeHonorGoodClient();
  switch (args.command) {
    case 'status': {
      const status = client.status();
      if (args.json) {
        printJson(status);
      } else {
        console.log(`settlement authority: ${status.settlement_authority}`);
        console.log(`configured: ${status.configured ? 'yes' : 'no'}`);
        console.log('local settlement authority: no');
      }
      return;
    }
    case 'propose-legacy': {
      const proposal = generateLegacyReceiptProposal({
        upstreamProject: args.upstreamProject,
        receivingSystem: args.receivingSystem,
        license: args.license,
        sourceUri: args.sourceUri,
        materialityTier: args.materialityTier,
        proposedBasis: parseBasis(args.basis),
      });
      if (args.json) {
        printJson(proposal);
      } else {
        console.log(`${proposal.legacy_receipt.contribution_name} -> ${proposal.legacy_receipt.receiving_system}`);
        console.log(`status: ${proposal.legacy_receipt.status}`);
        console.log(`settlement authority: ${EXOCHAIN_SETTLEMENT_AUTHORITY}`);
      }
      return;
    }
    case 'submit-legacy': {
      if (!args.file) {
        throw new Error('--file is required for submit-legacy');
      }
      const payload = JSON.parse(await readFile(args.file, 'utf8'));
      const response = await client.submitLegacyReceipt(payload);
      if (args.json) {
        printJson(response);
      } else {
        console.log('legacy receipt submitted to EXOCHAIN economy API');
      }
      return;
    }
    default:
      throw new Error(`unknown command: ${args.command}`);
  }
}

main().catch((err) => {
  const message = err && err.message ? err.message : String(err);
  console.error(message);
  process.exit(1);
});
