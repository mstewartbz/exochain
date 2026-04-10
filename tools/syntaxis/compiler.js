/**
 * Syntaxis Compiler
 *
 * The compilation engine that translates council verdicts and proposals
 * into complete Syntaxis workflows.
 */

const { NODE_IMPLEMENTATIONS } = require('./nodes');

/**
 * BCTS State Transition Map
 * Defines valid state transitions for the 14-state BCTS model
 */
const BCTS_TRANSITIONS = {
  'INITIALIZED': ['IDENTITY_REQUIRED', 'INVALID'],
  'IDENTITY_REQUIRED': ['IDENTITY_VERIFIED', 'VERIFICATION_FAILED'],
  'IDENTITY_VERIFIED': ['AUTHORITY_CHECK', 'INSUFFICIENT_AUTHORITY'],
  'AUTHORITY_CHECK': ['AUTHORIZED', 'UNAUTHORIZED'],
  'AUTHORIZED': ['CONSENT_PHASE', 'DIRECT_EXECUTION'],
  'CONSENT_PHASE': ['CONSENT_VERIFIED', 'CONSENT_INSUFFICIENT'],
  'CONSENT_VERIFIED': ['GOVERNANCE_REVIEW', 'EXECUTION_READY'],
  'GOVERNANCE_REVIEW': ['GOVERNANCE_PASSED', 'GOVERNANCE_FAILED'],
  'GOVERNANCE_PASSED': ['EXECUTION_READY', 'DISPUTE_ESCALATION'],
  'EXECUTION_READY': ['EXECUTING', 'AWAITING_FINAL_AUTHORIZATION'],
  'EXECUTING': ['COMPLETED', 'EXECUTION_FAILED'],
  'COMPLETED': ['FINALIZED', 'ROLLBACK_REQUIRED'],
  'FINALIZED': ['CLOSED'],
  'CLOSED': []
};

/**
 * Proposal type to Syntaxis node mappings
 */
const PROPOSAL_TYPE_MAPPINGS = {
  'governance-amendment': {
    nodes: ['governance-propose', 'consent-request', 'governance-vote', 'governance-resolve', 'kernel-adjudicate'],
    requiredPanels: ['Governance Panel', 'Consent Panel', 'Kernel Panel'],
    stateFlow: ['INITIALIZED', 'CONSENT_PHASE', 'GOVERNANCE_REVIEW', 'GOVERNANCE_PASSED', 'EXECUTION_READY']
  },
  'feature-implementation': {
    nodes: ['governance-propose', 'authority-delegate', 'proof-generate', 'tenant-isolate', 'combinator-sequence'],
    requiredPanels: ['Governance Panel', 'Identity Panel', 'Kernel Panel', 'Infrastructure Panel'],
    stateFlow: ['INITIALIZED', 'IDENTITY_VERIFIED', 'AUTHORIZED', 'EXECUTION_READY', 'EXECUTING']
  },
  'bug-fix': {
    nodes: ['governance-propose', 'proof-generate', 'proof-verify', 'combinator-sequence', 'dag-append'],
    requiredPanels: ['Governance Panel', 'Kernel Panel'],
    stateFlow: ['INITIALIZED', 'GOVERNANCE_REVIEW', 'EXECUTION_READY', 'EXECUTING', 'COMPLETED']
  },
  'security-patch': {
    nodes: ['governance-propose', 'identity-verify', 'proof-generate', 'kernel-adjudicate', 'invariant-check'],
    requiredPanels: ['Governance Panel', 'Identity Panel', 'Kernel Panel'],
    stateFlow: ['INITIALIZED', 'IDENTITY_VERIFIED', 'AUTHORIZED', 'GOVERNANCE_PASSED', 'EXECUTION_READY']
  },
  'infrastructure-change': {
    nodes: ['governance-propose', 'authority-check', 'tenant-isolate', 'combinator-parallel', 'mcp-enforce'],
    requiredPanels: ['Governance Panel', 'Identity Panel', 'Infrastructure Panel', 'AI Panel'],
    stateFlow: ['INITIALIZED', 'IDENTITY_VERIFIED', 'AUTHORIZED', 'EXECUTION_READY', 'EXECUTING']
  },
  'escalation-resolution': {
    nodes: ['escalation-trigger', 'kernel-adjudicate', 'human-override', 'consent-verify'],
    requiredPanels: ['Escalation Panel', 'Kernel Panel', 'Executive Panel', 'Consent Panel'],
    stateFlow: ['INITIALIZED', 'GOVERNANCE_REVIEW', 'DISPUTE_ESCALATION', 'EXECUTION_READY', 'COMPLETED']
  },
  'access-control-update': {
    nodes: ['identity-verify', 'authority-delegate', 'consent-request', 'authority-check', 'governance-vote'],
    requiredPanels: ['Identity Panel', 'Governance Panel', 'Consent Panel'],
    stateFlow: ['INITIALIZED', 'IDENTITY_VERIFIED', 'CONSENT_PHASE', 'GOVERNANCE_REVIEW', 'GOVERNANCE_PASSED']
  }
};

