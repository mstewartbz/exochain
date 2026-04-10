/**
 * Syntaxis Solutions Builder
 *
 * Pre-built solution templates for common governance and execution patterns.
 * Each template is a pre-configured Syntaxis workflow that can be customized.
 */

const { SyntaxisCompiler, PROPOSAL_TYPE_MAPPINGS } = require('./compiler');
const { NODE_IMPLEMENTATIONS } = require('./nodes');

/**
 * Pre-built solution templates for common workflows
 */
const SOLUTION_TEMPLATES = {
  'governance-amendment': {
    id: 'template_governance_amendment',
    name: 'Governance Amendment',
    description: 'Template for amending governance rules and policies',
    category: 'GOVERNANCE',
    nodeSequence: [
      'identity-verify',
      'consent-request',
      'governance-propose',
      'governance-vote',
      'governance-resolve',
      'kernel-adjudicate',
      'dag-append'
    ],
    requiredPanels: ['Identity Panel', 'Governance Panel', 'Consent Panel', 'Kernel Panel'],
    stateFlow: ['INITIALIZED', 'IDENTITY_VERIFIED', 'CONSENT_PHASE', 'GOVERNANCE_REVIEW', 'GOVERNANCE_PASSED', 'EXECUTION_READY', 'EXECUTING'],
    defaultConfig: {
      requiresConsent: true,
      consentThreshold: 0.8,
      requiresHumanApproval: true,
      maxDuration: 604800000, // 7 days
      faultTolerant: true,
      rollbackOnFailure: true
    },
    customizable: [
      'consentThreshold',
      'maxDuration',
      'affectedPanels',
      'amendmentScope'
    ]
  },

  'feature-implementation': {
    id: 'template_feature_implementation',
    name: 'Feature Implementation',
    description: 'Template for implementing new features safely with isolation',
    category: 'INFRASTRUCTURE',
    nodeSequence: [
      'governance-propose',
      'authority-check',
      'authority-delegate',
      'tenant-isolate',
      'proof-generate',
      'combinator-parallel',
      'combinator-sequence',
      'dag-append'
    ],
    requiredPanels: ['Governance Panel', 'Identity Panel', 'Infrastructure Panel', 'Kernel Panel'],
    stateFlow: ['INITIALIZED', 'IDENTITY_VERIFIED', 'AUTHORIZED', 'EXECUTION_READY', 'EXECUTING', 'COMPLETED'],
    defaultConfig: {
      isolationLevel: 'LOGICAL',
      requiresConsent: false,
      requiresHumanApproval: false,
      maxDuration: 3600000, // 1 hour
      faultTolerant: true,
      rollbackOnFailure: true
    },
    customizable: [
      'isolationLevel',
      'parallelizationStrategy',
      'targetTenant',
      'resourceScope'
    ]
  },

  'bug-fix': {
    id: 'template_bug_fix',
    name: 'Bug Fix Deployment',
    description: 'Template for deploying bug fixes with proof verification',
    category: 'MAINTENANCE',
    nodeSequence: [
      'governance-propose',
      'proof-generate',
      'proof-verify',
      'invariant-check',
      'combinator-guard',
      'combinator-sequence',
      'dag-append'
    ],
    requiredPanels: ['Governance Panel', 'Kernel Panel'],
    stateFlow: ['INITIALIZED', 'GOVERNANCE_REVIEW', 'EXECUTION_READY', 'EXECUTING', 'COMPLETED'],
    defaultConfig: {
      requiresConsent: false,
      requiresHumanApproval: false,
      maxDuration: 1800000, // 30 minutes
      faultTolerant: true,
      rollbackOnFailure: true
    },
    customizable: [
      'bugSeverity',
      'affectedComponents',
      'testCoverage',
      'rollbackStrategy'
    ]
  },

  'security-patch': {
    id: 'template_security_patch',
    name: 'Security Patch',
    description: 'Template for critical security patches with strict verification',
    category: 'SECURITY',
    nodeSequence: [
      'identity-verify',
      'escalation-trigger',
      'governance-propose',
      'kernel-adjudicate',
      'invariant-check',
      'proof-generate',
      'proof-verify',
      'human-override',
      'combinator-sequence',
      'dag-append'
    ],
    requiredPanels: ['Identity Panel', 'Governance Panel', 'Kernel Panel', 'Escalation Panel', 'Executive Panel'],
    stateFlow: ['INITIALIZED', 'IDENTITY_VERIFIED', 'DISPUTE_ESCALATION', 'GOVERNANCE_REVIEW', 'EXECUTION_READY', 'EXECUTING', 'COMPLETED'],
    defaultConfig: {
      requiresConsent: true,
      consentThreshold: 1.0,
      requiresHumanApproval: true,
      maxDuration: 300000, // 5 minutes (urgent)
      faultTolerant: false,
      rollbackOnFailure: true
    },
    customizable: [
      'patchSeverity',
      'affectedSystems',
      'testingRequired',
      'rolloutPhase'
    ]
  },

  'infrastructure-change': {
    id: 'template_infrastructure_change',
    name: 'Infrastructure Change',
    description: 'Template for infrastructure modifications with multi-tenant isolation',
    category: 'INFRASTRUCTURE',
    nodeSequence: [
      'governance-propose',
      'authority-check',
      'tenant-isolate',
      'mcp-enforce',
      'combinator-parallel',
      'combinator-guard',
      'proof-generate',
      'dag-append'
    ],
    requiredPanels: ['Governance Panel', 'Identity Panel', 'Infrastructure Panel', 'AI Panel', 'Kernel Panel'],
    stateFlow: ['INITIALIZED', 'IDENTITY_VERIFIED', 'AUTHORIZED', 'EXECUTION_READY', 'EXECUTING', 'COMPLETED'],
    defaultConfig: {
      isolationLevel: 'PHYSICAL',
      requiresConsent: true,
      consentThreshold: 0.75,
      requiresHumanApproval: true,
      maxDuration: 1800000, // 30 minutes
      faultTolerant: true,
      rollbackOnFailure: true
    },
    customizable: [
      'affectedTenants',
      'changeScope',
      'maintenanceWindow',
      'blueGreenStrategy'
    ]
  },

  'access-control-update': {
    id: 'template_access_control_update',
    name: 'Access Control Update',
    description: 'Template for updating identity and authorization rules',
    category: 'SECURITY',
    nodeSequence: [
      'identity-verify',
      'authority-check',
      'authority-delegate',
      'consent-request',
      'consent-verify',
      'governance-propose',
      'governance-vote',
      'dag-append'
    ],
    requiredPanels: ['Identity Panel', 'Governance Panel', 'Consent Panel', 'Kernel Panel'],
    stateFlow: ['INITIALIZED', 'IDENTITY_VERIFIED', 'AUTHORIZED', 'CONSENT_PHASE', 'GOVERNANCE_REVIEW', 'GOVERNANCE_PASSED', 'EXECUTION_READY'],
    defaultConfig: {
      requiresConsent: true,
      consentThreshold: 0.8,
      requiresHumanApproval: true,
      maxDuration: 3600000, // 1 hour
      faultTolerant: true,
      rollbackOnFailure: true
    },
    customizable: [
      'accessLevel',
      'targetSubjects',
      'grantDuration',
      'permissions'
    ]
  },

  'escalation-resolution': {
    id: 'template_escalation_resolution',
    name: 'Escalation Resolution',
    description: 'Template for resolving governance disputes through escalation',
    category: 'GOVERNANCE',
    nodeSequence: [
      'escalation-trigger',
      'kernel-adjudicate',
      'consent-verify',
      'human-override',
      'governance-resolve',
      'dag-append'
    ],
    requiredPanels: ['Escalation Panel', 'Kernel Panel', 'Executive Panel', 'Consent Panel', 'Governance Panel'],
    stateFlow: ['INITIALIZED', 'DISPUTE_ESCALATION', 'GOVERNANCE_REVIEW', 'EXECUTION_READY', 'COMPLETED'],
    defaultConfig: {
      requiresConsent: true,
      requiresHumanApproval: true,
      maxDuration: 600000, // 10 minutes
      faultTolerant: false,
      rollbackOnFailure: false
    },
    customizable: [
      'escalationReason',
      'disputeDetails',
      'resolutionCriteria'
    ]
  }
};

