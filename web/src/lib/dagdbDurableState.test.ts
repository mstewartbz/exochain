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

import { readFileSync } from 'node:fs'
import path from 'node:path'
import { describe, expect, it } from 'vitest'

const webRoot = path.resolve(__dirname, '../..')

function source(relPath: string): string {
  return readFileSync(path.join(webRoot, relPath), 'utf8')
}

describe('DAG DB durable web state contract', () => {
  it('keeps durable product state off browser localStorage keys', () => {
    const durableSource = [
      'src/lib/council.ts',
      'src/stores/feedbackStore.ts',
      'src/stores/layoutTemplateStore.ts',
      'src/pages/APE/OnboardPage.tsx',
      'src/pages/APE/APEDashboardPage.tsx',
      'src/lib/auth.tsx',
    ].map(source).join('\n')

    expect(durableSource).not.toMatch(/df_council_tickets|df_council_conversations/)
    expect(durableSource).not.toMatch(/exo_feedback_issues/)
    expect(durableSource).not.toMatch(/exo_layout_templates|exo_active_template_id/)
    expect(durableSource).not.toMatch(/ape_onboarding/)
  })

  it('uses the DAG DB intake adapter for durable product state families', () => {
    const adapter = source('src/lib/dagdbDurableState.ts')

    expect(adapter).toMatch(/\/api\/v1\/dag-db\/intake/)
    expect(adapter).toMatch(/web_durable_state_result/)
    expect(adapter).toMatch(/x-exo-authority-scope/)
    expect(adapter).toMatch(/council-tickets/)
    expect(adapter).toMatch(/council-conversations/)
    expect(adapter).toMatch(/feedback-issues/)
    expect(adapter).toMatch(/layout-templates/)
    expect(adapter).toMatch(/ape-onboarding/)
  })
})