/**
 * Syntaxis Compiler Class
 */
class SyntaxisCompiler {
  constructor() {
    this.proposalTypeMappers = PROPOSAL_TYPE_MAPPINGS;
    this.bctsTransitions = BCTS_TRANSITIONS;
  }

  /**
   * Compiles a council verdict and proposal into a Syntaxis workflow
   * @param {Object} councilVerdict - Verdict from council with panel assessments
   * @param {Object} proposal - The proposal being compiled
   * @returns {Object} Complete Syntaxis workflow definition
   */
  compileSyntaxis(councilVerdict, proposal) {
    if (!councilVerdict || !proposal) {
      throw new Error('Council verdict and proposal are required');
    }

    // Validate inputs
    if (!proposal.type || !this.proposalTypeMappers[proposal.type]) {
      throw new Error(`Unknown proposal type: ${proposal.type}`);
    }

    const proposalMapping = this.proposalTypeMappers[proposal.type];

    // Build the node graph
    const nodes = [];
    const nodesByType = {};
    const dependencies = {};

    // Create nodes based on proposal type
    for (const nodeType of proposalMapping.nodes) {
      const node = {
        id: `node_${nodeType}_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`,
        type: nodeType,
        category: this._getCategoryForNodeType(nodeType),
        inputs: this._buildNodeInputs(nodeType, councilVerdict, proposal),
        requiredPanels: NODE_IMPLEMENTATIONS[nodeType].getRequiredPanels(),
        requiredConsent: this._getConsentRequirement(nodeType),
        timeoutMs: this._getNodeTimeout(nodeType),
        retryPolicy: this._getRetryPolicy(nodeType),
        fallback: null
      };

      nodes.push(node);
      nodesByType[nodeType] = node;
    }

    // Build dependency graph based on BCTS state flow
    const stateFlow = proposalMapping.stateFlow;
    for (let i = 0; i < nodes.length - 1; i++) {
      dependencies[nodes[i].id] = [nodes[i + 1].id];
    }

    // Validate constitutional invariants
    const invariantChecks = this._validateInvariants(proposal, councilVerdict);

    // Build final workflow
    const workflow = {
      workflowId: `workflow_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`,
      proposalId: proposal.id,
      proposalType: proposal.type,
      councilVerdictId: councilVerdict.id,
      createdAt: Date.now(),
      status: 'COMPILED',
      nodes,
      dependencies,
      initialNode: nodes[0].id,
      stateFlow,
      bctsMappings: this._mapNodesToBCTS(nodes, stateFlow),
      invariantCoverage: invariantChecks,
      panelAssessments: councilVerdict.panelAssessments || {},
      executionPolicy: {
        parallelizable: this._canParallelize(nodes),
        faultTolerant: proposal.faultTolerant !== false,
        requiresHumanApproval: this._requiresHumanApproval(proposal.type),
        maxDuration: proposal.maxDuration || 3600000, // 1 hour default
        rollbackOnFailure: proposal.rollbackOnFailure !== false
      },
      metadata: {
        proposer: proposal.proposer,
        affectedPanels: proposalMapping.requiredPanels,
        consentRequired: proposal.requiresConsent !== false,
        estimatedCost: proposal.estimatedCost || 0
      }
    };

    return workflow;
  }