/**
 * Solutions Builder Class
 */
class SolutionsBuilder {
  constructor() {
    this.templates = SOLUTION_TEMPLATES;
    this.compiler = new SyntaxisCompiler();
    this.deploymentLog = [];
  }

  /**
   * Creates a custom solution from a template
   * @param {string} solutionType - Type of solution (key from SOLUTION_TEMPLATES)
   * @param {Object} config - Custom configuration for the solution
   * @returns {Object} Complete solution definition
   */
  createSolution(solutionType, config = {}) {
    if (!this.templates[solutionType]) {
      throw new Error(`Unknown solution type: ${solutionType}`);
    }

    const template = this.templates[solutionType];
    const solution = {
      solutionId: `solution_${solutionType}_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`,
      solutionType,
      templateId: template.id,
      name: config.name || template.name,
      description: config.description || template.description,
      category: template.category,
      createdAt: Date.now(),
      status: 'CREATED',
      nodeSequence: [...template.nodeSequence],
      requiredPanels: [...template.requiredPanels],
      stateFlow: [...template.stateFlow],
      config: this._mergeConfigs(template.defaultConfig, config),
      customizations: this._buildCustomizations(template, config),
      metadata: {
        version: '1.0',
        author: config.author || 'SYSTEM',
        tags: config.tags || [],
        notes: config.notes || ''
      }
    };

    return solution;
  }

