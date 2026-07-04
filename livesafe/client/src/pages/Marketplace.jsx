import React, { useEffect, useMemo, useState } from 'react';
import Navbar from '../components/Navbar';
import api from '../services/api';

const CATEGORY_OPTIONS = [
  'all',
  'business',
  'coaching',
  'emergency',
  'legal',
  'medical',
  'personal',
  'sales',
];

const TYPE_OPTIONS = [
  { value: 'all', label: 'All types' },
  { value: 'panel', label: 'Guidance panels' },
  { value: 'meeting', label: 'Meeting frameworks' },
  { value: 'emergency', label: 'Emergency protocols' },
];

function Marketplace() {
  const [items, setItems] = useState([]);
  const [roles, setRoles] = useState([]);
  const [libraryInstalls, setLibraryInstalls] = useState([]);
  const [search, setSearch] = useState('');
  const [category, setCategory] = useState('all');
  const [objectType, setObjectType] = useState('all');
  const [selectedItem, setSelectedItem] = useState(null);
  const [loading, setLoading] = useState(true);
  const [installingId, setInstallingId] = useState(null);
  const [error, setError] = useState('');
  const [notice, setNotice] = useState('');

  const installedIds = useMemo(
    () => new Set(libraryInstalls.map((install) => install.marketplace_item_id)),
    [libraryInstalls],
  );

  useEffect(() => {
    fetchCatalog();
    fetchRoles();
    fetchLibrary();
  }, [category, objectType]);

  const fetchCatalog = async () => {
    setLoading(true);
    setError('');
    try {
      const params = new URLSearchParams();
      if (category !== 'all') params.set('category', category);
      if (objectType !== 'all') params.set('object_type', objectType);
      if (search.trim()) params.set('search', search.trim());
      const res = await api.get(`/marketplace/catalog?${params.toString()}`);
      setItems(res.data.items || []);
    } catch (err) {
      setError(err.response?.data?.error || err.message || 'Marketplace catalog is unavailable.');
    } finally {
      setLoading(false);
    }
  };

  const fetchRoles = async () => {
    try {
      const res = await api.get('/marketplace/roles');
      setRoles(res.data.roles || []);
    } catch (err) {
      setRoles([]);
    }
  };

  const fetchLibrary = async () => {
    try {
      const res = await api.get('/marketplace/library');
      setLibraryInstalls(res.data.installs || []);
    } catch (err) {
      setLibraryInstalls([]);
    }
  };

  const handleInstall = async (item) => {
    setInstallingId(item.id);
    setNotice('');
    setError('');
    try {
      const res = await api.post('/marketplace/installs', {
        marketplace_item_id: item.id,
      });
      setLibraryInstalls((current) => {
        const withoutDuplicate = current.filter(
          (install) => install.marketplace_item_id !== res.data.marketplace_item_id,
        );
        return [res.data, ...withoutDuplicate];
      });
      setNotice(`${item.title} is now in your library.`);
    } catch (err) {
      setError(err.response?.data?.error || err.message || 'Install failed.');
    } finally {
      setInstallingId(null);
    }
  };

  const visibleItems = useMemo(() => {
    const normalized = search.trim().toLowerCase();
    if (!normalized) return items;
    return items.filter((item) => {
      return [
        item.title,
        item.summary,
        item.category,
        item.object_type,
        ...(item.tags || []),
      ]
        .filter(Boolean)
        .some((value) => String(value).toLowerCase().includes(normalized));
    });
  }, [items, search]);

  return (
    <div className="min-h-screen bg-slate-50">
      <Navbar />
      <main className="mx-auto max-w-7xl px-4 py-6 sm:px-6 lg:px-8">
        <div className="mb-6 flex flex-col gap-4 lg:flex-row lg:items-end lg:justify-between">
          <div>
            <p className="text-sm font-semibold uppercase tracking-normal text-teal-700">
              Launch catalog
            </p>
            <h1 className="mt-1 text-3xl font-bold tracking-normal text-slate-950">
              Marketplace
            </h1>
            <p className="mt-2 max-w-3xl text-sm leading-6 text-slate-600">
              Browse reviewed LiveSafe.ai cards, agent roles, meeting frameworks, and emergency protocols imported from the Ambientli library. Catalog entries stay fail-closed and make no external trust claim.
            </p>
          </div>
          <button
            onClick={fetchCatalog}
            className="rounded-md border border-slate-300 bg-white px-4 py-2 text-sm font-semibold text-slate-800 hover:bg-slate-100"
          >
            Refresh
          </button>
        </div>

        <section className="mb-5 grid gap-3 md:grid-cols-[1fr_180px_220px]">
          <label className="block">
            <span className="mb-1 block text-sm font-semibold text-slate-700">Search</span>
            <input
              data-testid="marketplace-search"
              value={search}
              onChange={(event) => setSearch(event.target.value)}
              onKeyDown={(event) => {
                if (event.key === 'Enter') fetchCatalog();
              }}
              className="w-full rounded-md border border-slate-300 px-3 py-2 text-sm focus:border-teal-600 focus:outline-none focus:ring-2 focus:ring-teal-100"
              placeholder="Find crisis, medical, sales, or family templates"
            />
          </label>
          <label className="block">
            <span className="mb-1 block text-sm font-semibold text-slate-700">Category</span>
            <select
              value={category}
              onChange={(event) => setCategory(event.target.value)}
              className="w-full rounded-md border border-slate-300 px-3 py-2 text-sm focus:border-teal-600 focus:outline-none focus:ring-2 focus:ring-teal-100"
            >
              {CATEGORY_OPTIONS.map((option) => (
                <option key={option} value={option}>
                  {option === 'all' ? 'All categories' : option}
                </option>
              ))}
            </select>
          </label>
          <label className="block">
            <span className="mb-1 block text-sm font-semibold text-slate-700">Type</span>
            <select
              value={objectType}
              onChange={(event) => setObjectType(event.target.value)}
              className="w-full rounded-md border border-slate-300 px-3 py-2 text-sm focus:border-teal-600 focus:outline-none focus:ring-2 focus:ring-teal-100"
            >
              {TYPE_OPTIONS.map((option) => (
                <option key={option.value} value={option.value}>
                  {option.label}
                </option>
              ))}
            </select>
          </label>
        </section>

        {notice && (
          <p className="mb-4 rounded-md border border-emerald-200 bg-emerald-50 px-3 py-2 text-sm font-semibold text-emerald-900">
            {notice}
          </p>
        )}
        {error && (
          <p className="mb-4 rounded-md border border-red-200 bg-red-50 px-3 py-2 text-sm font-semibold text-red-900">
            {error}
          </p>
        )}

        <section className="grid gap-5 lg:grid-cols-[1fr_300px]">
          <div>
            {loading ? (
              <div className="rounded-md border border-slate-200 bg-white p-6 text-sm text-slate-600">
                Loading marketplace catalog...
              </div>
            ) : visibleItems.length === 0 ? (
              <div className="rounded-md border border-slate-200 bg-white p-6 text-sm text-slate-600">
                No reviewed marketplace items match the current filters.
              </div>
            ) : (
              <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-3">
                {visibleItems.map((item) => {
                  const installed = installedIds.has(item.id);
                  return (
                    <article
                      data-testid="marketplace-card"
                      key={item.id}
                      className="flex min-h-[260px] flex-col rounded-md border border-slate-200 bg-white p-4 shadow-sm"
                    >
                      <div className="mb-3 flex items-start justify-between gap-3">
                        <div>
                          <p className="text-xs font-semibold uppercase tracking-normal text-slate-500">
                            {item.object_type} / {item.category}
                          </p>
                          <h2 className="mt-1 text-lg font-bold leading-6 text-slate-950">
                            {item.title}
                          </h2>
                        </div>
                        <span className="rounded-md bg-teal-50 px-2 py-1 text-xs font-semibold text-teal-800">
                          {item.public_claims_allowed ? 'Verified' : 'Review gated'}
                        </span>
                      </div>
                      <p className="line-clamp-4 flex-1 text-sm leading-6 text-slate-600">
                        {item.summary}
                      </p>
                      <div className="mt-3 flex flex-wrap gap-2">
                        {(item.tags || []).slice(0, 3).map((tag) => (
                          <span
                            key={tag}
                            className="rounded-md bg-slate-100 px-2 py-1 text-xs font-medium text-slate-700"
                          >
                            {tag}
                          </span>
                        ))}
                      </div>
                      <div className="mt-4 flex gap-2">
                        <button
                          onClick={() => setSelectedItem(item)}
                          className="flex-1 rounded-md border border-slate-300 px-3 py-2 text-sm font-semibold text-slate-800 hover:bg-slate-100"
                        >
                          View
                        </button>
                        <button
                          onClick={() => handleInstall(item)}
                          disabled={installed || installingId === item.id}
                          className="flex-1 rounded-md bg-teal-700 px-3 py-2 text-sm font-semibold text-white hover:bg-teal-800 disabled:cursor-not-allowed disabled:bg-slate-400"
                        >
                          {installed ? 'Installed' : installingId === item.id ? 'Installing' : 'Install'}
                        </button>
                      </div>
                    </article>
                  );
                })}
              </div>
            )}
          </div>

          <aside className="rounded-md border border-slate-200 bg-white p-4 shadow-sm">
            <h2 className="text-base font-bold text-slate-950">Agent roles</h2>
            <div className="mt-3 space-y-3">
              {roles.length === 0 ? (
                <p className="text-sm text-slate-600">No active roles are available yet.</p>
              ) : (
                roles.map((role) => (
                  <div key={role.role_name} className="rounded-md border border-slate-200 p-3">
                    <p className="text-sm font-semibold text-slate-950">{role.display_name}</p>
                    <p className="mt-1 text-xs leading-5 text-slate-600">{role.description}</p>
                  </div>
                ))
              )}
            </div>
          </aside>
        </section>

        {selectedItem && (
          <div className="fixed inset-0 z-40 flex items-center justify-center bg-slate-950/40 p-4">
            <section className="max-h-[85vh] w-full max-w-2xl overflow-auto rounded-md bg-white p-5 shadow-xl">
              <div className="flex items-start justify-between gap-4">
                <div>
                  <p className="text-xs font-semibold uppercase tracking-normal text-teal-700">
                    {selectedItem.object_type} / {selectedItem.category}
                  </p>
                  <h2 className="mt-1 text-2xl font-bold text-slate-950">{selectedItem.title}</h2>
                </div>
                <button
                  onClick={() => setSelectedItem(null)}
                  className="rounded-md border border-slate-300 px-3 py-1 text-sm font-semibold text-slate-700 hover:bg-slate-100"
                >
                  Close
                </button>
              </div>
              <p className="mt-4 text-sm leading-6 text-slate-700">{selectedItem.summary}</p>
              <div className="mt-4 rounded-md border border-slate-200 bg-slate-50 p-3">
                <p className="text-sm font-semibold text-slate-950">Safety posture</p>
                <p className="mt-1 text-sm leading-6 text-slate-600">
                  This catalog item is reviewed for launch visibility, stores no public source-account metadata, and does not make an external trust claim.
                </p>
              </div>
              <pre className="mt-4 max-h-72 overflow-auto rounded-md bg-slate-950 p-3 text-xs leading-5 text-slate-100">
                {JSON.stringify(selectedItem.content || {}, null, 2)}
              </pre>
            </section>
          </div>
        )}
      </main>
    </div>
  );
}

export default Marketplace;