  /**
   * Validates a compiled workflow against the Syntaxis registry
   * @param {Object} workflow - The workflow to validate
   * @returns {Object} { valid: boolean, errors: string[] }
   */
  validateSyntaxisWorkflow(workflow) {
    const errors = [];

    if (!workflow.workflowId) {
      errors.push('workflowId is required');
    }

    if (!Array.isArray(workflow.nodes) || workflow.nodes.length === 0) {
      errors.push('workflow must contain at least one node');
    }

    if (!workflow.initialNode) {
      errors.push('initialNode must be specified');
    }

    // Validate each node
    for (const node of workflow.nodes || []) {
      const nodeImpl = NODE_IMPLEMENTATIONS[node.type];
      if (!nodeImpl) {
        errors.push(`Unknown node type: ${node.type}`);
        continue;
      }

      const validation = nodeImpl.validate(node.inputs);
      if (!validation.valid) {
        errors.push(`Node ${node.id}: ${validation.errors.join(', ')}`);
      }
    }

    // Validate dependencies
    for (const [nodeId, dependentIds] of Object.entries(workflow.dependencies || {})) {
      const nodeExists = workflow.nodes.some(n => n.id === nodeId);
      if (!nodeExists) {
        errors.push(`Dependency references unknown node: ${nodeId}`);
      }

      if (Array.isArray(dependentIds)) {
        for (const depId of dependentIds) {
          const depExists = workflow.nodes.some(n => n.id === depId);
          if (!depExists) {
            errors.push(`Dependency references unknown node: ${depId}`);
          }
        }
      }
    }

    // Validate state flow
    if (Array.isArray(workflow.stateFlow)) {
      for (let i = 0; i < workflow.stateFlow.length - 1; i++) {
        const currentState = workflow.stateFlow[i];
        const nextState = workflow.stateFlow[i + 1];
        const validTransitions = this.bctsTransitions[currentState] || [];

        if (!validTransitions.includes(nextState)) {
          errors.push(`Invalid state transition: ${currentState} -> ${nextState}`);
        }
      }
    }

    return {
      valid: errors.length === 0,
      errors,
      nodeCount: workflow.nodes?.length || 0,
      dependencyCount: Object.keys(workflow.dependencies || {}).length
    };
  }