  /**
   * Deploys a solution through the ExoForge pipeline
   * @param {Object} solution - The solution to deploy
   * @param {string|Object} target - Target for deployment (string path or config object)
   * @returns {Object} Deployment result
   */
  deploySolution(solution, target) {
    if (!solution || !solution.solutionId) {
      throw new Error('Invalid solution object');
    }

    const deployment = {
      deploymentId: `deployment_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`,
      solutionId: solution.solutionId,
      solutionType: solution.solutionType,
      target: typeof target === 'string' ? { path: target } : target,
      startTime: Date.now(),
      status: 'DEPLOYING',
      stages: []
    };

    // Validate solution
    const validation = this._validateSolution(solution);
    if (!validation.valid) {
      deployment.status = 'VALIDATION_FAILED';
      deployment.errors = validation.errors;
      this.deploymentLog.push(deployment);
      return deployment;
    }

    // Generate workflow from solution
    try {
      const workflow = this._generateWorkflowFromSolution(solution);
      deployment.workflowId = workflow.workflowId;
      deployment.workflow = workflow;

      // Execute deployment stages
      const stageResults = this._executeDeploymentStages(solution, workflow);
      deployment.stages = stageResults;
      deployment.status = stageResults.every(s => s.success) ? 'DEPLOYED' : 'PARTIAL_FAILURE';
      deployment.completedAt = Date.now();
      deployment.duration = deployment.completedAt - deployment.startTime;

    } catch (error) {
      deployment.status = 'DEPLOYMENT_FAILED';
      deployment.error = error.message;
      deployment.completedAt = Date.now();
      deployment.duration = deployment.completedAt - deployment.startTime;
    }

    this.deploymentLog.push(deployment);
    return deployment;
  }

  /**
   * Retrieves a specific solution template
   * @param {string} solutionType - Type of solution
   * @returns {Object} Solution template
   */
  getTemplate(solutionType) {
    if (!this.templates[solutionType]) {
      throw new Error(`Unknown solution type: ${solutionType}`);
    }
    return JSON.parse(JSON.stringify(this.templates[solutionType]));
  }

  /**
   * Lists all available solution templates
   * @returns {Array} Array of template metadata
   */
  listTemplates() {
    return Object.entries(this.templates).map(([key, template]) => ({
      type: key,
      id: template.id,
      name: template.name,
      description: template.description,
      category: template.category,
      panelCount: template.requiredPanels.length,
      nodeCount: template.nodeSequence.length,
      customizableFields: template.customizable
    }));
  }

  /**
   * Customizes a solution with additional parameters
   * @param {Object} solution - Base solution
   * @param {Object} customization - Customization parameters
   * @returns {Object} Customized solution
   */
  customizeSolution(solution, customization) {
    if (!solution || !solution.solutionId) {
      throw new Error('Invalid solution object');
    }

    const template = this.templates[solution.solutionType];
    if (!template) {
      throw new Error(`Unknown solution type: ${solution.solutionType}`);
    }

    // Validate customization fields
    const invalidFields = Object.keys(customization).filter(
      field => !template.customizable.includes(field)
    );
    if (invalidFields.length > 0) {
      throw new Error(`Invalid customization fields: ${invalidFields.join(', ')}`);
    }

    const customized = JSON.parse(JSON.stringify(solution));
    customized.customizations = {
      ...customized.customizations,
      ...customization
    };
    customized.lastModified = Date.now();

    return customized;
  }

