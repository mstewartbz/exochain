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

/** layoutTemplateStore.ts — Zustand store for grid layout templates.
 *
 * Manages: template CRUD, edit mode with draft state, DAG DB-backed durable
 * persistence, and forward-compatible layout merging when new panels are
 * added to the registry.
 */

import { create } from 'zustand'
import type { LayoutItem } from '../data/defaultLayouts'
import {
  BUILTIN_TEMPLATES,
  BUILTIN_TEMPLATES as builtins,
  mergeLayoutWithDefaults,
  type LayoutTemplate,
} from '../data/defaultLayouts'
import {
  cacheDagDbDurableState,
  hydrateDagDbDurableState,
  persistDagDbDurableState,
  readCachedDagDbDurableState,
} from '../lib/dagdbDurableState'

// ---------------------------------------------------------------------------
// DAG DB durable state helpers
// ---------------------------------------------------------------------------

type PersistedLayoutTemplateState = {
  templates: LayoutTemplate[]
  activeId: string
}

function normalizePersistedLayoutState(
  persisted: PersistedLayoutTemplateState,
): { templates: LayoutTemplate[]; activeId: string } {
  const userTemplates = Array.isArray(persisted.templates)
    ? persisted.templates.filter(t => !t.isBuiltIn)
    : []
  const activeId = typeof persisted.activeId === 'string' && persisted.activeId
    ? persisted.activeId
    : 'builtin-default'
  return { templates: [...builtins, ...userTemplates], activeId }
}

function loadDurableLayoutState(): { templates: LayoutTemplate[]; activeId: string } {
  return normalizePersistedLayoutState(
    readCachedDagDbDurableState<PersistedLayoutTemplateState>('layout-templates', {
      templates: [],
      activeId: 'builtin-default',
    }),
  )
}

function persistedLayoutState(
  templates: LayoutTemplate[],
  activeId: string,
): PersistedLayoutTemplateState {
  return {
    templates: templates.filter(t => !t.isBuiltIn),
    activeId,
  }
}

function persistLayoutState(templates: LayoutTemplate[], activeId: string) {
  const persisted = persistedLayoutState(templates, activeId)
  cacheDagDbDurableState('layout-templates', persisted)
  void persistDagDbDurableState('layout-templates', persisted).catch(() => undefined)
}

async function hydrateLayoutState(): Promise<{ templates: LayoutTemplate[]; activeId: string }> {
  const persisted = await hydrateDagDbDurableState<PersistedLayoutTemplateState>(
    'layout-templates',
    { templates: [], activeId: 'builtin-default' },
  )
  return normalizePersistedLayoutState(persisted)
}

// ---------------------------------------------------------------------------
// Store
// ---------------------------------------------------------------------------

interface LayoutTemplateState {
  templates: LayoutTemplate[]
  activeTemplateId: string
  editMode: boolean
  draftLayout: LayoutItem[] | null
  draftHiddenPanels: string[] | null

  // Actions
  selectTemplate: (id: string) => void
  setEditMode: (on: boolean) => void
  updateDraftLayout: (layout: LayoutItem[]) => void
  togglePanelVisibility: (panelId: string) => void
  saveTemplate: () => void
  saveAsTemplate: (name: string) => string
  renameTemplate: (id: string, name: string) => void
  deleteTemplate: (id: string) => void
  duplicateTemplate: (id: string, newName: string) => string
  hydrateFromDurableState: () => Promise<void>

  // Computed
  getActiveTemplate: () => LayoutTemplate
  getEffectiveLayout: () => LayoutItem[]
  getEffectiveHiddenPanels: () => string[]
}

const initial = loadDurableLayoutState()