  /**
   * Converts a Syntaxis workflow to Archon-compatible YAML DAG definition
   * @param {Object} syntaxisWorkflow - The compiled Syntaxis workflow
   * @returns {string} YAML DAG definition
   */
  syntaxisToArchonYaml(syntaxisWorkflow) {
    const yaml = [];

    // Header
    yaml.push('# Syntaxis Workflow to Archon DAG Conversion');
    yaml.push(`# Generated: ${new Date().toISOString()}`);
    yaml.push(`# Workflow ID: ${syntaxisWorkflow.workflowId}`);
    yaml.push('');

    // DAG definition
    yaml.push('dag:');
    yaml.push(`  id: "${syntaxisWorkflow.workflowId}"`);
    yaml.push(`  name: "${syntaxisWorkflow.proposalType}-${syntaxisWorkflow.proposalId}"`);
    yaml.push(`  status: "${syntaxisWorkflow.status}"`);
    yaml.push('');

    // Parameters
    yaml.push('parameters:');
    yaml.push(`  proposal_id: "${syntaxisWorkflow.proposalId}"`);
    yaml.push(`  proposal_type: "${syntaxisWorkflow.proposalType}"`);
    yaml.push(`  council_verdict_id: "${syntaxisWorkflow.councilVerdictId}"`);
    yaml.push('');

    // Tasks (nodes)
    yaml.push('tasks:');
    for (const node of syntaxisWorkflow.nodes) {
      yaml.push(`  - id: "${node.id}"`);
      yaml.push(`    type: "${node.type}"`);
      yaml.push(`    category: "${node.category}"`);
      yaml.push(`    timeout_ms: ${node.timeoutMs}`);
      yaml.push(`    required_panels: [${node.requiredPanels.map(p => `"${p}"`).join(', ')}]`);
      yaml.push(`    required_consent: ${node.requiredConsent}`);
      yaml.push('    inputs:');

      // Add inputs
      for (const [key, value] of Object.entries(node.inputs || {})) {
        if (typeof value === 'string') {
          yaml.push(`      ${key}: "${value}"`);
        } else if (typeof value === 'number' || typeof value === 'boolean') {
          yaml.push(`      ${key}: ${value}`);
        } else if (Array.isArray(value)) {
          yaml.push(`      ${key}: [${value.map(v => typeof v === 'string' ? `"${v}"` : v).join(', ')}]`);
        } else {
          yaml.push(`      ${key}: {}`);
        }
      }

      // Dependencies
      const deps = syntaxisWorkflow.dependencies[node.id] || [];
      if (deps.length > 0) {
        yaml.push(`    dependencies: [${deps.map(d => `"${d}"`).join(', ')}]`);
      }

      yaml.push('');
    }

    // Execution policy
    yaml.push('execution_policy:');
    yaml.push(`  parallelizable: ${syntaxisWorkflow.executionPolicy.parallelizable}`);
    yaml.push(`  fault_tolerant: ${syntaxisWorkflow.executionPolicy.faultTolerant}`);
    yaml.push(`  requires_human_approval: ${syntaxisWorkflow.executionPolicy.requiresHumanApproval}`);
    yaml.push(`  max_duration_ms: ${syntaxisWorkflow.executionPolicy.maxDuration}`);
    yaml.push(`  rollback_on_failure: ${syntaxisWorkflow.executionPolicy.rollbackOnFailure}`);
    yaml.push('');

    // Invariant coverage
    yaml.push('invariant_coverage:');
    for (const [invariant, coverage] of Object.entries(syntaxisWorkflow.invariantCoverage || {})) {
      yaml.push(`  ${invariant}: ${coverage.covered}`);
    }

    return yaml.join('\n');
  }

  /**
   * Private helper methods
   */

  _getCategoryForNodeType(nodeType) {
    const categoryMap = {
      'identity-verify': 'Identity & Access',
      'authority-check': 'Identity & Access',
      'authority-delegate': 'Identity & Access',
      'consent-request': 'Consent',
      'consent-verify': 'Consent',
      'consent-revoke': 'Consent',
      'governance-propose': 'Governance',
      'governance-vote': 'Governance',
      'governance-resolve': 'Governance',
      'kernel-adjudicate': 'Kernel',
      'invariant-check': 'Kernel',
      'proof-generate': 'Proof & Ledger',
      'proof-verify': 'Proof & Ledger',
      'dag-append': 'Proof & Ledger',
      'escalation-trigger': 'Escalation & Enforcement',
      'human-override': 'Escalation & Enforcement',
      'tenant-isolate': 'Multi-Tenancy & AI',
      'mcp-enforce': 'Multi-Tenancy & AI',
      'combinator-sequence': 'Flow Control',
      'combinator-parallel': 'Flow Control',
      'combinator-choice': 'Flow Control',
      'combinator-guard': 'Flow Control',
      'combinator-transform': 'Flow Control'
    };
    return categoryMap[nodeType] || 'Unknown';
  }

