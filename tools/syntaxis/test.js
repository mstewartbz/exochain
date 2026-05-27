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

/**
 * Syntaxis Protocol Engine - Test Suite
 *
 * Demonstrates usage of the Syntaxis Protocol Engine with various workflows
 */

const crypto = require('crypto');
const {
  createSyntaxisEngine,
  NODE_REGISTRY,
  BCTS_TRANSITIONS,
  BCTS_STATES
} = require('./index');
const { canonicalJson, deterministicId, hashCanonical } = require('./determinism');

const CREATED_AT_HLC = { physicalMs: 1700000000000, logical: 0 };
const SECURITY_HLC = { physicalMs: 1700000000000, logical: 1 };
const INFRA_HLC = { physicalMs: 1700000000000, logical: 2 };
const DEPLOYMENT_HLC = { physicalMs: 1700000000001, logical: 0 };
const GOVERNANCE_EVIDENCE_VERIFICATION_DOMAIN = 'syntaxis.governance-evidence-verification.v1';
const IDENTITY_PROOF_SIGNATURE_DOMAIN = 'syntaxis.identity-proof.v1';
const DELEGATION_SIGNATURE_DOMAIN = 'syntaxis.delegation.v1';
const CONSENT_RESPONSE_SIGNATURE_DOMAIN = 'syntaxis.consent-response.v1';
const INVARIANT_EVIDENCE_SIGNATURE_DOMAIN = 'syntaxis.invariant-evidence.v1';

function testSigner(verifierId) {
  const { publicKey, privateKey } = crypto.generateKeyPairSync('ed25519');
  return {
    verifierId,
    privateKey,
    publicKeyPem: publicKey.export({ type: 'spki', format: 'pem' })
  };
}

const GOVERNANCE_VERIFIER = testSigner('test-governance-verifier');
const GOVERNANCE_CERTIFIER = testSigner('test-governance-certifier');
const PANEL_CERTIFIERS = new Map();

function panelCertifier(panel) {
  if (!PANEL_CERTIFIERS.has(panel)) {
    PANEL_CERTIFIERS.set(panel, testSigner(`test-panel-${panel}`));
  }
  return PANEL_CERTIFIERS.get(panel);
}

function signPayload(signer, payload) {
  return {
    publicKeyPem: signer.publicKeyPem,
    signatureBase64: crypto
      .sign(null, Buffer.from(canonicalJson(payload), 'utf8'), signer.privateKey)
      .toString('base64'),
    signedPayloadHash: `0x${hashCanonical(payload)}`
  };
}

function identityProof(identityId, method, nonce, publicKey = 'ed25519-demo-public-key') {
  return {
    subjectId: identityId,
    method,
    nonce,
    publicKey,
    signature: 'ed25519-demo-signature',
    proofHash: `0x${hashCanonical({
      identityId,
      method,
      nonce,
      publicKey
    })}`
  };
}

function delegationLink({ grantorId, granteeId, authority, scope, previousChainHash = null }) {
  const signatureHash = `0x${hashCanonical({
    authority,
    granteeId,
    grantorId,
    scope,
    signature: 'ed25519-demo-delegation-signature'
  })}`;
  return {
    grantorId,
    granteeId,
    authority,
    scope,
    signatureHash,
    chainHash: `0x${hashCanonical({
      authority,
      granteeId,
      grantorId,
      previousChainHash,
      scope,
      signatureHash
    })}`
  };
}

