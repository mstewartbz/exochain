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

//! Tenant usage metering, billing export, and subscription state.
//!
//! Per ratified decision D7 (2026-07-02): metering lives in isolated
//! `exo-tenant`, never in `exo-economy`. Metering OBSERVES tenant activity
//! and never gates trust or isolation. Usage totals reconcile against the
//! actual [`crate::store::TenantStore`] state rather than an independently
//! tracked counter that can drift. Aggregation windows use HLC
//! [`exo_core::Timestamp`], never host wall-clock. Invoices are deterministic:
//! the same usage history always yields the same billing statement. Metering
//! NEVER triggers settlement or a charge by default — a tenant only settles
//! if a [`BillingPlan`] explicitly opts into paid settlement
//! (HonorGood zero-fee metrology).

use std::collections::BTreeMap;

use exo_core::Timestamp;
use uuid::Uuid;

use crate::{error::Result, store::TenantStore};

/// The kind of billable activity a usage event records.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsageKind {
    /// Bytes written into the tenant store via `TenantStore::put`.
    BytesWritten,
    /// A single tenant-scoped API call.
    ApiCall,
}

/// A single recorded, tenant-scoped, HLC-timestamped billable event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsageEvent {
    pub tenant_id: Uuid,
    pub kind: UsageKind,
    /// Magnitude of the event: byte count for `BytesWritten`, 1 for `ApiCall`.
    pub amount: u64,
    /// HLC timestamp of the event — never host wall-clock.
    pub at: Timestamp,
}

/// An inclusive-exclusive HLC time window: `[start, end)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UsageWindow {
    pub start: Timestamp,
    pub end: Timestamp,
}

impl UsageWindow {
    /// Construct a new HLC window `[start, end)`.
    #[must_use]
    pub fn new(start: Timestamp, end: Timestamp) -> Self {
        Self { start, end }
    }

    /// Returns true if `ts` falls within `[start, end)` using HLC ordering.
    #[must_use]
    pub fn contains(&self, ts: &Timestamp) -> bool {
        *ts >= self.start && *ts < self.end
    }
}

/// Per-tenant usage totals for a given window.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UsageTotals {
    pub bytes_written: u64,
    pub api_calls: u64,
}

/// How a tenant's metered usage translates (or does not translate) into
/// settlement. Per D7, the default is `Observed`: metering never charges.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettlementMode {
    /// Zero-fee metrology: usage is observed and recorded, never charged.
    /// This MUST be the default for any tenant without an explicit plan.
    Observed,
    /// Explicit opt-in: the tenant's billing plan authorizes real settlement.
    PaidOptIn,
}

/// A tenant's billing plan. Absent a plan, a tenant is implicitly
/// `SettlementMode::Observed`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BillingPlan {
    pub tenant_id: Uuid,
    pub mode: SettlementMode,
}

/// Deterministic billing export for a tenant over a window: the same usage
/// history must always yield an identical invoice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invoice {
    pub tenant_id: Uuid,
    pub window: UsageWindow,
    pub totals: UsageTotals,
    /// Whether this invoice authorizes settlement. Always `false` unless a
    /// `BillingPlan` explicitly opted into `SettlementMode::PaidOptIn`.
    pub settlement_authorized: bool,
}

/// Records tenant usage events and reconciles them against actual
/// [`TenantStore`] state, aggregates over HLC windows, and produces
/// deterministic billing exports. Metering never gates the isolation/trust
/// path and never settles/charges by default.
#[derive(Debug, Default)]
pub struct UsageMeter {
    events: Vec<UsageEvent>,
    plans: BTreeMap<Uuid, BillingPlan>,
}