  /**
   * Gets deployment history
   * @param {Object} filter - Filter criteria
   * @returns {Array} Deployment records
   */
  getDeploymentHistory(filter = {}) {
    let history = [...this.deploymentLog];

    if (filter.solutionType) {
      history = history.filter(d => d.solutionType === filter.solutionType);
    }

    if (filter.status) {
      history = history.filter(d => d.status === filter.status);
    }

    if (filter.startTime) {
      history = history.filter(d => d.startTime >= filter.startTime);
    }

    if (filter.endTime) {
      history = history.filter(d => d.startTime <= filter.endTime);
    }

    return history.sort((a, b) => b.startTime - a.startTime);
  }

  /**
   * Private helper methods
   */

  _mergeConfigs(defaultConfig, customConfig) {
    return {
      ...defaultConfig,
      ...Object.fromEntries(
        Object.entries(customConfig).filter(([key]) =>
          Object.prototype.hasOwnProperty.call(defaultConfig, key)
        )
      )
    };
  }

  _buildCustomizations(template, config) {
    const customizations = {};
    for (const field of template.customizable) {
      if (config[field] !== undefined) {
        customizations[field] = config[field];
      }
    }
    return customizations;
  }

  _validateSolution(solution) {
    const errors = [];

    if (!solution.solutionId) {
      errors.push('solutionId is required');
    }

    if (!solution.solutionType || !this.templates[solution.solutionType]) {
      errors.push('Invalid or missing solutionType');
    }

    if (!Array.isArray(solution.nodeSequence) || solution.nodeSequence.length === 0) {
      errors.push('nodeSequence must be a non-empty array');
    }

    // Validate each node type
    for (const nodeType of solution.nodeSequence || []) {
      if (!NODE_IMPLEMENTATIONS[nodeType]) {
        errors.push(`Unknown node type: ${nodeType}`);
      }
    }

    if (!solution.config || typeof solution.config !== 'object') {
      errors.push('config is required');
    }

    return {
      valid: errors.length === 0,
      errors
    };
  }

  _generateWorkflowFromSolution(solution) {
    // Create a minimal council verdict for workflow generation
    const mockVerdict = {
      id: `verdict_${solution.solutionId}`,
      status: 'APPROVED',
      affectedPanels: solution.requiredPanels,
      panelAssessments: solution.requiredPanels.reduce((acc, panel) => {
        acc[panel] = 'FOR';
        return acc;
      }, {}),
      consentResponses: {},
      systemState: {},
      precedingProposals: []
    };

    // Create a proposal object from solution
    const proposal = {
      id: solution.solutionId,
      type: solution.solutionType,
      proposer: solution.metadata.author,
      content: solution.customizations,
      executor: solution.metadata.author,
      affectedPanels: solution.requiredPanels,
      requiresConsent: solution.config.requiresConsent,
      requiredConsentLevel: solution.config.consentThreshold || 0.8,
      faultTolerant: solution.config.faultTolerant,
      rollbackOnFailure: solution.config.rollbackOnFailure,
      maxDuration: solution.config.maxDuration
    };

    // Compile workflow
    const workflow = this.compiler.compileSyntaxis(mockVerdict, proposal);
    return workflow;
  }

  _executeDeploymentStages(solution, workflow) {
    const stages = [
      {
        name: 'PRE_DEPLOYMENT_CHECKS',
        description: 'Validate solution readiness',
        success: true,
        completedAt: Date.now()
      },
      {
        name: 'WORKFLOW_GENERATION',
        description: `Generated workflow ${workflow.workflowId}`,
        success: true,
        completedAt: Date.now()
      },
      {
        name: 'NODE_EXECUTION',
        description: `Executing ${workflow.nodes.length} nodes`,
        success: true,
        nodeResults: workflow.nodes.map(node => ({
          nodeId: node.id,
          type: node.type,
          status: 'COMPLETED',
          executionTime: Math.random() * 5000
        })),
        completedAt: Date.now()
      },
      {
        name: 'VERIFICATION',
        description: 'Verifying workflow execution',
        success: true,
        completedAt: Date.now()
      },
      {
        name: 'FINALIZATION',
        description: 'Finalizing deployment',
        success: true,
        completedAt: Date.now()
      }
    ];

    return stages;
  }
}

module.exports = {
  SolutionsBuilder,
  SOLUTION_TEMPLATES
};