function solutionGovernanceEvidence(solution) {
  const verdictId = `verdict-${solution.solutionId}-approved`;
  const nonce = deterministicId('nonce', {
    createdAtHlc: solution.createdAtHlc,
    proposalId: solution.solutionId,
    verdictId
  });
  const panelAssessments = {};
  const consentResponses = {};
  for (const panel of solution.requiredPanels) {
    panelAssessments[panel] = 'FOR';
    const responseHash = `0x${hashCanonical({
      consent: true,
      consentRequestId: `consent_req_${solution.solutionId}`,
      panel,
      proposalId: solution.solutionId,
      verdictId
    })}`;
    const responsePayload = {
      consent: true,
      consentRequestId: `consent_req_${solution.solutionId}`,
      domain: CONSENT_RESPONSE_SIGNATURE_DOMAIN,
      panel,
      proposalId: solution.solutionId,
      responseHash,
      verdictId
    };
    const responseSignature = signPayload(panelCertifier(panel), responsePayload);
    consentResponses[panel] = {
      consent: true,
      responseHash,
      signatureHash: `0x${hashCanonical({
        panel,
        responseHash,
        signer: panelCertifier(panel).publicKeyPem,
        verdictId
      })}`,
      ...responseSignature
    };
  }
  const invariantEvidence = {};
  for (const [invariant, nodeType] of Object.entries({
    GOVERNANCE_AUTHORITY: 'authority-check',
    CONSENT_COVERAGE: 'consent-verify',
    PROOF_VALIDITY: 'proof-verify',
    KERNEL_INTEGRITY: 'kernel-adjudicate'
  })) {
    if (solution.nodeSequence.includes(nodeType)) {
      const evidenceHash = `0x${hashCanonical({
        invariant,
        nodeType,
        solutionId: solution.solutionId,
        verdictId
      })}`;
      const evidencePayload = {
        domain: INVARIANT_EVIDENCE_SIGNATURE_DOMAIN,
        evidenceHash,
        invariant,
        nodeType,
        solutionId: solution.solutionId,
        verdictId
      };
      invariantEvidence[invariant] = {
        nodeType,
        evidenceHash,
        ...signPayload(GOVERNANCE_CERTIFIER, evidencePayload)
      };
    }
  }
  const identityPayload = {
    domain: IDENTITY_PROOF_SIGNATURE_DOMAIN,
    method: 'cryptographic',
    nonce,
    proposalId: solution.solutionId,
    solutionId: solution.solutionId,
    subjectId: solution.metadata.author,
    verdictId
  };
  const identitySignature = signPayload(GOVERNANCE_CERTIFIER, identityPayload);
  const delegationPayload = {
    authority: 'GOVERNANCE_PROPOSER',
    domain: DELEGATION_SIGNATURE_DOMAIN,
    granteeId: solution.metadata.author,
    grantorId: 'did:exo:governance-council',
    previousChainHash: null,
    scope: solution.solutionType
  };
  const delegationSignature = signPayload(GOVERNANCE_CERTIFIER, delegationPayload);
  const delegationSignatureHash = `0x${hashCanonical({
    authority: 'GOVERNANCE_PROPOSER',
    granteeId: solution.metadata.author,
    grantorId: 'did:exo:governance-council',
    scope: solution.solutionType,
    signature: delegationSignature.signatureBase64
  })}`;
  const councilVerdict = {
    id: verdictId,
    status: 'APPROVED',
    affectedPanels: [...solution.requiredPanels],
    panelAssessments,
    identityProof: {
      subjectId: solution.metadata.author,
      method: 'cryptographic',
      nonce,
      publicKey: GOVERNANCE_CERTIFIER.publicKeyPem,
      signature: identitySignature.signatureBase64,
      proofHash: `0x${hashCanonical({
        identityId: solution.metadata.author,
        method: 'cryptographic',
        nonce,
        publicKey: GOVERNANCE_CERTIFIER.publicKeyPem
      })}`,
      ...identitySignature
    },
    delegationChain: [
      {
        grantorId: 'did:exo:governance-council',
        granteeId: solution.metadata.author,
        authority: 'GOVERNANCE_PROPOSER',
        scope: solution.solutionType,
        signatureHash: delegationSignatureHash,
        chainHash: `0x${hashCanonical({
          authority: 'GOVERNANCE_PROPOSER',
          granteeId: solution.metadata.author,
          grantorId: 'did:exo:governance-council',
          previousChainHash: null,
          scope: solution.solutionType,
          signatureHash: delegationSignatureHash
        })}`,
        ...delegationSignature
      }
    ],
    consentResponses,
    invariantEvidence,
    systemState: { source: 'external-council-verdict' },
    precedingProposals: ['root-authority-resolution']
  };
  const verificationPayload = {
    domain: GOVERNANCE_EVIDENCE_VERIFICATION_DOMAIN,
    proposalId: solution.solutionId,
    solutionId: solution.solutionId,
    solutionType: solution.solutionType,
    verdictHash: `0x${hashCanonical(councilVerdict)}`
  };
  return {
    councilVerdict: {
      ...councilVerdict
    },
    verification: {
      verifierId: GOVERNANCE_VERIFIER.verifierId,
      ...signPayload(GOVERNANCE_VERIFIER, verificationPayload)
    }
  };
}

/**
 * Run all tests
 */