impl UsageMeter {
    /// Create an empty usage meter.
    #[must_use]
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            plans: BTreeMap::new(),
        }
    }

    /// Record a `BytesWritten` usage event for a tenant. Callers are expected
    /// to invoke this alongside `TenantStore::put` so the meter's totals
    /// reconcile against the store's actual stored bytes.
    pub fn record_bytes_written(
        &mut self,
        tenant_id: Uuid,
        amount: u64,
        at: Timestamp,
    ) -> Result<()> {
        self.events.push(UsageEvent {
            tenant_id,
            kind: UsageKind::BytesWritten,
            amount,
            at,
        });
        Ok(())
    }

    /// Record an `ApiCall` usage event for a tenant.
    pub fn record_api_call(&mut self, tenant_id: Uuid, at: Timestamp) -> Result<()> {
        self.events.push(UsageEvent {
            tenant_id,
            kind: UsageKind::ApiCall,
            amount: 1,
            at,
        });
        Ok(())
    }

    /// Aggregate this meter's recorded events for `tenant_id` within `window`,
    /// using HLC timestamp ordering (never host wall-clock).
    #[must_use]
    pub fn totals_in_window(&self, tenant_id: &Uuid, window: &UsageWindow) -> UsageTotals {
        let mut totals = UsageTotals::default();
        for event in &self.events {
            if event.tenant_id != *tenant_id || !window.contains(&event.at) {
                continue;
            }
            match event.kind {
                UsageKind::BytesWritten => totals.bytes_written += event.amount,
                UsageKind::ApiCall => totals.api_calls += event.amount,
            }
        }
        totals
    }

    /// Reconcile this meter's recorded byte totals for `tenant_id` against
    /// the actual items present in `store`: only the first `store.count`
    /// recorded `BytesWritten` events (in HLC order) for the tenant are
    /// credited, so a meter that has drifted ahead of what is really
    /// persisted cannot silently inflate the reconciled total. This is a
    /// reconciliation check against actual store state, not a second
    /// independently-tracked counter.
    pub fn reconcile_bytes_with_store(&self, tenant_id: &Uuid, store: &TenantStore) -> Result<u64> {
        let stored_item_count = store.count(tenant_id);

        let mut tenant_events: Vec<&UsageEvent> = self
            .events
            .iter()
            .filter(|e| e.tenant_id == *tenant_id && e.kind == UsageKind::BytesWritten)
            .collect();
        tenant_events.sort_by_key(|e| e.at);

        let reconciled: u64 = tenant_events
            .into_iter()
            .take(stored_item_count)
            .map(|e| e.amount)
            .sum();

        Ok(reconciled)
    }

    /// Register (or replace) a tenant's billing plan. Absent a call to this
    /// method, a tenant defaults to `SettlementMode::Observed`.
    pub fn set_billing_plan(&mut self, plan: BillingPlan) {
        self.plans.insert(plan.tenant_id, plan);
    }

    /// Look up the effective settlement mode for a tenant. Defaults to
    /// `SettlementMode::Observed` when no plan has been registered.
    #[must_use]
    pub fn settlement_mode(&self, tenant_id: &Uuid) -> SettlementMode {
        self.plans
            .get(tenant_id)
            .map_or(SettlementMode::Observed, |plan| plan.mode)
    }

    /// Produce a deterministic billing export (invoice) for `tenant_id` over
    /// `window`. The same usage history must always yield the identical
    /// invoice. `settlement_authorized` is only ever `true` when the
    /// tenant's billing plan is `SettlementMode::PaidOptIn`.
    #[must_use]
    pub fn invoice(&self, tenant_id: &Uuid, window: &UsageWindow) -> Invoice {
        let totals = self.totals_in_window(tenant_id, window);
        let settlement_authorized =
            matches!(self.settlement_mode(tenant_id), SettlementMode::PaidOptIn);
        Invoice {
            tenant_id: *tenant_id,
            window: *window,
            totals,
            settlement_authorized,
        }
    }
}

#[cfg(test)]
mod tests {
    use exo_core::{Did, Hash256};

    use super::*;
    use crate::store::TenantData;

    fn uuid(byte: u8) -> Uuid {
        Uuid::from_bytes([byte; 16])
    }

    fn ts(ms: u64) -> Timestamp {
        Timestamp::new(ms, 0)
    }

    fn item(tenant_id: Uuid, item_id: Uuid, tag: &str) -> TenantData {
        TenantData {
            id: item_id,
            tenant_id,
            owner: Did::new("did:exo:owner").unwrap(),
            content_hash: Hash256::digest(tag.as_bytes()),
        }
    }

    /// (1) Usage totals must reconcile against TenantStore's ACTUAL stored
    /// state across multiple tenants — not a counter that can silently
    /// drift away from what is really persisted.
    #[test]
    fn usage_meter_totals_match_tenant_store_state() {
        let mut store = TenantStore::new();
        let mut meter = UsageMeter::new();

        let t1 = uuid(1);
        let t2 = uuid(2);

        // Tenant 1: two writes of known sizes.
        let t1_sizes = [128u64, 256u64];
        for (i, size) in t1_sizes.iter().enumerate() {
            let item_id = uuid(10 + i as u8);
            store
                .put(t1, item(t1, item_id, &format!("t1-{i}")))
                .unwrap();
            meter
                .record_bytes_written(t1, *size, ts(1_000 + i as u64))
                .unwrap();
        }

        // Tenant 2: three writes of known sizes.
        let t2_sizes = [64u64, 32u64, 512u64];
        for (i, size) in t2_sizes.iter().enumerate() {
            let item_id = uuid(20 + i as u8);
            store
                .put(t2, item(t2, item_id, &format!("t2-{i}")))
                .unwrap();
            meter
                .record_bytes_written(t2, *size, ts(2_000 + i as u64))
                .unwrap();
        }

        // Reconciliation must confirm the meter's totals match what the
        // store actually holds for each tenant, and must report the byte
        // totals themselves (matching the sums above) — not merely "ok".
        let t1_reconciled = meter.reconcile_bytes_with_store(&t1, &store).unwrap();
        let t2_reconciled = meter.reconcile_bytes_with_store(&t2, &store).unwrap();

        assert_eq!(t1_reconciled, t1_sizes.iter().sum::<u64>());
        assert_eq!(t2_reconciled, t2_sizes.iter().sum::<u64>());

        // Store item counts must independently agree with what was written.
        assert_eq!(store.count(&t1), t1_sizes.len());
        assert_eq!(store.count(&t2), t2_sizes.len());

        // A tenant with no store activity and no meter records reconciles
        // to zero, not an error and not a stale nonzero value.
        let t3 = uuid(3);
        assert_eq!(meter.reconcile_bytes_with_store(&t3, &store).unwrap(), 0);
    }

