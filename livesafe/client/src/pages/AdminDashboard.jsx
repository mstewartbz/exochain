import React, { useState, useEffect, useRef } from 'react';
import { useNavigate } from 'react-router-dom';
import { useAuth } from '../context/AuthContext';
import api from '../services/api';

function AdminDashboard() {
  const { user, logout, startViewAsRole } = useAuth();
  const navigate = useNavigate();

  const [stats, setStats] = useState(null);
  const [subscribers, setSubscribers] = useState([]);
  const [total, setTotal] = useState(0);
  const [page, setPage] = useState(1);
  const [search, setSearch] = useState('');
  const [searchInput, setSearchInput] = useState('');
  const [loading, setLoading] = useState(true);
  const [statsLoading, setStatsLoading] = useState(true);
  const [error, setError] = useState('');
  const [updateMsg, setUpdateMsg] = useState('');
  const [viewAsMsg, setViewAsMsg] = useState('');
  const [auditRecords, setAuditRecords] = useState([]);
  const [activeTab, setActiveTab] = useState('accounts');
  const [modifyAttempt, setModifyAttempt] = useState(null);
  // Agency management state (Feature #235)
  const [agencies, setAgencies] = useState([]);
  const [agenciesLoading, setAgenciesLoading] = useState(false);
  const [agencyMsg, setAgencyMsg] = useState('');
  // Per-agency responder list state (Feature #325)
  const [expandedAgency, setExpandedAgency] = useState(null);
  const [agencyResponders, setAgencyResponders] = useState({});
  const [agencyRespondersLoading, setAgencyRespondersLoading] = useState({});
  const [responderMsg, setResponderMsg] = useState({});

  // Feature #374: AbortController ref for stale response handling.
  // When a new subscriber search/page request starts, the previous pending
  // request is aborted so its response cannot overwrite newer results.
  const subscriberAbortRef = useRef(null);

  const LIMIT = 10;

  useEffect(() => {
    // Only allow subscriber_admin
    if (user && user.role !== 'subscriber_admin') {
      navigate('/dashboard');
    }
  }, [user, navigate]);

  useEffect(() => {
    fetchStats();
  }, []);

  useEffect(() => {
    fetchSubscribers();
    // Cleanup: abort any in-flight subscriber request when effect re-runs or unmounts
    return () => {
      if (subscriberAbortRef.current) {
        subscriberAbortRef.current.abort();
        subscriberAbortRef.current = null;
      }
    };
  }, [page, search]);

  const fetchStats = async () => {
    setStatsLoading(true);
    try {
      const res = await api.get('/admin/stats');
      setStats(res.data);
    } catch (err) {
      console.error('Failed to fetch stats:', err);
    } finally {
      setStatsLoading(false);
    }
  };

  const fetchSubscribers = async () => {
    // Feature #374: Stale response handling — abort any previous in-flight request
    // before starting a new one. This ensures a slow earlier response can never
    // overwrite the results of a more recent request.
    if (subscriberAbortRef.current) {
      subscriberAbortRef.current.abort();
    }
    const controller = new AbortController();
    subscriberAbortRef.current = controller;

    setLoading(true);
    setError('');
    try {
      const params = new URLSearchParams({ page, limit: LIMIT });
      if (search) params.append('search', search);
      const res = await api.get(`/admin/subscribers?${params}`, {
        signal: controller.signal,
      });
      // Only update state if this request was not superseded by a newer one
      if (!controller.signal.aborted) {
        setSubscribers(res.data.subscribers);
        setTotal(res.data.total);
      }
    } catch (err) {
      // Ignore abort errors — they are intentional cancellations, not real failures
      if (err.name === 'CanceledError' || err.name === 'AbortError' || err.code === 'ERR_CANCELED') {
        return;
      }
      setError('Failed to load subscribers');
    } finally {
      // Only clear loading if this is still the current request
      if (subscriberAbortRef.current === controller) {
        setLoading(false);
        subscriberAbortRef.current = null;
      }
    }
  };

  const fetchAuditRecords = async () => {
    try {
      const res = await api.get('/admin/audit?limit=20');
      setAuditRecords(res.data.records || []);
    } catch (err) {
      console.error('Failed to fetch audit records:', err);
    }
  };

  const handleSearch = (e) => {
    e.preventDefault();
    setSearch(searchInput);
    setPage(1);
  };

  const handleRoleChange = async (subscriberId, newRole) => {
    setUpdateMsg('');
    try {
      await api.patch(`/admin/subscribers/${subscriberId}`, { role: newRole });
      setUpdateMsg(`Account ${subscriberId} role updated to ${newRole}`);
      fetchSubscribers();
      fetchStats();
    } catch (err) {
      setUpdateMsg(`Error: ${err.response?.data?.error || 'Update failed'}`);
    }
  };

  const handleVerifyEmail = async (subscriberId) => {
    setUpdateMsg('');
    try {
      await api.patch(`/admin/subscribers/${subscriberId}`, { email_verified: true });
      setUpdateMsg(`Account ${subscriberId} email verified`);
      fetchSubscribers();
    } catch (err) {
      setUpdateMsg(`Error: ${err.response?.data?.error || 'Update failed'}`);
    }
  };

  const handleAuditModifyAttempt = async () => {
    setModifyAttempt(null);
    try {
      await api.put('/admin/audit/99999', { data: 'tamper' });
      setModifyAttempt({ success: false, message: 'ERROR: Modification was allowed (should be blocked)' });
    } catch (err) {
      const msg = err.response?.data?.error || err.message;
      setModifyAttempt({ success: true, message: `✅ Correctly blocked: ${msg}` });
    }
  };

  const handleTabChange = (tab) => {
    setActiveTab(tab);
    if (tab === 'audit') {
      fetchAuditRecords();
    }
    if (tab === 'agencies') {
      fetchAgencies();
    }
  };

  const fetchAgencies = async () => {
    setAgenciesLoading(true);
    try {
      const res = await api.get('/admin/agencies');
      setAgencies(res.data.agencies || []);
    } catch (err) {
      console.error('Failed to fetch agencies:', err);
    } finally {
      setAgenciesLoading(false);
    }
  };

  const handleDeactivateAgency = async (agencyId, agencyName) => {
    if (!window.confirm(`Deactivate agency "${agencyName}" and all its responder accounts? They will not be able to scan until re-activated.`)) return;
    setAgencyMsg('');
    try {
      const res = await api.delete(`/admin/agencies/${agencyId}`);
      const count = res.data.affected_responders;
      setAgencyMsg(`✅ Agency "${agencyName}" deactivated. ${count} responder account(s) deactivated.`);
      fetchAgencies();
    } catch (err) {
      setAgencyMsg(`❌ Error: ${err.response?.data?.error || 'Failed to deactivate agency'}`);
    }
  };

  const handleReactivateAgency = async (agencyId, agencyName) => {
    setAgencyMsg('');
    try {
      const res = await api.post(`/admin/agencies/${agencyId}/reactivate`);
      setAgencyMsg(`✅ Agency "${agencyName}" reactivated. ${res.data.affected_responders} responder(s) reactivated.`);
      fetchAgencies();
      // Refresh expanded agency responders if open
      if (expandedAgency === agencyId) fetchAgencyResponders(agencyId);
    } catch (err) {
      setAgencyMsg(`❌ Error: ${err.response?.data?.error || 'Failed to reactivate agency'}`);
    }
  };

  // Feature #325: Fetch responders for a specific agency
  const fetchAgencyResponders = async (agencyId) => {
    setAgencyRespondersLoading(prev => ({ ...prev, [agencyId]: true }));
    try {
      const res = await api.get(`/admin/agencies/${agencyId}/responders`);
      setAgencyResponders(prev => ({ ...prev, [agencyId]: res.data.responders || [] }));
    } catch (err) {
      console.error('Failed to fetch agency responders:', err);
    } finally {
      setAgencyRespondersLoading(prev => ({ ...prev, [agencyId]: false }));
    }
  };

  // Feature #325: Toggle expanded agency to show/hide responder list
  const handleToggleAgencyExpand = (agencyId) => {
    if (expandedAgency === agencyId) {
      setExpandedAgency(null);
    } else {
      setExpandedAgency(agencyId);
      fetchAgencyResponders(agencyId);
    }
  };

  // Feature #325: Toggle individual responder active status
  const handleToggleResponder = async (agencyId, responderId, isActive, email) => {
    setResponderMsg(prev => ({ ...prev, [responderId]: 'loading' }));
    try {
      await api.patch(`/admin/responders/${responderId}`, { is_active: !isActive });
      const action = isActive ? 'deactivated' : 'activated';
      setResponderMsg(prev => ({ ...prev, [responderId]: `✅ ${email} ${action}` }));
      // Refresh responders and agency counts
      fetchAgencyResponders(agencyId);
      fetchAgencies();
      setTimeout(() => {
        setResponderMsg(prev => ({ ...prev, [responderId]: '' }));
      }, 3000);
    } catch (err) {
      setResponderMsg(prev => ({ ...prev, [responderId]: `❌ ${err.response?.data?.error || 'Update failed'}` }));
    }
  };

  const handleLogout = () => {
    logout();
    navigate('/login');
  };

  const handleViewAsSubscriber = async () => {
    setViewAsMsg('');
    try {
      await startViewAsRole('subscriber');
      navigate('/dashboard');
    } catch (err) {
      setViewAsMsg(err.response?.data?.error || 'Failed to start subscriber view');
    }
  };

  if (!user || user.role !== 'subscriber_admin') {
    return (
      <div className="min-h-screen flex items-center justify-center">
        <p className="text-gray-600">Access denied. Admin role required.</p>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-gray-50" data-testid="admin-dashboard">
      {/* Header */}
      <header className="bg-white border-b border-gray-200 shadow-sm">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-4 flex items-center justify-between">
          <div>
            <h1 className="text-2xl font-bold text-sky-700">
              LiveSafe<span className="text-emerald-600">.ai</span>
              <span className="ml-3 text-lg font-medium text-gray-700">Platform Admin</span>
            </h1>
            <p className="text-sm text-gray-500 mt-1">Logged in as {user.email} (subscriber_admin)</p>
          </div>
          <div className="flex items-center gap-2">
            <button
              onClick={handleViewAsSubscriber}
              className="px-4 py-2 text-sm text-sky-700 border border-sky-300 rounded-lg hover:bg-sky-50 transition"
              data-testid="view-as-subscriber-btn"
            >
              View as Subscriber
            </button>
            <button
              onClick={handleLogout}
              className="px-4 py-2 text-sm text-red-600 border border-red-300 rounded-lg hover:bg-red-50 transition"
              data-testid="admin-logout-btn"
            >
              Sign Out
            </button>
          </div>
        </div>
      </header>

      <main className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        {viewAsMsg && (
          <div className="mb-4 p-3 rounded-lg text-sm bg-red-50 text-red-700" role="alert" data-testid="view-as-error">
            {viewAsMsg}
          </div>
        )}

        {/* Stats */}
        {!statsLoading && stats && (
          <div className="grid grid-cols-2 md:grid-cols-4 gap-4 mb-8" data-testid="admin-stats">
            <div className="bg-white rounded-xl p-5 shadow-sm border border-gray-100">
              <p className="text-sm text-gray-500">Total Subscribers</p>
              <p className="text-3xl font-bold text-sky-700 mt-1" data-testid="stat-total-subscribers">
                {stats.subscribers.total}
              </p>
            </div>
            <div className="bg-white rounded-xl p-5 shadow-sm border border-gray-100">
              <p className="text-sm text-gray-500">Providers</p>
              <p className="text-3xl font-bold text-emerald-600 mt-1" data-testid="stat-providers">
                {stats.providers}
              </p>
            </div>
            <div className="bg-white rounded-xl p-5 shadow-sm border border-gray-100">
              <p className="text-sm text-gray-500">Health Records</p>
              <p className="text-3xl font-bold text-violet-600 mt-1" data-testid="stat-records">
                {stats.medical_records}
              </p>
            </div>
            <div className="bg-white rounded-xl p-5 shadow-sm border border-gray-100">
              <p className="text-sm text-gray-500">Card Scans</p>
              <p className="text-3xl font-bold text-amber-600 mt-1" data-testid="stat-scans">
                {stats.scans}
              </p>
            </div>
          </div>
        )}

        {/* Tabs */}
        <div className="bg-white rounded-xl shadow-sm border border-gray-100">
          <div className="flex border-b border-gray-200">
            <button
              onClick={() => handleTabChange('accounts')}
              className={`px-6 py-4 text-sm font-medium transition ${
                activeTab === 'accounts'
                  ? 'text-sky-700 border-b-2 border-sky-700'
                  : 'text-gray-500 hover:text-gray-700'
              }`}
              data-testid="tab-accounts"
            >
              👥 Manage Accounts
            </button>
            <button
              onClick={() => handleTabChange('audit')}
              className={`px-6 py-4 text-sm font-medium transition ${
                activeTab === 'audit'
                  ? 'text-sky-700 border-b-2 border-sky-700'
                  : 'text-gray-500 hover:text-gray-700'
              }`}
              data-testid="tab-audit"
            >
              🔒 Audit Trail
            </button>
            <button
              onClick={() => handleTabChange('agencies')}
              className={`px-6 py-4 text-sm font-medium transition ${
                activeTab === 'agencies'
                  ? 'text-sky-700 border-b-2 border-sky-700'
                  : 'text-gray-500 hover:text-gray-700'
              }`}
              data-testid="tab-agencies"
            >
              🏢 Agency Management
            </button>
          </div>

          {/* Accounts Tab */}
          {activeTab === 'accounts' && (
            <div className="p-6" data-testid="accounts-panel">
              <div className="flex items-center justify-between mb-4">
                <h2 className="text-lg font-semibold text-gray-900">Subscriber Accounts</h2>
                <span className="text-sm text-gray-500">{total} total accounts</span>
              </div>

              {/* Search */}
              <form onSubmit={handleSearch} className="flex gap-2 mb-4">
                <input
                  type="text"
                  value={searchInput}
                  onChange={(e) => setSearchInput(e.target.value)}
                  placeholder="Search by email or name..."
                  className="flex-1 px-3 py-2 border border-gray-300 rounded-lg text-sm focus:ring-2 focus:ring-sky-500 focus:border-sky-500"
                  data-testid="account-search"
                />
                <button
                  type="submit"
                  className="px-4 py-2 bg-sky-500 text-white text-sm rounded-lg hover:bg-sky-600 transition"
                  data-testid="account-search-btn"
                >
                  Search
                </button>
                {search && (
                  <button
                    type="button"
                    onClick={() => { setSearch(''); setSearchInput(''); setPage(1); }}
                    className="px-4 py-2 text-sm text-gray-600 border border-gray-300 rounded-lg hover:bg-gray-50 transition"
                  >
                    Clear
                  </button>
                )}
              </form>

              {updateMsg && (
                <div
                  className={`mb-4 p-3 rounded-lg text-sm ${
                    updateMsg.startsWith('Error') ? 'bg-red-50 text-red-700' : 'bg-green-50 text-green-700'
                  }`}
                  data-testid="update-message"
                >
                  {updateMsg}
                </div>
              )}

              {error && (
                <div className="mb-4 p-3 bg-red-50 text-red-700 rounded-lg text-sm" role="alert">{error}</div>
              )}

              {loading ? (
                <div className="text-center py-8 text-gray-500">Loading accounts...</div>
              ) : (
                <>
                  <div className="overflow-x-auto" data-testid="subscribers-table">
                    <table className="w-full text-sm">
                      <thead>
                        <tr className="border-b border-gray-200">
                          <th className="text-left py-2 px-3 text-gray-500 font-medium">Email</th>
                          <th className="text-left py-2 px-3 text-gray-500 font-medium">Name</th>
                          <th className="text-left py-2 px-3 text-gray-500 font-medium">Role</th>
                          <th className="text-left py-2 px-3 text-gray-500 font-medium">Verified</th>
                          <th className="text-left py-2 px-3 text-gray-500 font-medium">Joined</th>
                          <th className="text-left py-2 px-3 text-gray-500 font-medium">Actions</th>
                        </tr>
                      </thead>
                      <tbody>
                        {subscribers.map((sub) => (
                          <tr key={sub.id} className="border-b border-gray-100 hover:bg-gray-50" data-testid={`subscriber-row-${sub.id}`}>
                            <td className="py-2 px-3 text-gray-900">{sub.email}</td>
                            <td className="py-2 px-3 text-gray-600">
                              {sub.first_name || sub.last_name ? `${sub.first_name || ''} ${sub.last_name || ''}`.trim() : '—'}
                            </td>
                            <td className="py-2 px-3">
                              <span className={`inline-flex px-2 py-0.5 rounded-full text-xs font-medium ${
                                sub.role === 'subscriber_admin' ? 'bg-red-100 text-red-700' : 'bg-blue-100 text-blue-700'
                              }`}>
                                {sub.role}
                              </span>
                            </td>
                            <td className="py-2 px-3">
                              {sub.email_verified ? (
                                <span className="text-green-600">✓ Yes</span>
                              ) : (
                                <span className="text-amber-600">Pending</span>
                              )}
                            </td>
                            <td className="py-2 px-3 text-gray-500">
                              {new Date(sub.created_at).toLocaleDateString()}
                            </td>
                            <td className="py-2 px-3">
                              <div className="flex gap-1">
                                {!sub.email_verified && (
                                  <button
                                    onClick={() => handleVerifyEmail(sub.id)}
                                    className="px-2 py-1 text-xs bg-green-100 text-green-700 rounded hover:bg-green-200 transition"
                                    data-testid={`verify-btn-${sub.id}`}
                                  >
                                    Verify
                                  </button>
                                )}
                                {sub.role !== 'subscriber_admin' ? (
                                  <button
                                    onClick={() => handleRoleChange(sub.id, 'subscriber_admin')}
                                    className="px-2 py-1 text-xs bg-amber-100 text-amber-700 rounded hover:bg-amber-200 transition"
                                    data-testid={`promote-btn-${sub.id}`}
                                  >
                                    Make Admin
                                  </button>
                                ) : (
                                  sub.id !== user.id && (
                                    <button
                                      onClick={() => handleRoleChange(sub.id, 'subscriber')}
                                      className="px-2 py-1 text-xs bg-gray-100 text-gray-700 rounded hover:bg-gray-200 transition"
                                      data-testid={`demote-btn-${sub.id}`}
                                    >
                                      Remove Admin
                                    </button>
                                  )
                                )}
                              </div>
                            </td>
                          </tr>
                        ))}
                        {subscribers.length === 0 && (
                          <tr>
                            <td colSpan={6} className="py-8 text-center text-gray-500">
                              No subscribers found
                            </td>
                          </tr>
                        )}
                      </tbody>
                    </table>
                  </div>

                  {/* Pagination */}
                  {total > LIMIT && (
                    <div className="flex items-center justify-between mt-4">
                      <p className="text-sm text-gray-500">
                        Showing {(page - 1) * LIMIT + 1}–{Math.min(page * LIMIT, total)} of {total}
                      </p>
                      <div className="flex gap-2">
                        <button
                          onClick={() => setPage(p => Math.max(1, p - 1))}
                          disabled={page === 1}
                          className="px-3 py-1 text-sm border border-gray-300 rounded hover:bg-gray-50 disabled:opacity-50"
                        >
                          Previous
                        </button>
                        <button
                          onClick={() => setPage(p => p + 1)}
                          disabled={page * LIMIT >= total}
                          className="px-3 py-1 text-sm border border-gray-300 rounded hover:bg-gray-50 disabled:opacity-50"
                        >
                          Next
                        </button>
                      </div>
                    </div>
                  )}
                </>
              )}
            </div>
          )}

          {/* Audit Tab */}
          {activeTab === 'audit' && (
            <div className="p-6" data-testid="audit-panel">
              <div className="flex items-center justify-between mb-6">
                <h2 className="text-lg font-semibold text-gray-900">Audit Trail (Read-Only)</h2>
                <span className="text-xs bg-red-100 text-red-700 px-3 py-1 rounded-full font-medium">
                  🔒 Immutable — Cannot be modified
                </span>
              </div>

              {/* Audit modification attempt test */}
              <div className="mb-6 p-4 bg-gray-50 rounded-lg border border-gray-200" data-testid="audit-immutability-test">
                <h3 className="text-sm font-medium text-gray-700 mb-2">Immutability Verification</h3>
                <p className="text-xs text-gray-500 mb-3">
                  Attempt to modify an audit record via admin API to verify it's blocked.
                </p>
                <button
                  onClick={handleAuditModifyAttempt}
                  className="px-4 py-2 text-sm bg-red-100 text-red-700 rounded-lg hover:bg-red-200 transition"
                  data-testid="attempt-modify-audit-btn"
                >
                  🔧 Attempt Audit Modification (should be rejected)
                </button>
                {modifyAttempt && (
                  <div
                    className={`mt-3 p-3 rounded-lg text-sm ${
                      modifyAttempt.success ? 'bg-green-50 text-green-700' : 'bg-red-50 text-red-700'
                    }`}
                    data-testid="audit-modify-result"
                  >
                    {modifyAttempt.message}
                  </div>
                )}
              </div>

              {/* Audit records list */}
              <div data-testid="audit-records-list">
                {auditRecords.length === 0 ? (
                  <p className="text-gray-500 text-sm">No audit records found</p>
                ) : (
                  <div className="space-y-2">
                    {auditRecords.slice(0, 10).map((record) => (
                      <div
                        key={record.id}
                        className="p-3 bg-gray-50 rounded-lg border border-gray-200 text-sm"
                        data-testid={`audit-record-${record.id}`}
                      >
                        <div className="flex items-center justify-between">
                          <span className="font-medium text-gray-700">{record.action || record.event_type}</span>
                          <span className="text-xs text-gray-500">
                            {new Date(record.created_at).toLocaleString()}
                          </span>
                        </div>
                        {record.subscriber_email && (
                          <p className="text-xs text-gray-500 mt-1">{record.subscriber_email}</p>
                        )}
                      </div>
                    ))}
                  </div>
                )}
              </div>
            </div>
          )}

          {/* Agency Management Tab (Feature #235) */}
          {activeTab === 'agencies' && (
            <div className="p-6" data-testid="agencies-panel">
              <div className="flex items-center justify-between mb-6">
                <h2 className="text-lg font-semibold text-gray-900">Agency Management</h2>
                <p className="text-sm text-gray-500">
                  Deactivating an agency deactivates all its responder accounts
                </p>
              </div>

              {agencyMsg && (
                <div
                  className={`mb-4 p-3 rounded-lg text-sm ${
                    agencyMsg.startsWith('❌') ? 'bg-red-50 text-red-700' : 'bg-green-50 text-green-700'
                  }`}
                  data-testid="agency-action-message"
                >
                  {agencyMsg}
                </div>
              )}

              {agenciesLoading ? (
                <div className="text-center py-8 text-gray-500">Loading agencies...</div>
              ) : agencies.length === 0 ? (
                <p className="text-gray-500 text-sm">No agencies registered</p>
              ) : (
                <div className="overflow-x-auto" data-testid="agencies-table">
                  <table className="w-full text-sm">
                    <thead>
                      <tr className="border-b border-gray-200">
                        <th className="text-left py-2 px-3 text-gray-500 font-medium">Agency</th>
                        <th className="text-left py-2 px-3 text-gray-500 font-medium">Type</th>
                        <th className="text-left py-2 px-3 text-gray-500 font-medium">Responders</th>
                        <th className="text-left py-2 px-3 text-gray-500 font-medium">Status</th>
                        <th className="text-left py-2 px-3 text-gray-500 font-medium">Actions</th>
                      </tr>
                    </thead>
                    <tbody>
                      {agencies.map((agency) => (
                        <React.Fragment key={agency.id}>
                        <tr
                          className="border-b border-gray-100 hover:bg-gray-50"
                          data-testid={`agency-row-${agency.id}`}
                        >
                          <td className="py-3 px-3">
                            <div>
                              <button
                                onClick={() => handleToggleAgencyExpand(agency.id)}
                                className="font-medium text-gray-900 hover:text-sky-700 text-left"
                                data-testid={`agency-expand-btn-${agency.id}`}
                              >
                                {expandedAgency === agency.id ? '▼' : '▶'} {agency.name}
                              </button>
                            </div>
                          </td>
                          <td className="py-3 px-3 text-gray-600 capitalize">{agency.type}</td>
                          <td className="py-3 px-3 text-gray-600">
                            <span data-testid={`agency-${agency.id}-responder-count`}>
                              {agency.active_responders} active / {agency.responder_count} total
                            </span>
                          </td>
                          <td className="py-3 px-3">
                            <span
                              className={`px-2 py-0.5 rounded-full text-xs font-medium ${
                                agency.is_active
                                  ? 'bg-green-100 text-green-700'
                                  : 'bg-red-100 text-red-700'
                              }`}
                              data-testid={`agency-${agency.id}-status`}
                            >
                              {agency.is_active ? 'Active' : 'Deactivated'}
                            </span>
                          </td>
                          <td className="py-3 px-3">
                            {agency.is_active ? (
                              <button
                                onClick={() => handleDeactivateAgency(agency.id, agency.name)}
                                className="px-3 py-1 text-xs bg-red-100 text-red-700 rounded-lg hover:bg-red-200 transition"
                                data-testid={`deactivate-agency-${agency.id}`}
                              >
                                Deactivate Agency
                              </button>
                            ) : (
                              <button
                                onClick={() => handleReactivateAgency(agency.id, agency.name)}
                                className="px-3 py-1 text-xs bg-green-100 text-green-700 rounded-lg hover:bg-green-200 transition"
                                data-testid={`reactivate-agency-${agency.id}`}
                              >
                                Reactivate Agency
                              </button>
                            )}
                          </td>
                        </tr>
                        {/* Feature #325: Expanded responder list for this agency */}
                        {expandedAgency === agency.id && (
                          <tr key={`${agency.id}-responders`} data-testid={`agency-${agency.id}-responders-row`}>
                            <td colSpan={5} className="px-6 pb-4 bg-gray-50">
                              <div className="mt-2" data-testid={`agency-${agency.id}-responder-list`}>
                                <p className="text-xs font-semibold text-gray-500 mb-2 uppercase tracking-wide">
                                  Responders ({(agencyResponders[agency.id] || []).length})
                                </p>
                                {agencyRespondersLoading[agency.id] ? (
                                  <p className="text-xs text-gray-400">Loading responders...</p>
                                ) : (agencyResponders[agency.id] || []).length === 0 ? (
                                  <p className="text-xs text-gray-400" data-testid={`agency-${agency.id}-no-responders`}>No responders registered</p>
                                ) : (
                                  <div className="space-y-1">
                                    {(agencyResponders[agency.id] || []).map(responder => (
                                      <div
                                        key={responder.id}
                                        className="flex items-center justify-between bg-white rounded-lg px-3 py-2 text-sm border border-gray-100"
                                        data-testid={`responder-row-${responder.id}`}
                                      >
                                        <div className="flex items-center gap-3">
                                          <span
                                            className={`w-2 h-2 rounded-full flex-shrink-0 ${responder.is_active ? 'bg-green-500' : 'bg-red-400'}`}
                                            data-testid={`responder-${responder.id}-status-dot`}
                                          />
                                          <span className="text-gray-700" data-testid={`responder-${responder.id}-email`}>{responder.email}</span>
                                          <span className="text-xs text-gray-400 capitalize">{responder.role}</span>
                                          <span
                                            className={`text-xs px-1.5 py-0.5 rounded ${responder.is_active ? 'bg-green-50 text-green-700' : 'bg-red-50 text-red-600'}`}
                                            data-testid={`responder-${responder.id}-status`}
                                          >
                                            {responder.is_active ? 'Active' : 'Inactive'}
                                          </span>
                                        </div>
                                        <div className="flex items-center gap-2">
                                          {responderMsg[responder.id] && responderMsg[responder.id] !== 'loading' && (
                                            <span className="text-xs text-gray-500">{responderMsg[responder.id]}</span>
                                          )}
                                          <button
                                            onClick={() => handleToggleResponder(agency.id, responder.id, responder.is_active, responder.email)}
                                            disabled={responderMsg[responder.id] === 'loading'}
                                            className={`px-2 py-1 text-xs rounded transition ${
                                              responder.is_active
                                                ? 'bg-red-50 text-red-600 hover:bg-red-100'
                                                : 'bg-green-50 text-green-600 hover:bg-green-100'
                                            }`}
                                            data-testid={`responder-${responder.id}-toggle`}
                                          >
                                            {responder.is_active ? 'Deactivate' : 'Activate'}
                                          </button>
                                        </div>
                                      </div>
                                    ))}
                                  </div>
                                )}
                              </div>
                            </td>
                          </tr>
                        )}
                        </React.Fragment>
                      ))}
                    </tbody>
                  </table>
                </div>
              )}
            </div>
          )}
        </div>
      </main>
    </div>
  );
}

export default AdminDashboard;