  _buildNodeInputs(nodeType, councilVerdict, proposal) {
    const baseInputs = {
      proposalId: proposal.id,
      verdictId: councilVerdict.id,
      timestamp: Date.now()
    };

    switch (nodeType) {
      case 'identity-verify':
        return {
          ...baseInputs,
          identity: { id: proposal.proposer },
          verificationMethod: 'cryptographic',
          nonce: Math.random().toString(36).substr(2, 9)
        };

      case 'authority-check':
        return {
          ...baseInputs,
          subjectId: proposal.proposer,
          requiredAuthority: 'GOVERNANCE_PROPOSER',
          scope: proposal.type
        };

      case 'authority-delegate':
        return {
          ...baseInputs,
          delegatorId: proposal.proposer,
          delegateeId: proposal.executor || proposal.proposer,
          authority: 'GOVERNANCE_EXECUTOR'
        };

      case 'consent-request':
        return {
          ...baseInputs,
          requesterId: proposal.proposer,
          consentType: proposal.consentType || 'STANDARD',
          recipientIds: councilVerdict.affectedPanels || [],
          consentData: proposal.consentData || {}
        };

      case 'consent-verify':
        return {
          ...baseInputs,
          consentRequestId: `consent_req_${proposal.id}`,
          recipientResponses: councilVerdict.consentResponses || {},
          requiredConsent: proposal.requiredConsentLevel || 0.8
        };

      case 'governance-propose':
        return {
          ...baseInputs,
          proposerId: proposal.proposer,
          proposalType: proposal.type,
          proposalContent: proposal.content || {},
          affectedPanels: councilVerdict.affectedPanels || []
        };

      case 'governance-vote':
        return {
          ...baseInputs,
          proposalId: proposal.id,
          panelVotes: councilVerdict.panelAssessments || {}
        };

      case 'governance-resolve':
        return {
          ...baseInputs,
          proposalId: proposal.id,
          voteResult: councilVerdict.status || 'PENDING',
          resolutionDetails: councilVerdict.resolution || {}
        };

      case 'kernel-adjudicate':
        return {
          ...baseInputs,
          conflictId: `conflict_${proposal.id}`,
          conflictType: 'GOVERNANCE_DISPUTE',
          evidenceProofs: proposal.evidence || []
        };

      case 'invariant-check':
        return {
          ...baseInputs,
          invariantId: `invariant_${proposal.type}`,
          invariantRule: { type: 'GOVERNANCE_RULE' },
          stateSnapshot: councilVerdict.systemState || {}
        };

      case 'proof-generate':
        return {
          ...baseInputs,
          dataHash: this._hashObject(proposal),
          prover: proposal.proposer,
          proofType: 'PROPOSAL_VALIDITY'
        };

      case 'proof-verify':
        return {
          ...baseInputs,
          proofId: `proof_${proposal.id}`,
          proofHash: this._hashObject(proposal),
          verifier: 'KERNEL_PANEL'
        };

      case 'dag-append':
        return {
          ...baseInputs,
          dagId: 'GOVERNANCE_DAG',
          nodeData: proposal,
          parentHashes: councilVerdict.precedingProposals || []
        };

      case 'escalation-trigger':
        return {
          ...baseInputs,
          escalationReason: proposal.escalationReason || 'GOVERNANCE_REVIEW_REQUIRED',
          escalationLevel: proposal.escalationLevel || 'WARNING',
          affectedComponent: proposal.type
        };

      case 'human-override':
        return {
          ...baseInputs,
          escalationId: `escalation_${proposal.id}`,
          overrideDecision: 'PROCEED',
          overridingAuthority: proposal.overseer || 'EXECUTIVE_PANEL',
          justification: proposal.justification || ''
        };

      case 'tenant-isolate':
        return {
          ...baseInputs,
          tenantId: proposal.tenantId || 'DEFAULT',
          isolationLevel: proposal.isolationLevel || 'LOGICAL',
          resourceScope: proposal.resourceScope || {}
        };

      case 'mcp-enforce':
        return {
          ...baseInputs,
          mcpInstanceId: proposal.mcpInstance || 'DEFAULT',
          enforcementPolicy: { type: 'GOVERNANCE_POLICY' },
          constraints: proposal.constraints || []
        };

      case 'combinator-sequence':
        return {
          ...baseInputs,
          steps: this._buildSequenceSteps(proposal),
          executionMode: 'STRICT'
        };

      case 'combinator-parallel':
        return {
          ...baseInputs,
          branches: proposal.parallelBranches || [],
          joinStrategy: 'ALL'
        };

      case 'combinator-choice':
        return {
          ...baseInputs,
          condition: proposal.condition || {},
          trueBranch: proposal.trueBranch || {},
          falseBranch: proposal.falseBranch || {}
        };

      case 'combinator-guard':
        return {
          ...baseInputs,
          guardCondition: { type: 'SAFETY_CHECK' },
          guardedAction: proposal.action || {},
          fallbackAction: proposal.fallback || {}
        };

      case 'combinator-transform':
        return {
          ...baseInputs,
          sourceData: proposal.sourceData || {},
          transformation: proposal.transformation || {},
          targetSchema: proposal.targetSchema || {}
        };

      default:
        return baseInputs;
    }
  }