export const useLayoutTemplateStore = create<LayoutTemplateState>((set, get) => ({
  templates: initial.templates,
  activeTemplateId: initial.activeId,
  editMode: false,
  draftLayout: null,
  draftHiddenPanels: null,

  // ── Select template ──────────────────────────────────────────
  selectTemplate: (id) => {
    if (get().editMode) return // blocked while editing
    set({ activeTemplateId: id })
    persistLayoutState(get().templates, id)
  },

  // ── Edit mode ────────────────────────────────────────────────
  setEditMode: (on) => {
    if (on) {
      const active = get().getActiveTemplate()
      const defaultLayout = BUILTIN_TEMPLATES[0].layout
      set({
        editMode: true,
        draftLayout: mergeLayoutWithDefaults([...active.layout], defaultLayout),
        draftHiddenPanels: [...active.hiddenPanels],
      })
    } else {
      // Discard draft
      set({ editMode: false, draftLayout: null, draftHiddenPanels: null })
    }
  },

  // ── Draft mutations (edit mode only) ─────────────────────────
  updateDraftLayout: (layout) => {
    if (!get().editMode) return
    set({ draftLayout: layout })
  },

  togglePanelVisibility: (panelId) => {
    if (!get().editMode) return
    const current = get().draftHiddenPanels || []
    const next = current.includes(panelId)
      ? current.filter(id => id !== panelId)
      : [...current, panelId]
    set({ draftHiddenPanels: next })
  },

  // ── Save (overwrite current user template) ───────────────────
  saveTemplate: () => {
    const { activeTemplateId, draftLayout, draftHiddenPanels, templates } = get()
    const active = templates.find(t => t.id === activeTemplateId)
    if (!active || active.isBuiltIn || !draftLayout || !draftHiddenPanels) return

    const now = Date.now()
    const updated: LayoutTemplate = {
      ...active,
      layout: draftLayout,
      hiddenPanels: draftHiddenPanels,
      updatedAt: now,
    }
    const next = templates.map(t => t.id === active.id ? updated : t)
    set({ templates: next, editMode: false, draftLayout: null, draftHiddenPanels: null })
    persistLayoutState(next, activeTemplateId)
  },

  // ── Save As (create new template from draft) ─────────────────
  saveAsTemplate: (name) => {
    const { draftLayout, draftHiddenPanels, templates } = get()
    if (!draftLayout || !draftHiddenPanels) return ''

    const now = Date.now()
    const id = `user-${now}-${Math.random().toString(36).slice(2, 8)}`
    const newTemplate: LayoutTemplate = {
      id,
      name,
      layout: draftLayout,
      hiddenPanels: draftHiddenPanels,
      isBuiltIn: false,
      createdAt: now,
      updatedAt: now,
    }
    const next = [...templates, newTemplate]
    set({
      templates: next,
      activeTemplateId: id,
      editMode: false,
      draftLayout: null,
      draftHiddenPanels: null,
    })
    persistLayoutState(next, id)
    return id
  },

  // ── Rename ───────────────────────────────────────────────────
  renameTemplate: (id, name) => {
    const { templates } = get()
    const t = templates.find(t => t.id === id)
    if (!t || t.isBuiltIn) return
    const updated = { ...t, name, updatedAt: Date.now() }
    const next = templates.map(x => x.id === id ? updated : x)
    set({ templates: next })
    persistLayoutState(next, get().activeTemplateId)
  },

  // ── Delete ───────────────────────────────────────────────────
  deleteTemplate: (id) => {
    const { templates, activeTemplateId } = get()
    const t = templates.find(t => t.id === id)
    if (!t || t.isBuiltIn) return
    const next = templates.filter(t => t.id !== id)
    const newActive = activeTemplateId === id ? 'builtin-default' : activeTemplateId
    set({ templates: next, activeTemplateId: newActive })
    persistLayoutState(next, newActive)
  },

  // ── Duplicate ────────────────────────────────────────────────
  duplicateTemplate: (id, newName) => {
    const { templates } = get()
    const source = templates.find(t => t.id === id)
    if (!source) return ''
    const now = Date.now()
    const newId = `user-${now}-${Math.random().toString(36).slice(2, 8)}`
    const dup: LayoutTemplate = {
      ...source,
      id: newId,
      name: newName,
      isBuiltIn: false,
      createdAt: now,
      updatedAt: now,
    }
    const next = [...templates, dup]
    set({ templates: next, activeTemplateId: newId })
    persistLayoutState(next, newId)
    return newId
  },

  hydrateFromDurableState: async () => {
    const hydrated = await hydrateLayoutState()
    set({
      templates: hydrated.templates,
      activeTemplateId: hydrated.activeId,
      editMode: false,
      draftLayout: null,
      draftHiddenPanels: null,
    })
  },

  // ── Computed ─────────────────────────────────────────────────
  getActiveTemplate: () => {
    const { templates, activeTemplateId } = get()
    return templates.find(t => t.id === activeTemplateId) || templates[0]
  },

  getEffectiveLayout: () => {
    const { editMode, draftLayout } = get()
    if (editMode && draftLayout) return draftLayout
    const defaultLayout = BUILTIN_TEMPLATES[0].layout
    return mergeLayoutWithDefaults(get().getActiveTemplate().layout, defaultLayout)
  },

  getEffectiveHiddenPanels: () => {
    const { editMode, draftHiddenPanels } = get()
    if (editMode && draftHiddenPanels) return draftHiddenPanels
    return get().getActiveTemplate().hiddenPanels
  },
}))
