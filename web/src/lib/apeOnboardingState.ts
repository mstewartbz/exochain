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

import {
  cacheDagDbDurableState,
  hydrateDagDbDurableState,
  persistDagDbDurableState,
  readCachedDagDbDurableState,
} from './dagdbDurableState'

export interface ApeBoardMember {
  id: string
  title: string
  shortTitle: string
  icon: string
  description: string
  capabilities: string[]
  decisionClass: string
}

export interface ApeOnboardingData {
  displayName: string
  email: string
  boardName: string
  governanceStyle: string
  boardMembers: ApeBoardMember[]
  createdAt: string
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

export function isApeOnboardingData(value: unknown): value is ApeOnboardingData {
  if (!isRecord(value)) return false
  return (
    typeof value.displayName === 'string' &&
    typeof value.email === 'string' &&
    typeof value.boardName === 'string' &&
    typeof value.governanceStyle === 'string' &&
    typeof value.createdAt === 'string' &&
    Array.isArray(value.boardMembers)
  )
}

export function persistApeOnboarding(data: ApeOnboardingData): void {
  cacheDagDbDurableState('ape-onboarding', data)
  void persistDagDbDurableState('ape-onboarding', data).catch(() => undefined)
}

export function loadCachedApeOnboarding(): ApeOnboardingData | null {
  const value = readCachedDagDbDurableState<unknown>('ape-onboarding', null)
  return isApeOnboardingData(value) ? value : null
}

export async function hydrateApeOnboarding(): Promise<ApeOnboardingData | null> {
  const value = await hydrateDagDbDurableState<unknown>('ape-onboarding', null)
  return isApeOnboardingData(value) ? value : null
}