  _getConsentRequirement(nodeType) {
    const requiresConsent = [
      'consent-request',
      'consent-verify',
      'authority-delegate',
      'governance-propose'
    ];
    return requiresConsent.includes(nodeType) ? 0.8 : 0;
  }

  _getNodeTimeout(nodeType) {
    const timeouts = {
      'identity-verify': 10000,
      'authority-check': 5000,
      'consent-verify': 60000,
      'governance-vote': 120000,
      'kernel-adjudicate': 30000,
      'proof-verify': 20000
    };
    return timeouts[nodeType] || 15000;
  }

  _getRetryPolicy(nodeType) {
    return {
      maxRetries: 3,
      backoffMs: 1000,
      exponentialBackoff: true
    };
  }

  _validateInvariants(proposal, verdict) {
    return {
      'GOVERNANCE_AUTHORITY': { covered: true, nodeId: 'authority-check' },
      'CONSENT_COVERAGE': { covered: true, nodeId: 'consent-verify' },
      'PROOF_VALIDITY': { covered: true, nodeId: 'proof-verify' },
      'KERNEL_INTEGRITY': { covered: true, nodeId: 'kernel-adjudicate' }
    };
  }

  _mapNodesToBCTS(nodes, stateFlow) {
    const mapping = {};
    for (let i = 0; i < nodes.length && i < stateFlow.length; i++) {
      mapping[nodes[i].id] = {
        currentState: stateFlow[i],
        nextState: stateFlow[i + 1] || 'COMPLETED'
      };
    }
    return mapping;
  }

  _canParallelize(nodes) {
    const flowControlTypes = [
      'combinator-parallel',
      'combinator-choice'
    ];
    return nodes.some(n => flowControlTypes.includes(n.type));
  }

  _requiresHumanApproval(proposalType) {
    const requiresApproval = [
      'governance-amendment',
      'infrastructure-change',
      'security-patch'
    ];
    return requiresApproval.includes(proposalType);
  }

  _buildSequenceSteps(proposal) {
    return proposal.steps || [
      { action: 'INITIALIZE' },
      { action: 'EXECUTE' },
      { action: 'FINALIZE' }
    ];
  }

  _hashObject(obj) {
    // Simple hash function for demonstration
    const str = JSON.stringify(obj);
    let hash = 0;
    for (let i = 0; i < str.length; i++) {
      const char = str.charCodeAt(i);
      hash = ((hash << 5) - hash) + char;
      hash = hash & hash;
    }
    return `0x${Math.abs(hash).toString(16)}`;
  }
}

module.exports = {
  SyntaxisCompiler,
  BCTS_TRANSITIONS,
  PROPOSAL_TYPE_MAPPINGS
};