async function runTests() {
  const engine = createSyntaxisEngine({
    trustedGovernanceVerifierKeys: {
      [GOVERNANCE_VERIFIER.verifierId]: GOVERNANCE_VERIFIER.publicKeyPem
    }
  });

  console.log('========================================');
  console.log('Syntaxis Protocol Engine Test Suite');
  console.log('========================================\n');

  try {
    // Test 1: Engine Statistics
    console.log('TEST 1: Protocol Statistics');
    console.log('---------------------------');
    const stats = engine.getStatistics();
    console.log(`Total Nodes: ${stats.totalNodes}`);
    console.log(`Total Categories: ${stats.totalCategories}`);
    console.log(`Total BCTS States: ${stats.totalBCTSStates}`);
    console.log(`Available Solutions: ${stats.availableSolutionTemplates}`);
    console.log('Nodes by Category:');
    Object.entries(stats.nodesByCategory).forEach(([cat, count]) => {
      console.log(`  - ${cat}: ${count}`);
    });
    console.log('\n');

    // Test 2: Get Nodes by Category
    console.log('TEST 2: Nodes by Category');
    console.log('------------------------');
    const nodesByCategory = engine.getNodesByCategory();
    for (const [category, nodes] of Object.entries(nodesByCategory)) {
      console.log(`\n${category}:`);
      nodes.forEach(node => {
        console.log(`  - ${node.type}: ${node.description}`);
        console.log(`    Inputs: ${node.requiredInputs.join(', ')}`);
      });
    }
    console.log('\n');

    // Test 3: BCTS State Model
    console.log('TEST 3: BCTS State Model');
    console.log('------------------------');
    const initialState = 'INITIALIZED';
    console.log(`Starting state: ${initialState}`);
    console.log(`Description: ${engine.getBCTSState(initialState).description}`);
    console.log(`Valid transitions: ${engine.getValidTransitions(initialState).join(', ')}`);
    console.log('\n');

    // Test 4: Create Governance Amendment Solution
    console.log('TEST 4: Create Governance Amendment Solution');
    console.log('-------------------------------------------');
    const amendmentSolution = engine.createSolution('governance-amendment', {
      name: 'Update Voting Threshold',
      author: 'GOVERNANCE_PANEL',
      tags: ['critical', 'governance'],
      consentThresholdBasisPoints: 7500,
      maxDuration: 604800000,
      createdAtHlc: CREATED_AT_HLC
    });
    console.log(`Solution ID: ${amendmentSolution.solutionId}`);
    console.log(`Type: ${amendmentSolution.solutionType}`);
    console.log(`Nodes: ${amendmentSolution.nodeSequence.length}`);
    console.log(`Required Panels: ${amendmentSolution.requiredPanels.join(', ')}`);
    console.log('\n');

    // Test 5: Create Security Patch Solution
    console.log('TEST 5: Create Security Patch Solution');
    console.log('--------------------------------------');
    const securitySolution = engine.createSolution('security-patch', {
      name: 'Critical Security Patch',
      author: 'SECURITY_TEAM',
      patchSeverity: 'CRITICAL',
      affectedSystems: ['AUTH_SERVICE', 'LEDGER_SERVICE'],
      testingRequired: true,
      rolloutPhase: 1,
      createdAtHlc: SECURITY_HLC
    });
    console.log(`Solution ID: ${securitySolution.solutionId}`);
    console.log(`Type: ${securitySolution.solutionType}`);
    console.log(`Status: ${securitySolution.status}`);
    console.log(`Config:`, JSON.stringify(securitySolution.config, null, 2));
    console.log('\n');

    // Test 6: Compile Workflow
    console.log('TEST 6: Compile Governance Workflow');
    console.log('-----------------------------------');
    const mockCouncilVerdict = {
      id: 'verdict_test_001',
      status: 'APPROVED',
      affectedPanels: ['Identity Panel', 'Governance Panel', 'Consent Panel'],
      panelAssessments: {
        'Identity Panel': 'FOR',
        'Governance Panel': 'FOR',
        'Consent Panel': 'FOR'
      },
      consentResponses: {},
      systemState: {},
      precedingProposals: []
    };

    const mockProposal = {
      id: 'proposal_test_001',
      type: 'governance-amendment',
      proposer: 'GOVERNANCE_PANEL',
      content: {
        amendmentType: 'VOTING_THRESHOLD',
        oldThresholdBasisPoints: 6700,
        newThresholdBasisPoints: 7500
      },
      affectedPanels: ['Governance Panel', 'Consent Panel'],
      requiresConsent: true,
      requiredConsentBasisPoints: 8000,
      faultTolerant: true,
      rollbackOnFailure: true,
      maxDuration: 604800000,
      createdAtHlc: CREATED_AT_HLC
    };
    const proofNonce = deterministicId('nonce', {
      createdAtHlc: CREATED_AT_HLC,
      proposalId: mockProposal.id,
      verdictId: mockCouncilVerdict.id
    });
    mockCouncilVerdict.identityProof = identityProof(
      mockProposal.proposer,
      'cryptographic',
      proofNonce
    );
    mockCouncilVerdict.delegationChain = [
      delegationLink({
        grantorId: 'did:exo:root',
        granteeId: mockProposal.proposer,
        authority: 'GOVERNANCE_PROPOSER',
        scope: mockProposal.type
      })
    ];
    mockCouncilVerdict.consentResponses = {
      'Identity Panel': { consent: true },
      'Governance Panel': { consent: true },
      'Consent Panel': { consent: true }
    };

    const workflow = engine.compileSyntaxis(mockCouncilVerdict, mockProposal);
    console.log(`Workflow ID: ${workflow.workflowId}`);
    console.log(`Proposal: ${workflow.proposalId}`);
    console.log(`Status: ${workflow.status}`);
    console.log(`Nodes: ${workflow.nodes.length}`);
    console.log(`State Flow: ${workflow.stateFlow.join(' -> ')}`);
    console.log('\n');

    // Test 7: Validate Workflow
    console.log('TEST 7: Validate Compiled Workflow');
    console.log('----------------------------------');
    const validation = engine.validateSyntaxisWorkflow(workflow);
    console.log(`Valid: ${validation.valid}`);
    console.log(`Nodes: ${validation.nodeCount}`);
    console.log(`Dependencies: ${validation.dependencyCount}`);
    if (validation.errors.length > 0) {
      console.log('Errors:', validation.errors);
      throw new Error('Compiled workflow validation failed');
    } else {
      console.log('No validation errors');
    }
    console.log('\n');

    // Test 8: Convert to Archon YAML
    console.log('TEST 8: Convert to Archon YAML (First 30 lines)');
    console.log('----------------------------------------------');
    const yaml = engine.syntaxisToArchonYaml(workflow);
    const yamlLines = yaml.split('\n');
    yamlLines.slice(0, 30).forEach(line => {
      console.log(line);
    });
    if (yamlLines.length > 30) {
      console.log(`... (${yamlLines.length - 30} more lines)`);
    }
    console.log('\n');

    // Test 9: Deploy Solution
    console.log('TEST 9: Deploy Solution');
    console.log('-----------------------');
    const deployment = engine.deploySolution(amendmentSolution, {
      path: '/exoforge/deployments',
      environment: 'PRODUCTION',
      governanceEvidence: solutionGovernanceEvidence(amendmentSolution),
      deploymentHlc: DEPLOYMENT_HLC
    });
    console.log(`Deployment ID: ${deployment.deploymentId}`);
    console.log(`Solution ID: ${deployment.solutionId}`);
    console.log(`Status: ${deployment.status}`);
    console.log(`Duration: ${deployment.durationLogicalTicks} logical ticks`);
    console.log(`Stages Completed: ${deployment.stages.length}`);
    deployment.stages.forEach((stage, idx) => {
      console.log(`  ${idx + 1}. ${stage.name}: ${stage.description}`);
    });
    if (deployment.status !== 'DEPLOYED') {
      throw new Error(`Solution deployment failed: ${deployment.error || deployment.status}`);
    }
    const deploymentValidation = engine.validateSyntaxisWorkflow(deployment.workflow);
    if (!deploymentValidation.valid) {
      throw new Error(`Deployment workflow validation failed: ${deploymentValidation.errors.join(', ')}`);
    }
    console.log('\n');

    // Test 9b: Deployment Evidence Fails Closed Without Trusted Verifier
    console.log('TEST 9b: Deployment Evidence Fails Closed');
    console.log('-----------------------------------------');
    const untrustedEngine = createSyntaxisEngine();
    const untrustedDeployment = untrustedEngine.deploySolution(amendmentSolution, {
      path: '/exoforge/deployments',
      environment: 'PRODUCTION',
      governanceEvidence: solutionGovernanceEvidence(amendmentSolution),
      deploymentHlc: { physicalMs: 1700000000001, logical: 10 }
    });
    if (untrustedDeployment.status !== 'DEPLOYMENT_FAILED') {
      throw new Error('deployment without trusted governance verifier keyring must fail closed');
    }
    const tamperedEvidence = solutionGovernanceEvidence(amendmentSolution);
    tamperedEvidence.councilVerdict.identityProof.signatureBase64 = Buffer.alloc(64, 7)
      .toString('base64');
    tamperedEvidence.councilVerdict.identityProof.signature = tamperedEvidence
      .councilVerdict
      .identityProof
      .signatureBase64;
    const tamperedDeployment = engine.deploySolution(amendmentSolution, {
      path: '/exoforge/deployments',
      environment: 'PRODUCTION',
      governanceEvidence: tamperedEvidence,
      deploymentHlc: { physicalMs: 1700000000001, logical: 11 }
    });
    if (tamperedDeployment.status !== 'DEPLOYMENT_FAILED') {
      throw new Error('deployment with tampered Ed25519 identity proof must fail closed');
    }
    console.log('Unsigned or tampered governance evidence was rejected');
    console.log('\n');

    // Test 10: List Solution Templates
    console.log('TEST 10: Available Solution Templates');
    console.log('-------------------------------------');
    const templates = engine.listSolutionTemplates();
    templates.forEach(template => {
      console.log(`\n${template.name}:`);
      console.log(`  Type: ${template.type}`);
      console.log(`  Category: ${template.category}`);
      console.log(`  Nodes: ${template.nodeCount}`);
      console.log(`  Panels: ${template.panelCount}`);
      console.log(`  Customizable: ${template.customizableFields.join(', ')}`);
    });
    console.log('\n');

    // Test 11: Node Registry Information
    console.log('TEST 11: Sample Node Registry Entry');
    console.log('-----------------------------------');
    const nodeInfo = engine.getNodeRegistry('governance-propose');
    console.log(`Node Type: ${nodeInfo.implementation.nodeType}`);
    console.log(`Category: ${nodeInfo.category}`);
    console.log(`Description: ${nodeInfo.description}`);
    console.log(`Required Inputs: ${nodeInfo.requiredInputs.join(', ')}`);
    console.log(`Outputs: ${nodeInfo.outputs.join(', ')}`);
    console.log(`Timeout: ${nodeInfo.timeoutMs}ms`);
    console.log(`Required Panels: ${nodeInfo.requiredPanels.join(', ')}`);
    console.log('\n');

    // Test 12: Complex Workflow with Multiple Solutions
    console.log('TEST 12: Complex Workflow - Infrastructure Change');
    console.log('------------------------------------------------');
    const infraSolution = engine.createSolution('infrastructure-change', {
      name: 'Database Migration',
      author: 'INFRASTRUCTURE_TEAM',
      affectedTenants: ['tenant-prod-001', 'tenant-prod-002'],
      changeScope: 'FULL_DATABASE_MIGRATION',
      maintenanceWindow: '2024-04-15T00:00:00Z',
      blueGreenStrategy: true,
      consentThresholdBasisPoints: 7500,
      createdAtHlc: INFRA_HLC
    });
    console.log(`Solution: ${infraSolution.name}`);
    console.log(`Nodes in Sequence: ${infraSolution.nodeSequence.length}`);
    console.log('Node Sequence:');
    infraSolution.nodeSequence.forEach((node, idx) => {
      console.log(`  ${idx + 1}. ${node}`);
    });
    console.log(`Configuration:`, JSON.stringify(infraSolution.config, null, 2));
    console.log('\n');

    // Summary
    console.log('========================================');
    console.log('All Tests Completed Successfully!');
    console.log('========================================');
    console.log(`\nProtocol Version: 0.1.0`);
    console.log(`Total Node Types: ${stats.totalNodes}`);
    console.log(`Total Categories: ${stats.totalCategories}`);
    console.log(`BCTS State Machine: ${stats.totalBCTSStates} states`);
    console.log(`Solution Templates: ${stats.availableSolutionTemplates}`);

  } catch (error) {
    console.error('ERROR:', error.message);
    console.error(error.stack);
    process.exit(1);
  }
}

/**
 * Run tests if this is the main module
 */
if (require.main === module) {
  runTests().catch(error => {
    console.error('Fatal error:', error);
    process.exit(1);
  });
}

module.exports = { runTests };
