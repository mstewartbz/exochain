import React, { useEffect, useState } from 'react';
import { Link } from 'react-router-dom';
import Navbar from '../components/Navbar';
import api from '../services/api';

function Library() {
  const [installs, setInstalls] = useState([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');

  useEffect(() => {
    fetchLibrary();
  }, []);

  const fetchLibrary = async () => {
    setLoading(true);
    setError('');
    try {
      const res = await api.get('/marketplace/library');
      setInstalls(res.data.installs || []);
    } catch (err) {
      setError(err.response?.data?.error || err.message || 'Library is unavailable.');
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="min-h-screen bg-slate-50">
      <Navbar />
      <main className="mx-auto max-w-5xl px-4 py-6 sm:px-6 lg:px-8">
        <div className="mb-6 flex flex-col gap-4 sm:flex-row sm:items-end sm:justify-between">
          <div>
            <p className="text-sm font-semibold uppercase tracking-normal text-teal-700">
              Installed capabilities
            </p>
            <h1 className="mt-1 text-3xl font-bold tracking-normal text-slate-950">
              Library
            </h1>
            <p className="mt-2 max-w-2xl text-sm leading-6 text-slate-600">
              Your installed marketplace cards and templates are listed here. Entries stay bounded to reviewed catalog metadata and do not create external trust claims.
            </p>
          </div>
          <Link
            to="/marketplace"
            className="rounded-md bg-teal-700 px-4 py-2 text-sm font-semibold text-white hover:bg-teal-800"
          >
            Browse marketplace
          </Link>
        </div>

        {error && (
          <p className="mb-4 rounded-md border border-red-200 bg-red-50 px-3 py-2 text-sm font-semibold text-red-900">
            {error}
          </p>
        )}

        {loading ? (
          <div className="rounded-md border border-slate-200 bg-white p-6 text-sm text-slate-600">
            Loading your library...
          </div>
        ) : installs.length === 0 ? (
          <div className="rounded-md border border-slate-200 bg-white p-6">
            <p className="text-base font-semibold text-slate-950">No installs yet</p>
            <p className="mt-2 text-sm leading-6 text-slate-600">
              Add reviewed cards, protocols, and guidance panels from the marketplace when you are ready.
            </p>
          </div>
        ) : (
          <div className="grid gap-3">
            {installs.map((install) => (
              <article
                data-testid="library-install"
                key={install.id}
                className="rounded-md border border-slate-200 bg-white p-4 shadow-sm"
              >
                <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
                  <div>
                    <h2 className="text-lg font-bold text-slate-950">{install.title}</h2>
                    <p className="mt-1 text-sm text-slate-600">
                      Installed {install.installed_at ? new Date(install.installed_at).toLocaleString() : 'recently'}
                    </p>
                  </div>
                  <Link
                    to={`/marketplace?item=${encodeURIComponent(install.slug || '')}`}
                    className="rounded-md border border-slate-300 px-3 py-2 text-sm font-semibold text-slate-800 hover:bg-slate-100"
                  >
                    View catalog
                  </Link>
                </div>
              </article>
            ))}
          </div>
        )}
      </main>
    </div>
  );
}

export default Library;
