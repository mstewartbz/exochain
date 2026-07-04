import React, { useState, useEffect } from 'react';
import { useParams, Link, useNavigate } from 'react-router-dom';
import { useAuth } from '../context/AuthContext';
import api from '../services/api';
import Navbar from '../components/Navbar';

function formatFileSize(bytes) {
  if (!bytes) return 'Unknown size';
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function formatDate(dateStr) {
  if (!dateStr) return 'Unknown date';
  try {
    return new Date(dateStr).toLocaleDateString('en-US', {
      year: 'numeric', month: 'long', day: 'numeric',
    });
  } catch (e) { return dateStr; }
}

export default function RecordDetail() {
  const { recordId } = useParams();
  const { user } = useAuth();
  const navigate = useNavigate();

  const [record, setRecord] = useState(null);
  const [versions, setVersions] = useState([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [downloading, setDownloading] = useState(false);
  const [downloadError, setDownloadError] = useState('');

  useEffect(() => {
    const fetchRecord = async () => {
      try {
        setLoading(true);
        // Fetch the specific record by getting all records and filtering
        const [recordsRes, versionsRes] = await Promise.all([
          api.get('/records'),
          api.get(`/records/${recordId}/versions`).catch(() => ({ data: { versions: [] } })),
        ]);

        const found = recordsRes.data.find(r => String(r.id) === String(recordId));
        if (!found) {
          setError('Record not found or you do not have permission to view it.');
          return;
        }

        setRecord(found);
        setVersions(versionsRes.data?.versions || []);
      } catch (err) {
        console.error('Failed to fetch record:', err);
        setError('Failed to load record details.');
      } finally {
        setLoading(false);
      }
    };

    if (recordId) fetchRecord();
  }, [recordId]);

  const handleDownload = async () => {
    setDownloading(true);
    setDownloadError('');
    try {
      const res = await api.get(`/records/${recordId}/download`, { responseType: 'blob' });
      const url = URL.createObjectURL(res.data);
      const a = document.createElement('a');
      a.href = url;
      a.download = record?.title || `record-${recordId}`;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);
    } catch (err) {
      setDownloadError('Failed to download record.');
    } finally {
      setDownloading(false);
    }
  };

  const visibilityLabel = (v) => {
    const labels = {
      all_providers: '👁️ All Providers',
      emergency_only: '🚨 Emergency Only',
      private: '🔒 Private (Only Me)',
      specific_providers: '👥 Specific Providers',
    };
    return labels[v] || v;
  };

  if (loading) {
    return (
      <div className="min-h-screen bg-gray-50">
        <Navbar />
        <div className="flex justify-center py-20">
          <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-sky-500"></div>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="min-h-screen bg-gray-50">
        <Navbar />
        <main className="max-w-3xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
          {/* Breadcrumbs */}
          <nav className="flex items-center gap-1 text-sm text-gray-500 mb-4" aria-label="Breadcrumb" data-testid="breadcrumbs">
            <Link to="/dashboard" className="hover:text-sky-600 transition">Dashboard</Link>
            <span className="text-gray-400">›</span>
            <Link to="/health-vault" className="hover:text-sky-600 transition">Health Vault</Link>
            <span className="text-gray-400">›</span>
            <span className="text-gray-900 font-medium">Record Detail</span>
          </nav>
          <div className="bg-red-50 border border-red-200 rounded-lg p-4 text-red-700" data-testid="record-not-found">{error}</div>
          <button
            onClick={() => navigate('/health-vault')}
            className="mt-4 text-sm text-sky-600 hover:underline"
            data-testid="back-to-vault-error"
          >
            ← Back to Health Vault
          </button>
        </main>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-gray-50">
      <Navbar />

      <main className="max-w-3xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        {/* Breadcrumbs (Feature #301) */}
        <nav className="flex items-center gap-1 text-sm text-gray-500 mb-4" aria-label="Breadcrumb" data-testid="breadcrumbs">
          <Link to="/dashboard" className="hover:text-sky-600 transition">Dashboard</Link>
          <span className="text-gray-400">›</span>
          <Link to="/health-vault" className="hover:text-sky-600 transition" data-testid="health-vault-breadcrumb">Health Vault</Link>
          <span className="text-gray-400">›</span>
          <span className="text-gray-900 font-medium truncate max-w-xs" data-testid="record-name-breadcrumb">
            {record?.title}
          </span>
        </nav>

        {/* Record Header */}
        <div className="bg-white rounded-xl shadow-sm border border-gray-200 p-6 mb-6">
          <div className="flex items-start justify-between">
            <div className="flex-1">
              <h2 className="text-2xl font-bold text-gray-900" data-testid="record-title">{record?.title}</h2>
              <div className="flex items-center gap-2 mt-2 flex-wrap">
                <span className="px-2 py-0.5 text-xs bg-sky-100 text-sky-700 rounded-full font-medium capitalize">
                  {record?.record_type?.replace(/_/g, ' ')}
                </span>
                {record?.category && (
                  <span className="px-2 py-0.5 text-xs bg-gray-100 text-gray-600 rounded-full">
                    {record.category}
                  </span>
                )}
                {record?.encrypted && (
                  <span
                    className="px-2 py-0.5 text-xs bg-emerald-100 text-emerald-700 rounded-full font-medium"
                    title="This record is encrypted with AES-256-GCM using your subscriber key"
                  >
                    🔒 Encrypted
                  </span>
                )}
                {record?.visibility && record.visibility !== 'all_providers' && (
                  <span className="px-2 py-0.5 text-xs bg-violet-100 text-violet-700 rounded-full font-medium">
                    {visibilityLabel(record.visibility)}
                  </span>
                )}
              </div>
            </div>
            <button
              onClick={handleDownload}
              disabled={downloading}
              className="ml-4 px-4 py-2 bg-sky-600 text-white rounded-lg hover:bg-sky-700 transition text-sm font-medium disabled:opacity-50"
              data-testid="download-btn"
            >
              {downloading ? 'Downloading...' : '⬇ Download'}
            </button>
          </div>

          {downloadError && (
            <div className="mt-3 p-2 bg-red-50 border border-red-200 text-red-700 rounded text-sm">
              {downloadError}
            </div>
          )}
        </div>

        {/* Record Details */}
        <div className="bg-white rounded-xl shadow-sm border border-gray-200 p-6 mb-6">
          <h3 className="text-lg font-semibold text-gray-900 mb-4">Record Details</h3>
          <dl className="space-y-3">
            <div className="flex gap-4">
              <dt className="text-sm text-gray-500 w-36 flex-shrink-0">Date</dt>
              <dd className="text-sm text-gray-900">{formatDate(record?.created_at)}</dd>
            </div>
            {record?.file_size && (
              <div className="flex gap-4">
                <dt className="text-sm text-gray-500 w-36 flex-shrink-0">File Size</dt>
                <dd className="text-sm text-gray-900">{formatFileSize(record.file_size)}</dd>
              </div>
            )}
            <div className="flex gap-4">
              <dt className="text-sm text-gray-500 w-36 flex-shrink-0">Visibility</dt>
              <dd className="text-sm text-gray-900">{visibilityLabel(record?.visibility || 'all_providers')}</dd>
            </div>
            {record?.annotation && (
              <div className="flex gap-4">
                <dt className="text-sm text-gray-500 w-36 flex-shrink-0">Note</dt>
                <dd className="text-sm text-gray-900">📝 {record.annotation}</dd>
              </div>
            )}
          </dl>
        </div>

        {/* Version History */}
        {versions.length > 0 && (
          <div className="bg-white rounded-xl shadow-sm border border-gray-200 p-6 mb-6">
            <h3 className="text-lg font-semibold text-gray-900 mb-4">Version History</h3>
            <div className="space-y-3">
              {versions.map((v, idx) => (
                <div key={v.id} className="flex items-center justify-between p-3 border border-gray-100 rounded-lg hover:bg-gray-50">
                  <div>
                    <div className="flex items-center gap-2">
                      <span className="text-sm font-medium text-gray-900">
                        v{v.version_number} — {v.title}
                      </span>
                      {idx === 0 && (
                        <span className="px-1.5 py-0.5 text-xs bg-sky-100 text-sky-700 rounded font-medium">Latest</span>
                      )}
                      {idx === versions.length - 1 && versions.length > 1 && (
                        <span className="px-1.5 py-0.5 text-xs bg-gray-100 text-gray-600 rounded font-medium">Original</span>
                      )}
                    </div>
                    <p className="text-xs text-gray-500 mt-0.5">
                      {formatDate(v.created_at)} {v.file_size ? `· ${formatFileSize(v.file_size)}` : ''}
                    </p>
                  </div>
                </div>
              ))}
            </div>
          </div>
        )}

        {/* Back link */}
        <div className="mt-2">
          <Link to="/health-vault" className="text-sm text-sky-600 hover:underline" data-testid="back-to-vault">
            ← Back to Health Vault
          </Link>
        </div>
      </main>
    </div>
  );
}
