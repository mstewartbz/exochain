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

/** feedbackStore.ts — Zustand store for mandated reporter feedback system.
 *
 * Every widget has a "Report Issue" capability. Reports are filed as
 * structured issues that get triaged by agent teams, tracked through
 * resolution, and surfaced on the Council Tickets board.
 *
 * This is the "mandated reporter" pattern: any widget can flag a problem,
 * and the system MUST acknowledge, triage, and resolve it. No report
 * gets silently dropped.
 */

import { create } from 'zustand'
import {
  cacheDagDbDurableState,
  hydrateDagDbDurableState,
  persistDagDbDurableState,
  readCachedDagDbDurableState,
} from '../lib/dagdbDurableState'

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export type IssueSeverity = 'critical' | 'high' | 'medium' | 'low' | 'info'
export type IssueCategory = 'bug' | 'ux' | 'data' | 'performance' | 'security' | 'feature' | 'question'
export type IssueStatus = 'open' | 'triaged' | 'assigned' | 'in-progress' | 'resolved' | 'dismissed'

export interface FeedbackIssue {
  id: string
  /** Widget that filed the report */
  sourceWidgetId: string
  sourceModuleType: string
  /** Reporter */
  title: string
  description: string
  severity: IssueSeverity
  category: IssueCategory
  status: IssueStatus
  /** Auto-captured context */
  widgetState?: Record<string, unknown>
  browserInfo?: string
  /** Agent assignment */
  assignedAgentTeam?: string
  assignedAgentId?: string
  /** Resolution */
  resolution?: string
  resolvedAt?: number
  /** Timestamps */
  createdAt: number
  updatedAt: number
}

// ---------------------------------------------------------------------------
// DAG DB durable state
// ---------------------------------------------------------------------------

function loadIssues(): FeedbackIssue[] {
  return readCachedDagDbDurableState<FeedbackIssue[]>('feedback-issues', [])
}

function persistIssues(issues: FeedbackIssue[]) {
  cacheDagDbDurableState('feedback-issues', issues)
  void persistDagDbDurableState('feedback-issues', issues).catch(() => undefined)
}

async function hydrateIssues(): Promise<FeedbackIssue[]> {
  return hydrateDagDbDurableState<FeedbackIssue[]>('feedback-issues', [])
}

// ---------------------------------------------------------------------------
// Store
// ---------------------------------------------------------------------------

interface FeedbackState {
  issues: FeedbackIssue[]
  /** Currently open reporter slideout */
  reporterOpen: boolean
  reporterWidgetId: string | null
  reporterModuleType: string | null

  // Actions
  openReporter: (widgetId: string, moduleType: string) => void
  closeReporter: () => void
  fileIssue: (input: {
    title: string
    description: string
    severity: IssueSeverity
    category: IssueCategory
    widgetState?: Record<string, unknown>
  }) => FeedbackIssue
  updateIssueStatus: (id: string, status: IssueStatus, resolution?: string) => void
  dismissIssue: (id: string, reason: string) => void
  hydrateIssues: () => Promise<void>

  // Computed
  openIssueCount: () => number
  issuesForWidget: (widgetId: string) => FeedbackIssue[]
}

export const useFeedbackStore = create<FeedbackState>((set, get) => ({
  issues: loadIssues(),
  reporterOpen: false,
  reporterWidgetId: null,
  reporterModuleType: null,

  openReporter: (widgetId, moduleType) => {
    set({ reporterOpen: true, reporterWidgetId: widgetId, reporterModuleType: moduleType })
  },

  closeReporter: () => {
    set({ reporterOpen: false, reporterWidgetId: null, reporterModuleType: null })
  },

  fileIssue: (input) => {
    const { reporterWidgetId, reporterModuleType, issues } = get()
    const now = Date.now()
    const issue: FeedbackIssue = {
      id: `fb-${now}-${Math.random().toString(36).slice(2, 8)}`,
      sourceWidgetId: reporterWidgetId || 'unknown',
      sourceModuleType: reporterModuleType || 'unknown',
      title: input.title,
      description: input.description,
      severity: input.severity,
      category: input.category,
      status: 'open',
      widgetState: input.widgetState,
      browserInfo: navigator.userAgent,
      createdAt: now,
      updatedAt: now,
    }
    const next = [issue, ...issues]
    set({ issues: next, reporterOpen: false, reporterWidgetId: null, reporterModuleType: null })
    persistIssues(next)
    return issue
  },

  updateIssueStatus: (id, status, resolution) => {
    const { issues } = get()
    const now = Date.now()
    const next = issues.map(i =>
      i.id === id ? { ...i, status, resolution, updatedAt: now, ...(status === 'resolved' ? { resolvedAt: now } : {}) } : i
    )
    set({ issues: next })
    persistIssues(next)
  },

  dismissIssue: (id, reason) => {
    const { issues } = get()
    const now = Date.now()
    const next = issues.map(i =>
      i.id === id ? { ...i, status: 'dismissed' as IssueStatus, resolution: reason, updatedAt: now } : i
    )
    set({ issues: next })
    persistIssues(next)
  },

  hydrateIssues: async () => {
    const issues = await hydrateIssues()
    set({ issues })
  },

  openIssueCount: () => {
    return get().issues.filter(i => !['resolved', 'dismissed'].includes(i.status)).length
  },

  issuesForWidget: (widgetId) => {
    return get().issues.filter(i => i.sourceWidgetId === widgetId)
  },
}))