    /// (2) Aggregation must use exo_core::Timestamp (HLC) ordering for the
    /// window boundaries, not host wall-clock — events strictly inside
    /// `[start, end)` aggregate; events outside do not.
    #[test]
    fn usage_aggregates_over_hlc_time_window() {
        let mut meter = UsageMeter::new();
        let tenant = uuid(1);

        // Before the window.
        meter.record_bytes_written(tenant, 100, ts(500)).unwrap();
        // Inside the window (inclusive start).
        meter.record_bytes_written(tenant, 200, ts(1_000)).unwrap();
        meter.record_api_call(tenant, ts(1_500)).unwrap();
        // Inside the window, right before the end boundary.
        meter.record_bytes_written(tenant, 300, ts(1_999)).unwrap();
        // At the exclusive end boundary — must NOT count.
        meter
            .record_bytes_written(tenant, 9_999, ts(2_000))
            .unwrap();
        // After the window.
        meter.record_api_call(tenant, ts(5_000)).unwrap();

        let window = UsageWindow::new(ts(1_000), ts(2_000));
        let totals = meter.totals_in_window(&tenant, &window);

        assert_eq!(totals.bytes_written, 200 + 300);
        assert_eq!(totals.api_calls, 1);

        // Sanity: the window itself must respect HLC contains() semantics
        // at both the inclusive start and exclusive end.
        assert!(window.contains(&ts(1_000)));
        assert!(!window.contains(&ts(2_000)));
        assert!(!window.contains(&ts(500)));
    }

    /// (3) Deterministic billing export: replaying the identical usage
    /// history twice must produce byte-for-byte identical invoices.
    #[test]
    fn invoice_is_deterministic() {
        let build_meter = || {
            let mut meter = UsageMeter::new();
            let tenant = uuid(7);
            meter.record_bytes_written(tenant, 111, ts(10)).unwrap();
            meter.record_api_call(tenant, ts(20)).unwrap();
            meter.record_bytes_written(tenant, 222, ts(30)).unwrap();
            meter
        };

        let tenant = uuid(7);
        let window = UsageWindow::new(ts(0), ts(1_000));

        let meter_a = build_meter();
        let meter_b = build_meter();

        let invoice_a = meter_a.invoice(&tenant, &window);
        let invoice_b = meter_b.invoice(&tenant, &window);

        assert_eq!(invoice_a, invoice_b);

        // Re-invoicing from the same meter again must also be stable.
        let invoice_a_again = meter_a.invoice(&tenant, &window);
        assert_eq!(invoice_a, invoice_a_again);
    }

    /// (4) Zero-fee metrology guard: metering produces usage records but
    /// triggers NO settlement or charge unless a billing plan explicitly
    /// opts into paid settlement.
    #[test]
    fn metering_does_not_settle_or_charge_by_default() {
        let mut meter = UsageMeter::new();
        let observed_tenant = uuid(1);
        let paid_tenant = uuid(2);

        meter
            .record_bytes_written(observed_tenant, 4_096, ts(10))
            .unwrap();
        meter
            .record_bytes_written(paid_tenant, 4_096, ts(10))
            .unwrap();

        let window = UsageWindow::new(ts(0), ts(1_000));

        // No plan registered at all -> must default to Observed, never charge.
        assert_eq!(
            meter.settlement_mode(&observed_tenant),
            SettlementMode::Observed
        );
        let observed_invoice = meter.invoice(&observed_tenant, &window);
        assert!(!observed_invoice.settlement_authorized);
        // Usage must still be recorded even though nothing settles.
        assert_eq!(observed_invoice.totals.bytes_written, 4_096);

        // Explicit opt-in plan -> settlement is authorized for that tenant.
        meter.set_billing_plan(BillingPlan {
            tenant_id: paid_tenant,
            mode: SettlementMode::PaidOptIn,
        });
        assert_eq!(
            meter.settlement_mode(&paid_tenant),
            SettlementMode::PaidOptIn
        );
        let paid_invoice = meter.invoice(&paid_tenant, &window);
        assert!(paid_invoice.settlement_authorized);

        // The observed tenant remains unaffected by the other tenant's plan.
        assert_eq!(
            meter.settlement_mode(&observed_tenant),
            SettlementMode::Observed
        );
        assert!(
            !meter
                .invoice(&observed_tenant, &window)
                .settlement_authorized
        );
    }
}
