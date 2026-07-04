import React, { useState, useEffect, useRef } from 'react';
import { useAuth } from '../context/AuthContext';
import { useNavigate, Link, useSearchParams } from 'react-router-dom';
import api from '../services/api';
import Navbar from '../components/Navbar';

// Sanitize a string value from URL params to prevent XSS/injection attempts
// Strips HTML tags, script content, and limits length
function sanitizeUrlParam(value) {
  if (!value || typeof value !== 'string') return '';
  // Strip HTML tags (React will also escape, but sanitize for defense-in-depth)
  let sanitized = value.replace(/<[^>]*>/g, '');
  // Remove script-like patterns
  sanitized = sanitized.replace(/javascript:/gi, '');
  sanitized = sanitized.replace(/on\w+\s*=/gi, '');
  // Limit length to prevent abuse
  if (sanitized.length > 500) sanitized = sanitized.slice(0, 500);
  return sanitized;
}

// Session storage key for persisting filter state within session
const FILTER_STORAGE_KEY = 'livesafe_health_vault_filters';

function getSessionFilters() {
  try {
    const stored = sessionStorage.getItem(FILTER_STORAGE_KEY);
    if (stored) return JSON.parse(stored);
  } catch (e) { /* ignore */ }
  return { searchText: '', typeFilter: 'all', categoryFilter: 'all', dateFrom: '', dateTo: '' };
}

function setSessionFilters(filters) {
  try {
    sessionStorage.setItem(FILTER_STORAGE_KEY, JSON.stringify(filters));
  } catch (e) { /* ignore */ }
}

function Records() {
  const { user } = useAuth();
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
  const fileInputRef = useRef(null);
  // Feature #274: Ref-based guard to prevent rapid double-click uploads (works even before React re-render)
  const isUploadingRef = useRef(false);

  // Read sanitized ?search= URL parameter for initial search filter
  // Sanitization prevents XSS/injection from URL manipulation
  const urlSearchParam = sanitizeUrlParam(searchParams.get('search') || '');

  // Restore filter state from session storage (URL param takes priority)
  const savedFilters = getSessionFilters();
  const [searchText, setSearchText] = useState(urlSearchParam || savedFilters.searchText || '');
  const [typeFilter, setTypeFilter] = useState(savedFilters.typeFilter || 'all');
  const [categoryFilter, setCategoryFilter] = useState(savedFilters.categoryFilter || 'all');
  const [dateFrom, setDateFrom] = useState(savedFilters.dateFrom || '');
  const [dateTo, setDateTo] = useState(savedFilters.dateTo || '');
  const [filtersRestored, setFiltersRestored] = useState(
    !!(urlSearchParam || savedFilters.searchText || savedFilters.typeFilter !== 'all' || savedFilters.categoryFilter !== 'all' ||
    savedFilters.dateFrom || savedFilters.dateTo)
  );

  // Feature #367: Sort by date (ascending or descending)
  const [sortBy, setSortBy] = useState('date_desc'); // 'date_desc' = newest first, 'date_asc' = oldest first

  // Track if search came from URL param (for display purposes)
  const [urlParamSearchActive] = useState(!!urlSearchParam);

  const [records, setRecords] = useState([]);
  const [loadingRecords, setLoadingRecords] = useState(true);

  // Upload state
  const [selectedFile, setSelectedFile] = useState(null);
  const [title, setTitle] = useState('');
  const [recordType, setRecordType] = useState('general');
  const [category, setCategory] = useState('uncategorized');
  // Feature #283: Default visibility to 'private' (most restrictive) for new records
  const [uploadVisibility, setUploadVisibility] = useState('private');
  const [uploading, setUploading] = useState(false);
  const [uploadProgress, setUploadProgress] = useState(0);
  const [uploadComplete, setUploadComplete] = useState(false);
  const [uploadError, setUploadError] = useState('');

  // HIPAA Records Request state
  const [providers, setProviders] = useState([]);
  const [recordRequests, setRecordRequests] = useState([]);
  const [loadingRequests, setLoadingRequests] = useState(true);
  const [requestMode, setRequestMode] = useState('provider'); // 'provider' or 'manual'
  const [selectedProviderId, setSelectedProviderId] = useState('');
  const [manualProviderName, setManualProviderName] = useState('');
  const [manualProviderNpi, setManualProviderNpi] = useState('');
  const [manualProviderAddress, setManualProviderAddress] = useState('');
  const [requestSubmitting, setRequestSubmitting] = useState(false);
  const [requestSuccess, setRequestSuccess] = useState('');
  const [requestError, setRequestError] = useState('');

  // Request tracker status update state (Feature #89)
  const [updatingStatus, setUpdatingStatus] = useState({}); // { [reqId]: 'loading' | 'saved' | 'error' }
  const [statusUpdateMsg, setStatusUpdateMsg] = useState({}); // { [reqId]: message }

  // Delete audit trail state (Feature #97)
  const [deleteMessage, setDeleteMessage] = useState('');

  // Custom confirmation dialog state (Feature #293)
  // Replaces window.confirm() with a React modal that has data-testid attributes for UI testing
  const [confirmDeleteDialog, setConfirmDeleteDialog] = useState(null); // { recordId, recordTitle }

  // Duplicate detection dialog state (Feature #386)
  const [duplicateDialog, setDuplicateDialog] = useState(null); // { existingRecord: {...}, pendingFormData: FormData }

  // Ref to track records currently being deleted (Feature #267)
  // Prevents rapid-click duplicates — synchronous Set, no batching issues
  const deletingRecordsRef = useRef(new Set());

  // Focus management refs for confirm delete dialog (Feature #357)
  const dialogRef = useRef(null);
  const deleteTriggerRef = useRef(null); // element that opened the dialog (focus restored on close)

  // Version history state (Feature #96)
  const [uploadingVersionFor, setUploadingVersionFor] = useState(null); // record object
  const [versionFile, setVersionFile] = useState(null);
  const [versionTitle, setVersionTitle] = useState('');
  const [versionUploading, setVersionUploading] = useState(false);
  const [versionUploadProgress, setVersionUploadProgress] = useState(0);
  const [versionUploadComplete, setVersionUploadComplete] = useState(false);
  const [versionUploadError, setVersionUploadError] = useState('');
  const [versionFileRef] = useState(React.createRef());
  const [expandedVersions, setExpandedVersions] = useState({}); // { [recordId]: versionData }
  const [loadingVersions, setLoadingVersions] = useState({});

  // Annotation & Category editing state (Feature #94)
  const [editingRecordId, setEditingRecordId] = useState(null);
  const [editAnnotation, setEditAnnotation] = useState('');
  const [editCategory, setEditCategory] = useState('');
  const [savingAnnotation, setSavingAnnotation] = useState(false);
  const [annotationSaveMsg, setAnnotationSaveMsg] = useState({});

  // Visibility settings state (Feature #95)
  const [visibilityRecordId, setVisibilityRecordId] = useState(null);
  const [editVisibility, setEditVisibility] = useState('all_providers');
  const [editVisibilityProviders, setEditVisibilityProviders] = useState([]);
  const [savingVisibility, setSavingVisibility] = useState(false);
  const [visibilitySaveMsg, setVisibilitySaveMsg] = useState({});

  // Clinical notes state (Feature #107)
  const [clinicalNotes, setClinicalNotes] = useState([]);
  const [loadingClinicalNotes, setLoadingClinicalNotes] = useState(true);
  const [noteActionMsg, setNoteActionMsg] = useState({});
  const [noteActionError, setNoteActionError] = useState({});

  // Provider access grants state (Feature #156)
  const [providerGrants, setProviderGrants] = useState([]);
  const [loadingGrants, setLoadingGrants] = useState(true);

  // Search length limit for Feature #220 (very long strings handled gracefully)
  const SEARCH_MAX_LENGTH = 500;

  // Pagination state (Feature #221)
  const RECORDS_PER_PAGE = 5;
  const [currentPage, setCurrentPage] = useState(1);

  // Persist filters whenever they change
  const updateSearchText = (val) => {
    // Feature #220: Truncate search string if over max length to prevent issues
    const trimmed = val.length > SEARCH_MAX_LENGTH ? val.slice(0, SEARCH_MAX_LENGTH) : val;
    setSearchText(trimmed);
    setCurrentPage(1); // Reset pagination on filter change (Feature #221)
    setFiltersRestored(false);
    setSessionFilters({ searchText: trimmed, typeFilter, categoryFilter, dateFrom, dateTo });
  };

  const updateTypeFilter = (val) => {
    setTypeFilter(val);
    setCurrentPage(1); // Reset pagination on filter change (Feature #221)
    setFiltersRestored(false);
    setSessionFilters({ searchText, typeFilter: val, categoryFilter, dateFrom, dateTo });
  };

  const updateCategoryFilter = (val) => {
    setCategoryFilter(val);
    setCurrentPage(1); // Reset pagination on filter change (Feature #221)
    setFiltersRestored(false);
    setSessionFilters({ searchText, typeFilter, categoryFilter: val, dateFrom, dateTo });
  };

  const updateDateFrom = (val) => {
    setDateFrom(val);
    setCurrentPage(1); // Reset pagination on filter change (Feature #221)
    setFiltersRestored(false);
    setSessionFilters({ searchText, typeFilter, categoryFilter, dateFrom: val, dateTo });
  };

  const updateDateTo = (val) => {
    setDateTo(val);
    setCurrentPage(1); // Reset pagination on filter change (Feature #221)
    setFiltersRestored(false);
    setSessionFilters({ searchText, typeFilter, categoryFilter, dateFrom, dateTo: val });
  };

  const clearFilters = () => {
    setSearchText('');
    setTypeFilter('all');
    setCategoryFilter('all');
    setDateFrom('');
    setDateTo('');
    setCurrentPage(1); // Reset pagination on clear (Feature #221, #222)
    setFiltersRestored(false);
    setSessionFilters({ searchText: '', typeFilter: 'all', categoryFilter: 'all', dateFrom: '', dateTo: '' });
  };

  const hasActiveFilters = searchText.trim() !== '' || typeFilter !== 'all' || categoryFilter !== 'all' || dateFrom !== '' || dateTo !== '';

  // Compute filtered records
  const filteredRecords = records.filter((record) => {
    // Search filter (whitespace-only input treated as no filter - shows all results)
    if (searchText.trim()) {
      const q = searchText.toLowerCase();
      const matchTitle = record.title?.toLowerCase().includes(q);
      const matchType = record.record_type?.toLowerCase().includes(q);
      const matchCategory = record.category?.toLowerCase().includes(q);
      if (!matchTitle && !matchType && !matchCategory) return false;
    }
    // Type filter
    if (typeFilter !== 'all' && record.record_type !== typeFilter) return false;
    // Category filter
    if (categoryFilter !== 'all' && record.category !== categoryFilter) return false;
    // Date range filter
    if (dateFrom || dateTo) {
      const recordDate = record.created_at ? new Date(record.created_at) : null;
      if (!recordDate) return false;
      // Compare against start of dateFrom day
      if (dateFrom) {
        const fromDate = new Date(dateFrom + 'T00:00:00');
        if (recordDate < fromDate) return false;
      }
      // Compare against end of dateTo day
      if (dateTo) {
        const toDate = new Date(dateTo + 'T23:59:59.999');
        if (recordDate > toDate) return false;
      }
    }
    return true;
  });

  // Feature #367: Sort filtered records by date (works correctly across month/year boundaries
  // since JS Date comparison is numeric — avoids string comparison pitfalls)
  const sortedRecords = [...filteredRecords].sort((a, b) => {
    const aTime = a.created_at ? new Date(a.created_at).getTime() : 0;
    const bTime = b.created_at ? new Date(b.created_at).getTime() : 0;
    return sortBy === 'date_asc' ? aTime - bTime : bTime - aTime;
  });

  // Pagination computation (Feature #221)
  const totalPages = Math.max(1, Math.ceil(sortedRecords.length / RECORDS_PER_PAGE));
  const safePage = Math.min(currentPage, totalPages);
  const paginatedRecords = sortedRecords.slice((safePage - 1) * RECORDS_PER_PAGE, safePage * RECORDS_PER_PAGE);

  const fetchRecords = async () => {
    try {
      setLoadingRecords(true);
      const res = await api.get('/records');
      setRecords(res.data);
    } catch (err) {
      console.error('Failed to fetch records:', err);
    } finally {
      setLoadingRecords(false);
    }
  };

  const fetchProviders = async () => {
    try {
      const res = await api.get('/records/providers');
      setProviders(res.data);
    } catch (err) {
      console.error('Failed to fetch providers:', err);
    }
  };

  const fetchRecordRequests = async () => {
    try {
      setLoadingRequests(true);
      const res = await api.get('/records/requests');
      setRecordRequests(res.data);
    } catch (err) {
      console.error('Failed to fetch record requests:', err);
    } finally {
      setLoadingRequests(false);
    }
  };

  const fetchClinicalNotes = async () => {
    try {
      setLoadingClinicalNotes(true);
      const res = await api.get('/records/clinical-notes/subscriber');
      setClinicalNotes(res.data?.notes || []);
    } catch (err) {
      console.error('Failed to fetch clinical notes:', err);
    } finally {
      setLoadingClinicalNotes(false);
    }
  };

  const handleApproveNote = async (noteId) => {
    setNoteActionMsg(prev => ({ ...prev, [noteId]: '' }));
    setNoteActionError(prev => ({ ...prev, [noteId]: '' }));
    try {
      await api.patch(`/records/clinical-notes/${noteId}/approve`);
      setNoteActionMsg(prev => ({ ...prev, [noteId]: '✅ Note approved and added to your record' }));
      fetchClinicalNotes();
    } catch (err) {
      setNoteActionError(prev => ({ ...prev, [noteId]: err.response?.data?.error || 'Failed to approve note' }));
    }
  };

  const handleRejectNote = async (noteId) => {
    setNoteActionMsg(prev => ({ ...prev, [noteId]: '' }));
    setNoteActionError(prev => ({ ...prev, [noteId]: '' }));
    try {
      await api.patch(`/records/clinical-notes/${noteId}/reject`, { reason: 'Subscriber declined' });
      setNoteActionMsg(prev => ({ ...prev, [noteId]: '❌ Note rejected' }));
      fetchClinicalNotes();
    } catch (err) {
      setNoteActionError(prev => ({ ...prev, [noteId]: err.response?.data?.error || 'Failed to reject note' }));
    }
  };

  const fetchProviderGrants = async () => {
    try {
      setLoadingGrants(true);
      const res = await api.get('/consent/my-consents');
      const grants = Array.isArray(res.data) ? res.data : [];
      // Only show active (not revoked, not expired)
      setProviderGrants(grants.filter(g => !g.revoked_at && (!g.expires_at || new Date(g.expires_at) > new Date())));
    } catch (err) {
      console.error('Failed to fetch provider grants:', err);
      setProviderGrants([]);
    } finally {
      setLoadingGrants(false);
    }
  };

  useEffect(() => {
    fetchRecords();
    fetchProviders();
    fetchRecordRequests();
    fetchClinicalNotes();
    fetchProviderGrants();
  }, []);

  // Feature #357: Focus trap for confirm delete dialog
  // When dialog opens: focus first focusable element
  // When dialog closes: restore focus to trigger element
  useEffect(() => {
    if (!confirmDeleteDialog) return;
    // Focus the first focusable element in the dialog
    if (dialogRef.current) {
      const focusable = dialogRef.current.querySelectorAll(
        'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])'
      );
      if (focusable.length > 0) {
        focusable[0].focus();
      }
    }
  }, [confirmDeleteDialog]);

  const ALLOWED_UPLOAD_EXTS = ['pdf', 'jpg', 'jpeg', 'png', 'gif', 'txt', 'doc', 'docx', 'json', 'xml'];
  const MAX_UPLOAD_SIZE_MB = 50;
  const MAX_UPLOAD_SIZE_BYTES = MAX_UPLOAD_SIZE_MB * 1024 * 1024;

  const handleFileSelect = (e) => {
    const file = e.target.files[0];
    if (file) {
      // Client-side file type validation
      const ext = file.name.toLowerCase().split('.').pop();
      if (!ALLOWED_UPLOAD_EXTS.includes(ext)) {
        setUploadError(
          `Unsupported file format ".${ext}". Accepted formats: PDF, JPEG, PNG, GIF, TXT, DOC, DOCX, JSON, XML`
        );
        setSelectedFile(null);
        if (fileInputRef.current) fileInputRef.current.value = '';
        return;
      }

      // Client-side file size validation
      if (file.size > MAX_UPLOAD_SIZE_BYTES) {
        const sizeMB = (file.size / 1024 / 1024).toFixed(1);
        setUploadError(
          `File too large (${sizeMB} MB). Maximum allowed size is ${MAX_UPLOAD_SIZE_MB} MB.`
        );
        setSelectedFile(null);
        if (fileInputRef.current) fileInputRef.current.value = '';
        return;
      }

      setSelectedFile(file);
      if (!title) {
        setTitle(file.name.replace(/\.[^/.]+$/, ''));
      }
      setUploadComplete(false);
      setUploadError('');
    }
  };

  const MAX_FILE_SIZE = 50 * 1024 * 1024; // 50MB - matches server limit

  const handleRetryUpload = () => {
    setUploadError('');
    setSelectedFile(null);
    setTitle('');
    if (fileInputRef.current) fileInputRef.current.value = '';
  };

  const handleUpload = async (e) => {
    e.preventDefault();

    if (!selectedFile) {
      setUploadError('Please select a file to upload');
      return;
    }
    // Client-side file size validation for clear error feedback
    if (selectedFile.size > MAX_FILE_SIZE) {
      const sizeMb = (selectedFile.size / (1024 * 1024)).toFixed(1);
      setUploadError(`File is too large (${sizeMb}MB). Maximum allowed size is 50MB. Please choose a smaller file.`);
      return;
    }
    if (!title.trim()) {
      setUploadError('Please enter a record title');
      return;
    }

    // Feature #274: Prevent double-click uploads using ref (fires before React re-render)
    if (isUploadingRef.current) return;
    isUploadingRef.current = true;

    setUploading(true);
    setUploadProgress(0);
    setUploadComplete(false);
    setUploadError('');

    const formData = new FormData();
    formData.append('file', selectedFile);
    formData.append('title', title.trim());
    formData.append('record_type', recordType);
    formData.append('category', category);
    formData.append('visibility', uploadVisibility);

    try {
      const token = localStorage.getItem('livesafe_token');
      const xhr = new XMLHttpRequest();

      await new Promise((resolve, reject) => {
        xhr.upload.addEventListener('progress', (event) => {
          if (event.lengthComputable) {
            const percent = Math.round((event.loaded / event.total) * 100);
            setUploadProgress(percent);
          }
        });

        xhr.addEventListener('load', () => {
          if (xhr.status >= 200 && xhr.status < 300) {
            resolve(JSON.parse(xhr.responseText));
          } else if (xhr.status === 409) {
            try {
              const dupData = JSON.parse(xhr.responseText);
              if (dupData.duplicate) {
                const dupErr = new Error('DUPLICATE_DETECTED');
                dupErr.duplicateData = dupData;
                dupErr.pendingFormData = formData;
                reject(dupErr);
              } else {
                reject(new Error(dupData.error || 'Upload failed'));
              }
            } catch {
              reject(new Error('Upload failed'));
            }
          } else {
            try {
              const errData = JSON.parse(xhr.responseText);
              reject(new Error(errData.error || 'Upload failed'));
            } catch {
              reject(new Error('Upload failed'));
            }
          }
        });

        xhr.addEventListener('error', () => reject(new Error('Network error during upload')));
        xhr.addEventListener('abort', () => reject(new Error('Upload cancelled')));

        xhr.open('POST', '/api/records/upload');
        xhr.setRequestHeader('Authorization', `Bearer ${token}`);
        xhr.send(formData);
      });

      setUploadProgress(100);
      setUploadComplete(true);

      // Reset form after short delay
      setTimeout(() => {
        setSelectedFile(null);
        setTitle('');
        setRecordType('general');
        setCategory('uncategorized');
        setUploadVisibility('private'); // Feature #283: reset to private default
        if (fileInputRef.current) fileInputRef.current.value = '';
        fetchRecords();
      }, 2000);
    } catch (err) {
      if (err.message === 'DUPLICATE_DETECTED' && err.duplicateData) {
        // Feature #386: Show duplicate detection dialog
        setDuplicateDialog({
          existingRecord: err.duplicateData.existing_record,
          pendingFormData: err.pendingFormData,
        });
        setUploadError('');
      } else {
        setUploadError(err.message || 'Failed to upload record');
      }
    } finally {
      setUploading(false);
      isUploadingRef.current = false; // Feature #274: release the guard
    }
  };

  // Feature #386: Handle overwrite — re-submit with overwrite=true
  const handleOverwriteDuplicate = async () => {
    if (!duplicateDialog) return;
    const fd = duplicateDialog.pendingFormData;
    fd.append('overwrite', 'true');
    setDuplicateDialog(null);

    // Re-upload with overwrite flag
    isUploadingRef.current = true;
    setUploading(true);
    setUploadProgress(0);
    setUploadComplete(false);
    setUploadError('');
    try {
      const token = localStorage.getItem('livesafe_token');
      const xhr = new XMLHttpRequest();
      await new Promise((resolve, reject) => {
        xhr.upload.addEventListener('progress', (event) => {
          if (event.lengthComputable) {
            setUploadProgress(Math.round((event.loaded / event.total) * 100));
          }
        });
        xhr.addEventListener('load', () => {
          if (xhr.status >= 200 && xhr.status < 300) {
            resolve(JSON.parse(xhr.responseText));
          } else {
            try { reject(new Error(JSON.parse(xhr.responseText).error || 'Upload failed')); }
            catch { reject(new Error('Upload failed')); }
          }
        });
        xhr.addEventListener('error', () => reject(new Error('Network error during upload')));
        xhr.open('POST', '/api/records/upload');
        xhr.setRequestHeader('Authorization', `Bearer ${token}`);
        xhr.send(fd);
      });
      setUploadProgress(100);
      setUploadComplete(true);
      setTimeout(() => {
        setSelectedFile(null);
        setTitle('');
        setRecordType('general');
        setCategory('uncategorized');
        setUploadVisibility('private'); // Feature #283: reset to private default
        if (fileInputRef.current) fileInputRef.current.value = '';
        fetchRecords();
      }, 2000);
    } catch (err) {
      setUploadError(err.message || 'Failed to upload record');
    } finally {
      setUploading(false);
      isUploadingRef.current = false;
    }
  };

  // Feature #386: Skip duplicate — just dismiss dialog and clear form
  const handleSkipDuplicate = () => {
    setDuplicateDialog(null);
    setSelectedFile(null);
    setTitle('');
    setRecordType('general');
    setCategory('uncategorized');
    setUploadVisibility('private'); // Feature #283: reset to private default
    if (fileInputRef.current) fileInputRef.current.value = '';
  };

  // Step 1: Show confirmation dialog (Feature #293 - replaces window.confirm)
  const handleDelete = (recordId, recordTitle) => {
    // Save the element that triggered the dialog so we can restore focus on close (Feature #357)
    deleteTriggerRef.current = document.activeElement;
    setConfirmDeleteDialog({ recordId, recordTitle: recordTitle || 'this record' });
  };

  // Step 2: User confirmed deletion via custom dialog
  const handleConfirmDelete = async () => {
    if (!confirmDeleteDialog) return;
    const { recordId, recordTitle } = confirmDeleteDialog;
    setConfirmDeleteDialog(null);
    // Clear trigger ref since the deleted record's button will be removed from DOM
    deleteTriggerRef.current = null;

    // Prevent rapid/duplicate delete clicks (Feature #267)
    if (deletingRecordsRef.current.has(recordId)) {
      return; // Already being deleted — silently ignore
    }
    deletingRecordsRef.current.add(recordId);

    try {
      const res = await api.delete(`/records/${recordId}`);
      fetchRecords();
      // Show audit trail confirmation
      const receipt = res.data?.audit_receipt;
      if (receipt) {
        setDeleteMessage(`Record deleted. Audit receipt preserved: ${receipt}`);
        setTimeout(() => setDeleteMessage(''), 6000);
      }
    } catch (err) {
      // Handle 404 gracefully: record already deleted — not an error for the user
      if (err.response?.status === 404) {
        fetchRecords(); // Refresh to reflect current state
      } else {
        console.error('Failed to delete record:', err);
      }
    } finally {
      deletingRecordsRef.current.delete(recordId);
    }
  };

  // Step 2b: User cancelled deletion via custom dialog
  const handleCancelDelete = () => {
    setConfirmDeleteDialog(null);
    // Restore focus to the delete button that triggered the dialog (Feature #357)
    if (deleteTriggerRef.current) {
      deleteTriggerRef.current.focus();
      deleteTriggerRef.current = null;
    }
  };

  // Feature #357: Keyboard handler to trap focus inside the confirmation dialog
  const handleDialogKeyDown = (e) => {
    if (!dialogRef.current) return;
    const focusable = dialogRef.current.querySelectorAll(
      'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])'
    );
    if (focusable.length === 0) return;
    const first = focusable[0];
    const last = focusable[focusable.length - 1];

    if (e.key === 'Tab') {
      if (e.shiftKey) {
        // Shift+Tab: wrap from first to last
        if (document.activeElement === first) {
          e.preventDefault();
          last.focus();
        }
      } else {
        // Tab: wrap from last to first
        if (document.activeElement === last) {
          e.preventDefault();
          first.focus();
        }
      }
    }
    // Escape closes the dialog and restores focus
    if (e.key === 'Escape') {
      handleCancelDelete();
    }
  };

  // ── Version upload handler (Feature #96) ────────────────────────────────
  const handleVersionUpload = async (e) => {
    e.preventDefault();
    if (!versionFile) {
      setVersionUploadError('Please select a file to upload');
      return;
    }
    if (!versionTitle.trim()) {
      setVersionUploadError('Please enter a version title');
      return;
    }
    setVersionUploading(true);
    setVersionUploadProgress(0);
    setVersionUploadComplete(false);
    setVersionUploadError('');

    const formData = new FormData();
    formData.append('file', versionFile);
    formData.append('title', versionTitle.trim());
    formData.append('record_type', uploadingVersionFor.record_type || 'general');
    formData.append('category', uploadingVersionFor.category || 'uncategorized');
    formData.append('parent_record_id', String(uploadingVersionFor.id));

    try {
      const token = localStorage.getItem('livesafe_token');
      const xhr = new XMLHttpRequest();

      await new Promise((resolve, reject) => {
        xhr.upload.addEventListener('progress', (event) => {
          if (event.lengthComputable) {
            const percent = Math.round((event.loaded / event.total) * 100);
            setVersionUploadProgress(percent);
          }
        });
        xhr.addEventListener('load', () => {
          if (xhr.status >= 200 && xhr.status < 300) {
            resolve(JSON.parse(xhr.responseText));
          } else {
            try {
              const errData = JSON.parse(xhr.responseText);
              reject(new Error(errData.error || 'Upload failed'));
            } catch {
              reject(new Error('Upload failed'));
            }
          }
        });
        xhr.addEventListener('error', () => reject(new Error('Network error during upload')));
        xhr.addEventListener('abort', () => reject(new Error('Upload cancelled')));
        xhr.open('POST', '/api/records/upload');
        xhr.setRequestHeader('Authorization', `Bearer ${token}`);
        xhr.send(formData);
      });

      setVersionUploadProgress(100);
      setVersionUploadComplete(true);
      setTimeout(() => {
        setUploadingVersionFor(null);
        setVersionFile(null);
        setVersionTitle('');
        setVersionUploading(false);
        setVersionUploadComplete(false);
        fetchRecords();
      }, 1500);
    } catch (err) {
      setVersionUploadError(err.message || 'Failed to upload version');
    } finally {
      setVersionUploading(false);
    }
  };

  // ── Fetch version history for a record (Feature #96) ─────────────────────
  const fetchVersionHistory = async (record) => {
    const recordId = record.id;
    if (expandedVersions[recordId]) {
      // Toggle off
      setExpandedVersions((prev) => { const n = {...prev}; delete n[recordId]; return n; });
      return;
    }
    setLoadingVersions((prev) => ({ ...prev, [recordId]: true }));
    try {
      const res = await api.get(`/records/${recordId}/versions`);
      setExpandedVersions((prev) => ({ ...prev, [recordId]: res.data }));
    } catch (err) {
      console.error('Failed to fetch version history:', err);
    } finally {
      setLoadingVersions((prev) => { const n = {...prev}; delete n[recordId]; return n; });
    }
  };

  const handleRequestRecords = async (e) => {
    e.preventDefault();
    setRequestError('');
    setRequestSuccess('');

    let providerName = '';
    let providerNpi = '';
    let providerAddress = '';

    if (requestMode === 'provider') {
      if (!selectedProviderId) {
        setRequestError('Please select a provider');
        return;
      }
      const provider = providers.find(p => String(p.id) === String(selectedProviderId));
      if (!provider) {
        setRequestError('Selected provider not found');
        return;
      }
      providerName = provider.provider_name || provider.facility || 'Unknown Provider';
      providerNpi = provider.npi || '';
      providerAddress = provider.facility || '';
    } else {
      if (!manualProviderName.trim()) {
        setRequestError('Provider name is required');
        return;
      }
      providerName = manualProviderName.trim();
      providerNpi = manualProviderNpi.trim();
      providerAddress = manualProviderAddress.trim();
    }

    setRequestSubmitting(true);
    try {
      const res = await api.post('/records/request', {
        provider_name: providerName,
        provider_npi: providerNpi || null,
        provider_address: providerAddress || null
      });

      setRequestSuccess(`HIPAA Right of Access request sent to ${providerName}! Status: ${res.data.status}`);
      // Reset form
      setSelectedProviderId('');
      setManualProviderName('');
      setManualProviderNpi('');
      setManualProviderAddress('');
      fetchRecordRequests();
    } catch (err) {
      setRequestError(err.response?.data?.error || 'Failed to submit records request');
    } finally {
      setRequestSubmitting(false);
    }
  };

  const formatFileSize = (bytes) => {
    if (!bytes) return 'N/A';
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  };

  const getStatusBadge = (status) => {
    const styles = {
      sent: 'bg-amber-100 text-amber-700',
      pending: 'bg-blue-100 text-blue-700',
      received: 'bg-emerald-100 text-emerald-700',
      failed: 'bg-red-100 text-red-700',
      denied: 'bg-red-100 text-red-700',
      processing: 'bg-sky-100 text-sky-700',
    };
    return styles[status] || 'bg-gray-100 text-gray-700';
  };

  const getStatusIcon = (status) => {
    const icons = {
      sent: '📤',
      pending: '⏳',
      received: '✅',
      failed: '❌',
      denied: '🚫',
      processing: '🔄',
    };
    return icons[status] || '📋';
  };

  // Feature #89: Update request status
  const handleUpdateRequestStatus = async (reqId, newStatus) => {
    setUpdatingStatus(prev => ({ ...prev, [reqId]: 'loading' }));
    setStatusUpdateMsg(prev => ({ ...prev, [reqId]: '' }));
    try {
      await api.patch(`/records/requests/${reqId}/status`, { status: newStatus });
      setStatusUpdateMsg(prev => ({ ...prev, [reqId]: `Status updated to "${newStatus}"` }));
      setUpdatingStatus(prev => ({ ...prev, [reqId]: 'saved' }));
      await fetchRecordRequests();
      // Clear message after 3 seconds
      setTimeout(() => {
        setStatusUpdateMsg(prev => ({ ...prev, [reqId]: '' }));
        setUpdatingStatus(prev => ({ ...prev, [reqId]: null }));
      }, 3000);
    } catch (err) {
      setStatusUpdateMsg(prev => ({ ...prev, [reqId]: err.response?.data?.error || 'Update failed' }));
      setUpdatingStatus(prev => ({ ...prev, [reqId]: 'error' }));
    }
  };

  // ── Feature #94: Open annotation/category editor ─────────────────────────
  const openAnnotationEditor = (record) => {
    setEditingRecordId(record.id);
    setEditAnnotation(record.annotation || '');
    setEditCategory(record.category || 'uncategorized');
    // Close visibility editor if open
    if (visibilityRecordId === record.id) setVisibilityRecordId(null);
  };

  const handleAnnotationSave = async (recordId) => {
    setSavingAnnotation(true);
    try {
      await api.patch(`/records/${recordId}`, {
        annotation: editAnnotation,
        category: editCategory,
      });
      setAnnotationSaveMsg(prev => ({ ...prev, [recordId]: 'Saved!' }));
      setEditingRecordId(null);
      fetchRecords();
      setTimeout(() => setAnnotationSaveMsg(prev => ({ ...prev, [recordId]: '' })), 3000);
    } catch (err) {
      setAnnotationSaveMsg(prev => ({ ...prev, [recordId]: err.response?.data?.error || 'Save failed' }));
    } finally {
      setSavingAnnotation(false);
    }
  };

  // ── Feature #95: Open visibility settings ────────────────────────────────
  const openVisibilitySettings = (record) => {
    setVisibilityRecordId(record.id);
    setEditVisibility(record.visibility || 'all_providers');
    setEditVisibilityProviders(record.visibility_providers || []);
    // Close annotation editor if open
    if (editingRecordId === record.id) setEditingRecordId(null);
  };

  const handleVisibilitySave = async (recordId) => {
    setSavingVisibility(true);
    try {
      const body = { visibility: editVisibility };
      if (editVisibility === 'specific_providers') {
        if (editVisibilityProviders.length === 0) {
          setVisibilitySaveMsg(prev => ({ ...prev, [recordId]: 'Select at least one provider' }));
          setSavingVisibility(false);
          return;
        }
        body.visibility_providers = editVisibilityProviders;
      }
      await api.patch(`/records/${recordId}/visibility`, body);
      setVisibilitySaveMsg(prev => ({ ...prev, [recordId]: 'Visibility saved!' }));
      setVisibilityRecordId(null);
      fetchRecords();
      setTimeout(() => setVisibilitySaveMsg(prev => ({ ...prev, [recordId]: '' })), 3000);
    } catch (err) {
      setVisibilitySaveMsg(prev => ({ ...prev, [recordId]: err.response?.data?.error || 'Save failed' }));
    } finally {
      setSavingVisibility(false);
    }
  };

  const getVisibilityLabel = (visibility) => {
    const labels = {
      all_providers: '👁️ All Providers',
      specific_providers: '👥 Specific Providers',
      emergency_only: '🚨 Emergency Only',
      private: '🔒 Private',
    };
    return labels[visibility] || visibility || '👁️ All Providers';
  };

  return (
    <div className="min-h-screen bg-gray-50">
      <Navbar />

      {/* Feature #386: Duplicate Record Detection Dialog */}
      {duplicateDialog && (
        <div
          className="fixed inset-0 z-50 flex items-center justify-center bg-black bg-opacity-50"
          data-testid="duplicate-record-dialog"
          role="dialog"
          aria-modal="true"
          aria-labelledby="duplicate-dialog-title"
        >
          <div className="bg-white rounded-xl shadow-xl max-w-md w-full mx-4 p-6">
            <div className="flex items-center gap-3 mb-3">
              <div className="w-10 h-10 rounded-full bg-amber-100 flex items-center justify-center flex-shrink-0">
                <svg className="w-5 h-5 text-amber-600" fill="none" viewBox="0 0 24 24" stroke="currentColor" aria-hidden="true">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 9v2m0 4h.01M12 3a9 9 0 100 18A9 9 0 0012 3z" />
                </svg>
              </div>
              <h3 id="duplicate-dialog-title" className="text-lg font-semibold text-gray-900">
                Duplicate Record Detected
              </h3>
            </div>
            <p className="text-sm text-gray-600 mb-2" data-testid="duplicate-dialog-message">
              This file has already been uploaded as{' '}
              <span className="font-semibold text-gray-900" data-testid="duplicate-existing-title">
                &ldquo;{duplicateDialog.existingRecord?.title}&rdquo;
              </span>
              {duplicateDialog.existingRecord?.created_at && (
                <> on {new Date(duplicateDialog.existingRecord.created_at).toLocaleDateString()}</>
              )}
              .
            </p>
            <p className="text-sm text-gray-500 mb-5">
              Would you like to skip this upload or overwrite the existing record?
            </p>
            <div className="flex justify-end gap-3">
              <button
                onClick={handleSkipDuplicate}
                className="px-4 py-2 text-sm font-medium text-gray-700 bg-white border border-gray-300 rounded-lg hover:bg-gray-50 transition"
                data-testid="duplicate-skip-btn"
              >
                Skip
              </button>
              <button
                onClick={handleOverwriteDuplicate}
                className="px-4 py-2 text-sm font-medium text-white bg-sky-700 rounded-lg hover:bg-sky-800 transition"
                data-testid="duplicate-overwrite-btn"
              >
                Overwrite
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Custom Confirmation Dialog for Destructive Actions (Feature #293) */}
      {confirmDeleteDialog && (
        <div
          className="fixed inset-0 z-50 flex items-center justify-center bg-black bg-opacity-50"
          data-testid="confirm-delete-dialog"
          role="dialog"
          aria-modal="true"
          aria-labelledby="confirm-delete-title"
          ref={dialogRef}
          onKeyDown={handleDialogKeyDown}
        >
          <div className="bg-white rounded-xl shadow-xl max-w-md w-full mx-4 p-6">
            <h3 id="confirm-delete-title" className="text-lg font-semibold text-gray-900 mb-2">
              Confirm Deletion
            </h3>
            <p
              className="text-sm text-gray-600 mb-6"
              data-testid="confirm-delete-message"
            >
              Are you sure you want to delete{' '}
              <span className="font-semibold text-gray-900">
                &ldquo;{confirmDeleteDialog.recordTitle}&rdquo;
              </span>
              ? This action cannot be undone. An audit trail of the deletion will be preserved.
            </p>
            <div className="flex justify-end gap-3">
              <button
                onClick={handleCancelDelete}
                className="px-4 py-2 text-sm font-medium text-gray-700 bg-white border border-gray-300 rounded-lg hover:bg-gray-50 transition"
                data-testid="confirm-delete-cancel-btn"
              >
                Cancel
              </button>
              <button
                onClick={handleConfirmDelete}
                className="px-4 py-2 text-sm font-medium text-white bg-red-600 rounded-lg hover:bg-red-700 transition"
                data-testid="confirm-delete-confirm-btn"
              >
                Delete Record
              </button>
            </div>
          </div>
        </div>
      )}

      <main id="main-content" tabIndex={-1} className="max-w-4xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        {/* Breadcrumbs (Feature #301) */}
        <nav className="flex items-center gap-1 text-sm text-gray-500 mb-4" aria-label="Breadcrumb" data-testid="breadcrumbs">
          <Link to="/dashboard" className="hover:text-sky-600 transition">Dashboard</Link>
          <span className="text-gray-400">›</span>
          <span className="text-gray-900 font-medium">Health Vault</span>
        </nav>

        <h1 className="text-2xl font-bold text-gray-900 mb-6">Medical Records</h1>

        {/* Upload Form */}
        <div className="bg-white rounded-xl shadow-sm border border-gray-200 p-6 mb-8">
          <h2 className="text-lg font-semibold text-gray-900 mb-4">Upload Record</h2>

          <form onSubmit={handleUpload} className="space-y-4">
            {/* File Selection */}
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-2">
                Select File
              </label>
              <div
                className="border-2 border-dashed border-gray-300 rounded-lg p-6 text-center hover:border-sky-400 transition cursor-pointer"
                onClick={() => fileInputRef.current?.click()}
              >
                <input
                  type="file"
                  ref={fileInputRef}
                  onChange={handleFileSelect}
                  className="hidden"
                  accept=".pdf,.jpg,.jpeg,.png,.gif,.txt,.doc,.docx,.json,.xml"
                  data-testid="file-input"
                />
                {selectedFile ? (
                  <div>
                    <p className="text-gray-900 font-medium">{selectedFile.name}</p>
                    <p className="text-sm text-gray-500 mt-1">
                      {formatFileSize(selectedFile.size)} &middot; {selectedFile.type || 'Unknown type'}
                    </p>
                    <p className="text-xs text-sky-700 mt-2">Click to choose a different file</p>
                  </div>
                ) : (
                  <div>
                    <svg className="mx-auto h-10 w-10 text-gray-400 mb-2" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12" />
                    </svg>
                    <p className="text-gray-600">Click to select a file</p>
                    <p className="text-xs text-gray-400 mt-1">PDF, JPEG, PNG, GIF, TXT, DOC, DOCX &mdash; Max 50MB</p>
                  </div>
                )}
              </div>
            </div>

            {/* Record Details */}
            <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">Title *</label>
                <input
                  type="text"
                  value={title}
                  onChange={(e) => setTitle(e.target.value)}
                  placeholder="e.g., Blood Test Results"
                  className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-sky-500"
                  required
                  data-testid="record-title-input"
                />
              </div>
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">Type</label>
                <select
                  value={recordType}
                  onChange={(e) => setRecordType(e.target.value)}
                  className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-sky-500"
                >
                  <option value="general">General</option>
                  <option value="lab_result">Lab Result</option>
                  <option value="imaging">Imaging / X-Ray</option>
                  <option value="clinical_note">Clinical Note</option>
                  <option value="prescription">Prescription</option>
                  <option value="discharge_summary">Discharge Summary</option>
                  <option value="vaccination">Vaccination</option>
                  <option value="allergy">Allergy Record</option>
                  <option value="surgical">Surgical Report</option>
                </select>
              </div>
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">Category</label>
                <select
                  value={category}
                  onChange={(e) => setCategory(e.target.value)}
                  className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-sky-500"
                >
                  <option value="uncategorized">Uncategorized</option>
                  <option value="emergency">Emergency</option>
                  <option value="primary_care">Primary Care</option>
                  <option value="specialist">Specialist</option>
                  <option value="dental">Dental</option>
                  <option value="mental_health">Mental Health</option>
                  <option value="pharmacy">Pharmacy</option>
                </select>
              </div>
            </div>

            {/* Feature #283: Default visibility selector - defaults to Private (most restrictive) */}
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-1">
                Visibility
                <span className="ml-1 text-xs text-gray-400 font-normal">(who can access this record)</span>
              </label>
              <select
                value={uploadVisibility}
                onChange={(e) => setUploadVisibility(e.target.value)}
                className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-sky-500"
                data-testid="upload-visibility-select"
              >
                <option value="private">🔒 Private (Only Me)</option>
                <option value="emergency_only">🚨 Emergency Only</option>
                <option value="all_providers">👁️ All Providers</option>
              </select>
              <p className="text-xs text-gray-500 mt-1">
                {uploadVisibility === 'private' && 'Only you can view this record. No provider access.'}
                {uploadVisibility === 'emergency_only' && 'Accessible to emergency responders only.'}
                {uploadVisibility === 'all_providers' && 'All authorized providers can access this record.'}
              </p>
            </div>

            {/* Progress Indicator */}
            {(uploading || uploadComplete) && (
              <div className="space-y-2" data-testid="upload-progress">
                <div className="flex justify-between items-center">
                  <span className="text-sm font-medium text-gray-700">
                    {uploadComplete ? 'Upload complete!' : 'Uploading...'}
                  </span>
                  <span className="text-sm font-bold text-sky-700" data-testid="progress-percent">
                    {uploadProgress}%
                  </span>
                </div>
                <div className="w-full bg-gray-200 rounded-full h-3 overflow-hidden">
                  <div
                    className={`h-3 rounded-full transition-all duration-300 ease-out ${
                      uploadComplete ? 'bg-emerald-500' : 'bg-sky-500'
                    }`}
                    style={{ width: `${uploadProgress}%` }}
                    data-testid="progress-bar"
                    role="progressbar"
                    aria-valuenow={uploadProgress}
                    aria-valuemin={0}
                    aria-valuemax={100}
                  />
                </div>
                {uploading && !uploadComplete && (
                  <p className="text-xs text-gray-500">
                    Uploading {selectedFile?.name} ({formatFileSize(selectedFile?.size)})...
                  </p>
                )}
              </div>
            )}

            {/* Upload Complete Notification */}
            {uploadComplete && (
              <div className="flex items-center gap-2 p-3 bg-emerald-50 border border-emerald-200 rounded-lg" data-testid="upload-success">
                <svg className="h-5 w-5 text-emerald-500 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 13l4 4L19 7" />
                </svg>
                <p className="text-sm text-emerald-700 font-medium">
                  Record uploaded successfully! Your file has been securely stored.
                </p>
              </div>
            )}

            {/* Error Message */}
            {uploadError && (
              <div className="p-3 bg-red-50 border border-red-200 rounded-lg" data-testid="upload-error">
                <div className="flex items-center gap-2 mb-2">
                  <svg className="h-5 w-5 text-red-500 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                  </svg>
                  <p className="text-sm text-red-700" data-testid="upload-error-message" role="alert">{uploadError}</p>
                </div>
                <button
                  type="button"
                  onClick={handleRetryUpload}
                  className="text-sm text-red-600 underline hover:text-red-800 font-medium"
                  data-testid="upload-retry-button"
                >
                  Try Again
                </button>
              </div>
            )}

            {/* Submit Button */}
            <button
              type="submit"
              disabled={uploading}
              className={`w-full py-3 rounded-lg text-white font-semibold transition ${
                uploading
                  ? 'bg-gray-400 cursor-not-allowed'
                  : 'bg-sky-600 hover:bg-sky-700'
              }`}
              data-testid="upload-button"
            >
              {uploading ? 'Uploading...' : 'Upload Record'}
            </button>
          </form>
        </div>

        {/* HIPAA Right of Access Request */}
        <div className="bg-white rounded-xl shadow-sm border border-gray-200 p-6 mb-8" data-testid="records-request-section">
          <h2 className="text-lg font-semibold text-gray-900 mb-2">Request Records (HIPAA Right of Access)</h2>
          <p className="text-sm text-gray-500 mb-4">
            Under HIPAA, you have the right to request copies of your medical records from any healthcare provider.
          </p>

          <form onSubmit={handleRequestRecords} className="space-y-4">
            {/* Mode Selection */}
            <div className="flex gap-2 mb-4">
              <button
                type="button"
                onClick={() => setRequestMode('provider')}
                className={`px-4 py-2 text-sm rounded-lg transition ${
                  requestMode === 'provider'
                    ? 'bg-sky-600 text-white'
                    : 'bg-gray-100 text-gray-700 hover:bg-gray-200'
                }`}
                data-testid="mode-provider-btn"
              >
                Select Provider
              </button>
              <button
                type="button"
                onClick={() => setRequestMode('manual')}
                className={`px-4 py-2 text-sm rounded-lg transition ${
                  requestMode === 'manual'
                    ? 'bg-sky-600 text-white'
                    : 'bg-gray-100 text-gray-700 hover:bg-gray-200'
                }`}
                data-testid="mode-manual-btn"
              >
                Enter Manually
              </button>
            </div>

            {requestMode === 'provider' ? (
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">Select Provider *</label>
                <select
                  value={selectedProviderId}
                  onChange={(e) => setSelectedProviderId(e.target.value)}
                  className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-sky-500"
                  data-testid="provider-select"
                >
                  <option value="">-- Select a provider --</option>
                  {providers.map(p => (
                    <option key={p.id} value={p.id}>
                      {p.provider_name || p.facility} {p.npi ? `(NPI: ${p.npi})` : ''} {p.specialty ? `- ${p.specialty}` : ''}
                    </option>
                  ))}
                </select>
                {selectedProviderId && (() => {
                  const p = providers.find(pr => String(pr.id) === String(selectedProviderId));
                  if (!p) return null;
                  return (
                    <div className="mt-3 p-3 bg-sky-50 border border-sky-200 rounded-lg" data-testid="selected-provider-info">
                      <p className="text-sm font-medium text-sky-800">Provider: {p.provider_name || p.facility}</p>
                      {p.npi && <p className="text-sm text-sky-700">NPI: {p.npi}</p>}
                      {p.facility && <p className="text-sm text-sky-700">Facility: {p.facility}</p>}
                      {p.specialty && <p className="text-sm text-sky-700">Specialty: {p.specialty}</p>}
                      {p.npi_verified && (
                        <span className="inline-block mt-1 px-2 py-0.5 text-xs bg-emerald-100 text-emerald-700 rounded-full">
                          NPI Verified
                        </span>
                      )}
                    </div>
                  );
                })()}
              </div>
            ) : (
              <div className="space-y-4">
                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-1">Provider Name *</label>
                  <input
                    type="text"
                    value={manualProviderName}
                    onChange={(e) => setManualProviderName(e.target.value)}
                    placeholder="e.g., Dr. John Smith"
                    className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-sky-500"
                    data-testid="manual-provider-name"
                  />
                </div>
                <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                  <div>
                    <label className="block text-sm font-medium text-gray-700 mb-1">NPI Number</label>
                    <input
                      type="text"
                      value={manualProviderNpi}
                      onChange={(e) => setManualProviderNpi(e.target.value)}
                      placeholder="e.g., 1234567890"
                      className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-sky-500"
                      data-testid="manual-provider-npi"
                    />
                  </div>
                  <div>
                    <label className="block text-sm font-medium text-gray-700 mb-1">Provider Address</label>
                    <input
                      type="text"
                      value={manualProviderAddress}
                      onChange={(e) => setManualProviderAddress(e.target.value)}
                      placeholder="e.g., 123 Medical Dr, Suite 100"
                      className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-sky-500"
                      data-testid="manual-provider-address"
                    />
                  </div>
                </div>
              </div>
            )}

            {/* Request success/error messages */}
            {requestSuccess && (
              <div className="flex items-center gap-2 p-3 bg-emerald-50 border border-emerald-200 rounded-lg" data-testid="request-success">
                <svg className="h-5 w-5 text-emerald-500 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 13l4 4L19 7" />
                </svg>
                <p className="text-sm text-emerald-700 font-medium">{requestSuccess}</p>
              </div>
            )}
            {requestError && (
              <div className="flex items-center gap-2 p-3 bg-red-50 border border-red-200 rounded-lg" data-testid="request-error" role="alert">
                <svg className="h-5 w-5 text-red-500 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                </svg>
                <p className="text-sm text-red-700">{requestError}</p>
              </div>
            )}

            {/* Submit Request Button */}
            <button
              type="submit"
              disabled={requestSubmitting}
              className={`w-full py-3 rounded-lg text-white font-semibold transition ${
                requestSubmitting
                  ? 'bg-gray-400 cursor-not-allowed'
                  : 'bg-sky-600 hover:bg-sky-700'
              }`}
              data-testid="submit-request-btn"
            >
              {requestSubmitting ? 'Submitting Request...' : 'Submit Records Request'}
            </button>
          </form>

        </div>

        {/* Provider Access Grants — Feature #156: Active provider access grants */}
        <div
          className="bg-white rounded-xl shadow-sm border border-gray-200 p-6 mb-8"
          data-testid="provider-access-grants"
          id="provider-access-grants"
        >
          <div className="flex items-center justify-between mb-4">
            <div>
              <h2 className="text-lg font-semibold text-gray-900">🔑 Provider Access Grants</h2>
              <p className="text-sm text-gray-500 mt-0.5">
                Active provider access to your health records
              </p>
            </div>
            <button
              onClick={() => navigate('/provider-access')}
              className="text-sm text-sky-700 hover:text-sky-800 font-medium"
              data-testid="manage-access-link"
            >
              Manage Access →
            </button>
          </div>
          {loadingGrants ? (
            <div className="text-sm text-gray-400">Loading access grants...</div>
          ) : providerGrants.length === 0 ? (
            <div className="text-center py-4 text-gray-400" data-testid="no-grants-msg">
              <p className="text-sm">No active provider access grants.</p>
              <button
                onClick={() => navigate('/provider-access')}
                className="mt-2 text-sm text-sky-700 hover:text-sky-800 font-medium"
                data-testid="grant-access-link"
              >
                Grant provider access →
              </button>
            </div>
          ) : (
            <div className="space-y-3" data-testid="provider-grants-list">
              {providerGrants.map(grant => (
                <div key={grant.id} className="flex items-start justify-between p-3 bg-emerald-50 border border-emerald-200 rounded-lg" data-testid={`grant-${grant.id}`}>
                  <div>
                    <p className="text-sm font-medium text-gray-800" data-testid={`grant-provider-${grant.id}`}>{grant.provider_name || 'Provider'}</p>
                    <p className="text-xs text-gray-500 mt-0.5">
                      Scope: <span className="font-medium">{grant.scope || 'health records'}</span>
                    </p>
                    {grant.expires_at && (
                      <p className="text-xs text-gray-400 mt-0.5">
                        Expires: {new Date(grant.expires_at).toLocaleDateString()}
                      </p>
                    )}
                  </div>
                  <span className="px-2 py-0.5 text-xs bg-emerald-100 text-emerald-700 rounded-full font-medium" data-testid={`grant-status-${grant.id}`}>
                    Active
                  </span>
                </div>
              ))}
            </div>
          )}
        </div>

        {/* Request Tracker — Feature #89: Dedicated section showing status per provider */}
        <div
          className="bg-white rounded-xl shadow-sm border border-gray-200 p-6 mb-8"
          data-testid="request-tracker"
          id="request-tracker"
        >
          <div className="flex items-center justify-between mb-4">
            <div>
              <h2 className="text-lg font-semibold text-gray-900">📋 Request Tracker</h2>
              <p className="text-sm text-gray-500 mt-0.5">
                Track the status of your HIPAA records requests per provider
              </p>
            </div>
            {!loadingRequests && (
              <span className="text-sm text-gray-500" data-testid="request-tracker-count">
                {recordRequests.length} request{recordRequests.length !== 1 ? 's' : ''}
              </span>
            )}
          </div>

          {loadingRequests ? (
            <div className="flex items-center gap-2 py-4 text-gray-500 text-sm">
              <div className="animate-spin rounded-full h-4 w-4 border-b-2 border-sky-500"></div>
              Loading requests...
            </div>
          ) : recordRequests.length === 0 ? (
            <div className="text-center py-6 text-gray-400" data-testid="no-requests-msg">
              <p className="text-sm">No records requests yet.</p>
              <p className="text-xs mt-1">Submit a request above to get started.</p>
            </div>
          ) : (
            <div className="space-y-3" data-testid="request-tracker-list">
              {recordRequests.map((reqItem) => (
                <div
                  key={reqItem.id}
                  className="border border-gray-200 rounded-xl p-4 hover:border-sky-300 transition"
                  data-testid={`tracker-request-${reqItem.id}`}
                >
                  {/* Row 1: Provider name + current status badge */}
                  <div className="flex items-start justify-between gap-2 mb-2">
                    <div className="flex-1 min-w-0">
                      <p
                        className="font-semibold text-gray-900 text-sm truncate"
                        data-testid={`provider-name-${reqItem.id}`}
                      >
                        {getStatusIcon(reqItem.status)} {reqItem.provider_name}
                      </p>
                      <div className="flex flex-wrap gap-3 mt-1">
                        {reqItem.provider_npi && (
                          <span className="text-xs text-gray-500">NPI: {reqItem.provider_npi}</span>
                        )}
                        {reqItem.provider_address && (
                          <span className="text-xs text-gray-500 truncate max-w-xs">{reqItem.provider_address}</span>
                        )}
                      </div>
                    </div>
                    <span
                      className={`flex-shrink-0 px-3 py-1 text-xs rounded-full font-semibold ${getStatusBadge(reqItem.status)}`}
                      data-testid={`request-status-${reqItem.id}`}
                    >
                      {reqItem.status}
                    </span>
                  </div>

                  {/* Row 2: Dates — all status transitions with timestamps */}
                  <div className="flex flex-wrap gap-4 text-xs text-gray-500 mb-3">
                    <span data-testid={`sent-date-${reqItem.id}`}>
                      📤 Sent: {new Date(reqItem.sent_at).toLocaleDateString()}
                    </span>
                    {reqItem.pending_at && (
                      <span data-testid={`pending-date-${reqItem.id}`}>
                        ⏳ Pending: {new Date(reqItem.pending_at).toLocaleDateString()}
                      </span>
                    )}
                    {reqItem.received_at && (
                      <span data-testid={`received-date-${reqItem.id}`}>
                        ✅ Received: {new Date(reqItem.received_at).toLocaleDateString()}
                      </span>
                    )}
                  </div>

                  {/* Row 3: Update Status + Download actions */}
                  <div className="flex flex-wrap items-center gap-2">
                    <span className="text-xs text-gray-500 font-medium">Update status:</span>
                    {['sent', 'pending', 'received', 'failed'].map((s) => (
                      <button
                        key={s}
                        onClick={() => handleUpdateRequestStatus(reqItem.id, s)}
                        disabled={reqItem.status === s || updatingStatus[reqItem.id] === 'loading'}
                        className={`px-2 py-1 text-xs rounded-lg transition font-medium ${
                          reqItem.status === s
                            ? getStatusBadge(s) + ' cursor-default opacity-50'
                            : 'bg-gray-100 text-gray-600 hover:bg-gray-200 cursor-pointer'
                        }`}
                        data-testid={`status-btn-${reqItem.id}-${s}`}
                      >
                        {s}
                      </button>
                    ))}
                    {reqItem.letter_ready && (
                      <a
                        href={`/api/records/request/${reqItem.id}/letter`}
                        onClick={(e) => {
                          e.preventDefault();
                          const token = localStorage.getItem('livesafe_token');
                          fetch(`/api/records/request/${reqItem.id}/letter`, {
                            headers: { 'Authorization': `Bearer ${token}` }
                          })
                            .then(r => r.blob())
                            .then(blob => {
                              const url = window.URL.createObjectURL(blob);
                              const a = document.createElement('a');
                              a.href = url;
                              a.download = `HIPAA_Request_${reqItem.provider_name.replace(/[^a-zA-Z0-9]/g, '_')}_${reqItem.id}.pdf`;
                              document.body.appendChild(a);
                              a.click();
                              window.URL.revokeObjectURL(url);
                              document.body.removeChild(a);
                            });
                        }}
                        className="ml-auto px-2 py-1 text-xs bg-sky-100 text-sky-700 rounded hover:bg-sky-200 transition font-medium cursor-pointer"
                        data-testid={`download-letter-${reqItem.id}`}
                      >
                        📄 Download Letter
                      </a>
                    )}
                  </div>

                  {/* Status update feedback */}
                  {statusUpdateMsg[reqItem.id] && (
                    <div
                      className={`mt-2 text-xs font-medium px-2 py-1 rounded ${
                        updatingStatus[reqItem.id] === 'error'
                          ? 'text-red-600 bg-red-50'
                          : 'text-emerald-600 bg-emerald-50'
                      }`}
                      data-testid={`status-update-msg-${reqItem.id}`}
                    >
                      {statusUpdateMsg[reqItem.id]}
                    </div>
                  )}
                </div>
              ))}
            </div>
          )}
        </div>

        {/* Filter Bar */}
        <div className="bg-white rounded-xl shadow-sm border border-gray-200 p-4 mb-4" data-testid="filter-bar">
          <div className="flex flex-col sm:flex-row gap-3 items-start sm:items-center flex-wrap">
            {/* Search */}
            <div className="flex-1 min-w-0" style={{minWidth: '200px'}}>
              <input
                type="text"
                value={searchText}
                onChange={(e) => updateSearchText(e.target.value)}
                placeholder="Search records by title, type, or category..."
                className="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-sky-500"
                data-testid="filter-search"
                maxLength={SEARCH_MAX_LENGTH}
              />
              {searchText.length >= SEARCH_MAX_LENGTH - 50 && searchText.length > 0 && (
                <p className={`text-xs mt-1 ${searchText.length >= SEARCH_MAX_LENGTH ? 'text-red-500' : 'text-amber-500'}`} data-testid="search-length-warning">
                  {searchText.length >= SEARCH_MAX_LENGTH
                    ? `Search truncated to ${SEARCH_MAX_LENGTH} characters maximum`
                    : `${SEARCH_MAX_LENGTH - searchText.length} characters remaining`}
                </p>
              )}
              {urlParamSearchActive && searchText === urlSearchParam && (
                <p className="text-xs mt-1 text-blue-500" data-testid="url-param-search-indicator">
                  🔍 Search from URL parameter (sanitized)
                </p>
              )}
            </div>
            {/* Type Filter */}
            <select
              value={typeFilter}
              onChange={(e) => updateTypeFilter(e.target.value)}
              className="px-3 py-2 border border-gray-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-sky-500"
              data-testid="filter-type"
            >
              <option value="all">All Types</option>
              <option value="general">General</option>
              <option value="lab_result">Lab Result</option>
              <option value="imaging">Imaging / X-Ray</option>
              <option value="clinical_note">Clinical Note</option>
              <option value="prescription">Prescription</option>
              <option value="discharge_summary">Discharge Summary</option>
              <option value="vaccination">Vaccination</option>
              <option value="allergy">Allergy Record</option>
              <option value="surgical">Surgical Report</option>
            </select>
            {/* Category Filter */}
            <select
              value={categoryFilter}
              onChange={(e) => updateCategoryFilter(e.target.value)}
              className="px-3 py-2 border border-gray-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-sky-500"
              data-testid="filter-category"
            >
              <option value="all">All Categories</option>
              <option value="uncategorized">Uncategorized</option>
              <option value="emergency">Emergency</option>
              <option value="primary_care">Primary Care</option>
              <option value="specialist">Specialist</option>
              <option value="dental">Dental</option>
              <option value="mental_health">Mental Health</option>
              <option value="pharmacy">Pharmacy</option>
            </select>
            {/* Clear Filters */}
            {hasActiveFilters && (
              <button
                onClick={clearFilters}
                className="px-3 py-2 text-sm text-red-600 hover:text-red-800 font-medium border border-red-200 rounded-lg hover:bg-red-50 transition whitespace-nowrap"
                data-testid="clear-filters-btn"
              >
                Clear Filters
              </button>
            )}
          </div>
          {/* Date Range Filter Row */}
          <div className="flex flex-col sm:flex-row gap-3 items-start sm:items-center mt-3">
            <span className="text-xs font-medium text-gray-500 whitespace-nowrap">Date range:</span>
            <div className="flex items-center gap-2">
              <label className="text-xs text-gray-500 whitespace-nowrap" htmlFor="filter-date-from">From</label>
              <input
                id="filter-date-from"
                type="date"
                value={dateFrom}
                onChange={(e) => updateDateFrom(e.target.value)}
                className="px-3 py-2 border border-gray-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-sky-500"
                data-testid="filter-date-from"
              />
            </div>
            <div className="flex items-center gap-2">
              <label className="text-xs text-gray-500 whitespace-nowrap" htmlFor="filter-date-to">To</label>
              <input
                id="filter-date-to"
                type="date"
                value={dateTo}
                onChange={(e) => updateDateTo(e.target.value)}
                max={new Date().toISOString().split('T')[0]}
                className="px-3 py-2 border border-gray-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-sky-500"
                data-testid="filter-date-to"
              />
            </div>
            {(dateFrom || dateTo) && (
              <span className="text-xs text-sky-700" data-testid="date-filter-active">
                {dateFrom && dateTo
                  ? `${dateFrom} → ${dateTo}`
                  : dateFrom
                  ? `From ${dateFrom}`
                  : `Until ${dateTo}`}
              </span>
            )}
          </div>
          {/* Feature #367: Date Sort Control */}
          <div className="flex flex-col sm:flex-row gap-3 items-start sm:items-center mt-3">
            <span className="text-xs font-medium text-gray-500 whitespace-nowrap">Sort by date:</span>
            <div className="flex items-center gap-2">
              <button
                onClick={() => setSortBy('date_desc')}
                className={`px-3 py-1.5 text-xs font-medium rounded-lg border transition ${sortBy === 'date_desc' ? 'bg-sky-600 text-white border-sky-600' : 'bg-white text-gray-600 border-gray-300 hover:bg-gray-50'}`}
                data-testid="sort-date-desc"
                aria-pressed={sortBy === 'date_desc'}
              >
                Newest First ↓
              </button>
              <button
                onClick={() => setSortBy('date_asc')}
                className={`px-3 py-1.5 text-xs font-medium rounded-lg border transition ${sortBy === 'date_asc' ? 'bg-sky-600 text-white border-sky-600' : 'bg-white text-gray-600 border-gray-300 hover:bg-gray-50'}`}
                data-testid="sort-date-asc"
                aria-pressed={sortBy === 'date_asc'}
              >
                Oldest First ↑
              </button>
            </div>
            <span className="text-xs text-gray-500" data-testid="sort-status">
              {sortBy === 'date_desc' ? 'Showing newest records first' : 'Showing oldest records first'}
            </span>
          </div>
          {/* Restored filters notice — Feature #281: Filter state obvious after section navigation */}
          {filtersRestored && hasActiveFilters && (
            <div className="mt-2 flex items-center justify-between gap-2 p-2 bg-sky-50 border border-sky-200 rounded-lg text-sm text-sky-700" data-testid="filters-restored-notice">
              <div className="flex items-center gap-2">
                <svg className="h-4 w-4 text-sky-500 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                </svg>
                <span>Search/filter settings from your previous visit are active.</span>
              </div>
              <button
                type="button"
                onClick={clearFilters}
                className="flex-shrink-0 text-xs text-sky-700 hover:text-sky-800 font-medium underline"
                data-testid="clear-restored-filters-btn"
              >
                Clear all
              </button>
            </div>
          )}
        </div>

        {/* Record Deletion Audit Trail Toast (Feature #97) */}
        {deleteMessage && (
          <div className="mb-4 p-3 bg-amber-50 border border-amber-200 text-amber-800 rounded-lg text-sm flex items-start gap-2" data-testid="delete-audit-toast" role="status">
            <span>🗑️ {deleteMessage}</span>
          </div>
        )}

        {/* Records List */}
        <div className="bg-white rounded-xl shadow-sm border border-gray-200 p-6">
          <div className="flex items-center justify-between mb-4">
            <h2 className="text-lg font-semibold text-gray-900">
              {hasActiveFilters
                ? `Showing ${filteredRecords.length} of ${records.length} records`
                : `Your Records (${records.length})`}
            </h2>
            {hasActiveFilters && (
              <span className="text-xs text-sky-700 font-medium bg-sky-50 px-2 py-1 rounded-full" data-testid="filter-active-badge">
                Filters active
              </span>
            )}
          </div>

          {loadingRecords ? (
            <div className="text-center py-8">
              <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-sky-500 mx-auto"></div>
              <p className="text-gray-500 text-sm mt-2">Loading records...</p>
            </div>
          ) : records.length === 0 ? (
            <div className="text-center py-8">
              <p className="text-gray-500">No records uploaded yet. Use the form above to add your first record.</p>
            </div>
          ) : filteredRecords.length === 0 ? (
            <div className="text-center py-8" data-testid="no-filter-results">
              <p className="text-gray-500">No records match your current filters.</p>
              <button
                onClick={clearFilters}
                className="mt-2 text-sm text-sky-700 hover:text-sky-800 font-medium"
              >
                Clear all filters
              </button>
            </div>
          ) : (
            <div className="space-y-3">
              {paginatedRecords.map((record) => (
                <div
                  key={record.id}
                  className="p-4 border border-gray-200 rounded-lg hover:bg-gray-50"
                  data-testid={`record-${record.id}`}
                  data-created-at={record.created_at || ''}
                >
                  <div className="flex flex-col sm:flex-row items-start justify-between gap-2">
                    <div className="flex-1 min-w-0 w-full sm:w-auto">
                      {/* Title + badges row */}
                      <div className="flex items-center gap-2 flex-wrap">
                        <Link
                          to={`/health-vault/${record.id}`}
                          className="font-medium text-sky-700 hover:text-sky-900 hover:underline"
                          data-testid={`record-link-${record.id}`}
                        >{record.title}</Link>
                        {/* Feature #92: Encrypted badge */}
                        {record.encrypted && (
                          <span
                            className="px-2 py-0.5 text-xs bg-emerald-100 text-emerald-700 rounded-full font-medium"
                            data-testid={`encrypted-badge-${record.id}`}
                            title="This record is encrypted with AES-256-GCM using your subscriber key"
                          >
                            🔒 Encrypted
                          </span>
                        )}
                        {record.extracted_data?.format === 'C-CDA' && (
                          <span className="px-2 py-0.5 text-xs bg-sky-100 text-sky-700 rounded-full font-medium" data-testid={`ccda-badge-${record.id}`}>
                            C-CDA
                          </span>
                        )}
                        {record.extracted_data?.format === 'FHIR R4' && (
                          <span className="px-2 py-0.5 text-xs bg-indigo-100 text-indigo-700 rounded-full font-medium" data-testid={`fhir-badge-${record.id}`}>
                            FHIR R4
                          </span>
                        )}
                        {record.extracted_data?.parse_error && (
                          <span className="px-2 py-0.5 text-xs bg-amber-100 text-amber-700 rounded-full font-medium" data-testid={`parse-error-badge-${record.id}`} title={`File format error: ${record.extracted_data.parse_error}`}>
                            ⚠ Format Error
                          </span>
                        )}
                        {/* Feature #95: Visibility badge */}
                        {record.visibility && record.visibility !== 'all_providers' && (
                          <span
                            className="px-2 py-0.5 text-xs bg-violet-100 text-violet-700 rounded-full font-medium"
                            data-testid={`visibility-badge-${record.id}`}
                          >
                            {getVisibilityLabel(record.visibility)}
                          </span>
                        )}
                      </div>

                      {/* Metadata row */}
                      <div className="flex gap-3 mt-1 flex-wrap items-center">
                        <span className="text-xs text-gray-500 capitalize">{record.record_type || 'general'}</span>
                        <span className="text-xs text-gray-400">|</span>
                        <span className="text-xs text-gray-500 capitalize">{record.category || 'uncategorized'}</span>
                        <span className="text-xs text-gray-400">|</span>
                        <span className="text-xs text-gray-500">{new Date(record.created_at).toLocaleDateString()}</span>
                        {record.file_size && (
                          <>
                            <span className="text-xs text-gray-400">|</span>
                            <span className="text-xs text-gray-500">{formatFileSize(record.file_size)}</span>
                          </>
                        )}
                      </div>

                      {/* Feature #94: Show existing annotation */}
                      {record.annotation && editingRecordId !== record.id && (
                        <div className="mt-2 p-2 bg-amber-50 border border-amber-100 rounded text-xs text-amber-800" data-testid={`annotation-display-${record.id}`}>
                          <span className="font-medium">📝 Note: </span>{record.annotation}
                        </div>
                      )}

                      {/* Show saved feedback messages */}
                      {annotationSaveMsg[record.id] && (
                        <div className="mt-1 text-xs text-emerald-600 font-medium" data-testid={`annotation-saved-${record.id}`}>{annotationSaveMsg[record.id]}</div>
                      )}
                      {visibilitySaveMsg[record.id] && (
                        <div className="mt-1 text-xs text-emerald-600 font-medium" data-testid={`visibility-saved-${record.id}`}>{visibilitySaveMsg[record.id]}</div>
                      )}

                      {/* Show extracted C-CDA data */}
                      {record.extracted_data?.format === 'C-CDA' && (
                        <div className="mt-2 p-2 bg-sky-50 rounded-lg border border-sky-100 text-xs" data-testid={`ccda-extracted-${record.id}`}>
                          <p className="font-semibold text-sky-800 mb-1">Extracted from C-CDA:</p>
                          {record.extracted_data.patient?.name && <p className="text-sky-700">Patient: <strong>{record.extracted_data.patient.name}</strong></p>}
                          {record.extracted_data.patient?.dob && <p className="text-sky-700">DOB: {record.extracted_data.patient.dob}</p>}
                          {record.extracted_data.patient?.gender && <p className="text-sky-700">Gender: {record.extracted_data.patient.gender}</p>}
                          {record.extracted_data.document_date && <p className="text-sky-700">Document Date: {record.extracted_data.document_date}</p>}
                          <div className="flex flex-wrap gap-2 mt-1">
                            {record.extracted_data.summary?.allergies_count > 0 && <span className="text-emerald-700 font-medium">✓ {record.extracted_data.summary.allergies_count} allerg{record.extracted_data.summary.allergies_count === 1 ? 'y' : 'ies'}</span>}
                            {record.extracted_data.summary?.medications_count > 0 && <span className="text-emerald-700 font-medium">✓ {record.extracted_data.summary.medications_count} medication{record.extracted_data.summary.medications_count !== 1 ? 's' : ''}</span>}
                            {record.extracted_data.summary?.problems_count > 0 && <span className="text-emerald-700 font-medium">✓ {record.extracted_data.summary.problems_count} condition{record.extracted_data.summary.problems_count !== 1 ? 's' : ''}</span>}
                            {record.extracted_data.summary?.results_count > 0 && <span className="text-emerald-700 font-medium">✓ {record.extracted_data.summary.results_count} lab result{record.extracted_data.summary.results_count !== 1 ? 's' : ''}</span>}
                          </div>
                          {record.extracted_data.allergies?.length > 0 && <p className="text-sky-700 mt-1"><strong>Allergies:</strong> {record.extracted_data.allergies.map(a => a.substance).join(', ')}</p>}
                          {record.extracted_data.medications?.length > 0 && <p className="text-sky-700 mt-1"><strong>Medications:</strong> {record.extracted_data.medications.map(m => m.name + (m.dose ? ` ${m.dose}` : '')).join(', ')}</p>}
                          {record.extracted_data.problems?.length > 0 && <p className="text-sky-700 mt-1"><strong>Conditions:</strong> {record.extracted_data.problems.map(p => p.condition).join(', ')}</p>}
                        </div>
                      )}

                      {/* Show extracted FHIR R4 data */}
                      {record.extracted_data?.format === 'FHIR R4' && (
                        <div className="mt-2 p-2 bg-indigo-50 rounded-lg border border-indigo-100 text-xs" data-testid={`fhir-extracted-${record.id}`}>
                          <p className="font-semibold text-indigo-800 mb-1">Extracted from FHIR R4{record.extracted_data.resource_type ? ` (${record.extracted_data.resource_type})` : ''}:</p>
                          {record.extracted_data.patient?.name && <p className="text-indigo-700">Patient: <strong>{record.extracted_data.patient.name}</strong></p>}
                          {record.extracted_data.patient?.dob && <p className="text-indigo-700">DOB: {record.extracted_data.patient.dob}</p>}
                          {record.extracted_data.patient?.gender && <p className="text-indigo-700">Gender: {record.extracted_data.patient.gender}</p>}
                          <div className="flex flex-wrap gap-2 mt-1">
                            {record.extracted_data.summary?.allergies_count > 0 && <span className="text-emerald-700 font-medium">✓ {record.extracted_data.summary.allergies_count} allerg{record.extracted_data.summary.allergies_count === 1 ? 'y' : 'ies'}</span>}
                            {record.extracted_data.summary?.medications_count > 0 && <span className="text-emerald-700 font-medium">✓ {record.extracted_data.summary.medications_count} medication{record.extracted_data.summary.medications_count !== 1 ? 's' : ''}</span>}
                            {record.extracted_data.summary?.problems_count > 0 && <span className="text-emerald-700 font-medium">✓ {record.extracted_data.summary.problems_count} condition{record.extracted_data.summary.problems_count !== 1 ? 's' : ''}</span>}
                            {record.extracted_data.summary?.results_count > 0 && <span className="text-emerald-700 font-medium">✓ {record.extracted_data.summary.results_count} lab result{record.extracted_data.summary.results_count !== 1 ? 's' : ''}</span>}
                            {record.extracted_data.summary?.immunizations_count > 0 && <span className="text-emerald-700 font-medium">✓ {record.extracted_data.summary.immunizations_count} immunization{record.extracted_data.summary.immunizations_count !== 1 ? 's' : ''}</span>}
                          </div>
                          {record.extracted_data.allergies?.length > 0 && <p className="text-indigo-700 mt-1"><strong>Allergies:</strong> {record.extracted_data.allergies.map(a => a.substance).join(', ')}</p>}
                          {record.extracted_data.medications?.length > 0 && <p className="text-indigo-700 mt-1"><strong>Medications:</strong> {record.extracted_data.medications.map(m => m.name + (m.dose ? ` ${m.dose}` : '')).join(', ')}</p>}
                          {record.extracted_data.problems?.length > 0 && <p className="text-indigo-700 mt-1"><strong>Conditions:</strong> {record.extracted_data.problems.map(p => p.condition).join(', ')}</p>}
                          {record.extracted_data.results?.length > 0 && <p className="text-indigo-700 mt-1"><strong>Lab Results:</strong> {record.extracted_data.results.map(r => r.test + (r.value ? `: ${r.value}` : '')).join(', ')}</p>}
                          {record.extracted_data.immunizations?.length > 0 && <p className="text-indigo-700 mt-1"><strong>Immunizations:</strong> {record.extracted_data.immunizations.map(i => i.vaccine).join(', ')}</p>}
                        </div>
                      )}

                      {/* Show parse error warning for malformed files (#385) */}
                      {record.extracted_data?.parse_error && (
                        <div className="mt-2 p-2 bg-amber-50 rounded-lg border border-amber-200 text-xs" data-testid={`parse-error-warning-${record.id}`} role="alert">
                          <p className="font-semibold text-amber-800 mb-1">⚠ File Format Error</p>
                          <p className="text-amber-700">This file could not be fully parsed. It may be malformed or in an unsupported format.</p>
                          <p className="text-amber-600 mt-1 font-mono break-all">{record.extracted_data.parse_error}</p>
                        </div>
                      )}

                      {/* Feature #94: Inline annotation + category editor */}
                      {editingRecordId === record.id && (
                        <div className="mt-3 p-3 bg-amber-50 border border-amber-200 rounded-lg" data-testid={`annotation-editor-${record.id}`}>
                          <p className="text-sm font-semibold text-gray-700 mb-2">Edit Notes &amp; Category</p>
                          <div className="mb-2">
                            <label className="block text-xs font-medium text-gray-600 mb-1">Category</label>
                            <select
                              value={editCategory}
                              onChange={(e) => setEditCategory(e.target.value)}
                              className="w-full text-sm border border-gray-300 rounded px-2 py-1.5 focus:outline-none focus:ring-2 focus:ring-sky-400"
                              data-testid={`edit-category-select-${record.id}`}
                            >
                              <option value="uncategorized">Uncategorized</option>
                              <option value="emergency">Emergency</option>
                              <option value="primary_care">Primary Care</option>
                              <option value="specialist">Specialist</option>
                              <option value="dental">Dental</option>
                              <option value="mental_health">Mental Health</option>
                              <option value="pharmacy">Pharmacy</option>
                            </select>
                          </div>
                          <div className="mb-2">
                            <label className="block text-xs font-medium text-gray-600 mb-1">Notes / Annotation</label>
                            <textarea
                              value={editAnnotation}
                              onChange={(e) => setEditAnnotation(e.target.value)}
                              placeholder="Add a personal note about this record..."
                              rows={3}
                              className="w-full text-sm border border-gray-300 rounded px-2 py-1.5 focus:outline-none focus:ring-2 focus:ring-sky-400 resize-none"
                              data-testid={`annotation-textarea-${record.id}`}
                            />
                          </div>
                          <div className="flex gap-2">
                            <button
                              onClick={() => handleAnnotationSave(record.id)}
                              disabled={savingAnnotation}
                              className="px-3 py-1.5 text-xs bg-emerald-600 text-white rounded hover:bg-emerald-700 transition disabled:opacity-50"
                              data-testid={`save-annotation-btn-${record.id}`}
                            >
                              {savingAnnotation ? 'Saving…' : 'Save'}
                            </button>
                            <button
                              onClick={() => setEditingRecordId(null)}
                              className="px-3 py-1.5 text-xs bg-gray-100 text-gray-700 rounded hover:bg-gray-200 transition"
                              data-testid={`cancel-annotation-btn-${record.id}`}
                            >
                              Cancel
                            </button>
                          </div>
                        </div>
                      )}

                      {/* Feature #95: Inline visibility settings editor */}
                      {visibilityRecordId === record.id && (
                        <div className="mt-3 p-3 bg-violet-50 border border-violet-200 rounded-lg" data-testid={`visibility-editor-${record.id}`}>
                          <p className="text-sm font-semibold text-gray-700 mb-2">Record Visibility Settings</p>
                          <div className="mb-2">
                            <label className="block text-xs font-medium text-gray-600 mb-1">Who can access this record?</label>
                            <select
                              value={editVisibility}
                              onChange={(e) => {
                                setEditVisibility(e.target.value);
                                if (e.target.value !== 'specific_providers') setEditVisibilityProviders([]);
                              }}
                              className="w-full text-sm border border-gray-300 rounded px-2 py-1.5 focus:outline-none focus:ring-2 focus:ring-violet-400"
                              data-testid={`visibility-select-${record.id}`}
                            >
                              <option value="all_providers">👁️ All Providers</option>
                              <option value="specific_providers">👥 Specific Providers Only</option>
                              <option value="emergency_only">🚨 Emergency Access Only</option>
                              <option value="private">🔒 Private (Only Me)</option>
                            </select>
                          </div>
                          {/* If specific_providers, allow provider selection from available providers */}
                          {editVisibility === 'specific_providers' && (
                            <div className="mb-2">
                              <label className="block text-xs font-medium text-gray-600 mb-1">Select Providers</label>
                              {providers.length > 0 ? (
                                <div className="space-y-1 max-h-32 overflow-y-auto border border-gray-200 rounded p-2 bg-white">
                                  {providers.map((p) => (
                                    <label key={p.id} className="flex items-center gap-2 text-xs cursor-pointer hover:bg-gray-50 p-1 rounded">
                                      <input
                                        type="checkbox"
                                        checked={editVisibilityProviders.includes(String(p.id))}
                                        onChange={(e) => {
                                          const pid = String(p.id);
                                          setEditVisibilityProviders(prev =>
                                            e.target.checked ? [...prev, pid] : prev.filter(x => x !== pid)
                                          );
                                        }}
                                        data-testid={`provider-checkbox-${p.id}`}
                                      />
                                      <span className="text-gray-700">{p.provider_name || p.facility}</span>
                                      {p.npi && <span className="text-gray-400">NPI: {p.npi}</span>}
                                    </label>
                                  ))}
                                </div>
                              ) : (
                                <div className="p-2 bg-amber-50 border border-amber-200 rounded text-xs text-amber-700">
                                  No verified providers available. You can still save — add provider IDs manually if needed.
                                </div>
                              )}
                            </div>
                          )}
                          <div className="text-xs text-gray-500 mb-2">
                            {editVisibility === 'all_providers' && 'Any verified provider in the system can access this record.'}
                            {editVisibility === 'specific_providers' && 'Only the selected providers can access this record.'}
                            {editVisibility === 'emergency_only' && 'This record is only visible during emergency scans.'}
                            {editVisibility === 'private' && 'Only you can view this record. No provider access.'}
                          </div>
                          <div className="flex gap-2">
                            <button
                              onClick={() => handleVisibilitySave(record.id)}
                              disabled={savingVisibility}
                              className="px-3 py-1.5 text-xs bg-violet-600 text-white rounded hover:bg-violet-700 transition disabled:opacity-50"
                              data-testid={`save-visibility-btn-${record.id}`}
                            >
                              {savingVisibility ? 'Saving…' : 'Save Visibility'}
                            </button>
                            <button
                              onClick={() => setVisibilityRecordId(null)}
                              className="px-3 py-1.5 text-xs bg-gray-100 text-gray-700 rounded hover:bg-gray-200 transition"
                              data-testid={`cancel-visibility-btn-${record.id}`}
                            >
                              Cancel
                            </button>
                          </div>
                        </div>
                      )}
                    </div>

                    {/* Action buttons */}
                    <div className="flex items-center gap-1 flex-shrink-0 flex-wrap sm:ml-2">
                      {/* Feature #94: Annotate/Category button */}
                      <button
                        onClick={() => editingRecordId === record.id ? setEditingRecordId(null) : openAnnotationEditor(record)}
                        className={`text-xs px-2 py-1 rounded transition ${
                          editingRecordId === record.id
                            ? 'bg-amber-200 text-amber-800'
                            : 'text-amber-600 hover:text-amber-800 border border-amber-200 hover:bg-amber-50'
                        }`}
                        title="Add or edit annotation and category"
                        data-testid={`annotate-btn-${record.id}`}
                      >
                        📝 {record.annotation ? 'Edit Note' : 'Add Note'}
                      </button>
                      {/* Feature #95: Visibility button */}
                      <button
                        onClick={() => visibilityRecordId === record.id ? setVisibilityRecordId(null) : openVisibilitySettings(record)}
                        className={`text-xs px-2 py-1 rounded transition ${
                          visibilityRecordId === record.id
                            ? 'bg-violet-200 text-violet-800'
                            : 'text-violet-600 hover:text-violet-800 border border-violet-200 hover:bg-violet-50'
                        }`}
                        title="Set visibility/sharing settings"
                        data-testid={`visibility-btn-${record.id}`}
                      >
                        🔍 Visibility
                      </button>
                      {/* Feature #96: Version History button */}
                      <button
                        onClick={() => fetchVersionHistory(record)}
                        className={`text-xs px-2 py-1 rounded transition ${
                          expandedVersions[record.id]
                            ? 'bg-sky-200 text-sky-800'
                            : 'text-sky-700 hover:text-sky-800 border border-sky-200 hover:bg-sky-50'
                        }`}
                        title="View version history"
                        data-testid={`version-history-btn-${record.id}`}
                      >
                        {loadingVersions[record.id] ? '⏳' : '🕐'} Versions
                      </button>
                      <button
                        onClick={() => handleDelete(record.id, record.title)}
                        className="text-xs text-red-500 hover:text-red-700 px-2 py-1 rounded transition border border-red-200 hover:bg-red-50"
                        data-testid={`delete-record-${record.id}`}
                      >
                        Delete
                      </button>
                    </div>
                  </div>

                  {/* Feature #96: Version History Panel */}
                  {expandedVersions[record.id] && (
                    <div className="mt-3 p-3 bg-sky-50 border border-sky-200 rounded-lg" data-testid={`version-history-panel-${record.id}`}>
                      <div className="flex items-center justify-between mb-2">
                        <p className="text-sm font-semibold text-sky-800">📋 Version History ({expandedVersions[record.id].total_versions} version{expandedVersions[record.id].total_versions !== 1 ? 's' : ''})</p>
                        <button
                          onClick={() => setUploadingVersionFor(uploadingVersionFor?.id === record.id ? null : record)}
                          className="text-xs px-2 py-1 bg-sky-600 text-white rounded hover:bg-sky-700 transition"
                          data-testid={`upload-new-version-btn-${record.id}`}
                        >
                          + Upload New Version
                        </button>
                      </div>
                      <div className="space-y-2">
                        {expandedVersions[record.id].versions.map((ver) => (
                          <div key={ver.id} className="flex items-center justify-between p-2 bg-white border border-sky-100 rounded text-xs" data-testid={`version-item-${ver.id}`}>
                            <div className="flex items-center gap-2">
                              <span className="font-bold text-sky-700">v{ver.version}</span>
                              <span className="text-gray-700 truncate max-w-xs">{ver.title}</span>
                            </div>
                            <div className="flex items-center gap-3 text-gray-500 flex-shrink-0">
                              <span data-testid={`version-timestamp-${ver.id}`}>{new Date(ver.created_at).toLocaleString()}</span>
                              {ver.file_size && <span>{formatFileSize(ver.file_size)}</span>}
                              {ver.version === 1 && <span className="px-1.5 py-0.5 bg-emerald-100 text-emerald-700 rounded-full font-medium">Original</span>}
                            </div>
                          </div>
                        ))}
                      </div>

                      {/* Upload New Version Form */}
                      {uploadingVersionFor?.id === record.id && (
                        <div className="mt-3 p-3 bg-white border border-sky-300 rounded-lg" data-testid={`version-upload-form-${record.id}`}>
                          <p className="text-sm font-semibold text-gray-700 mb-2">Upload New Version</p>
                          {versionUploadError && (
                            <div className="mb-2 p-2 bg-red-50 border border-red-200 rounded text-xs text-red-700" data-testid="version-upload-error">{versionUploadError}</div>
                          )}
                          <div className="mb-2">
                            <label className="block text-xs font-medium text-gray-600 mb-1">Version Title *</label>
                            <input
                              type="text"
                              value={versionTitle}
                              onChange={(e) => setVersionTitle(e.target.value)}
                              placeholder={`${record.title} v${(expandedVersions[record.id]?.total_versions || 1) + 1}`}
                              className="w-full text-sm border border-gray-300 rounded px-2 py-1.5 focus:outline-none focus:ring-2 focus:ring-sky-400"
                              data-testid="version-title-input"
                            />
                          </div>
                          <div className="mb-2">
                            <label className="block text-xs font-medium text-gray-600 mb-1">Select File *</label>
                            <input
                              type="file"
                              ref={versionFileRef}
                              onChange={(e) => setVersionFile(e.target.files[0])}
                              className="w-full text-sm"
                              accept=".pdf,.jpg,.jpeg,.png,.gif,.txt,.doc,.docx,.json,.xml"
                              data-testid="version-file-input"
                            />
                            {versionFile && <p className="text-xs text-gray-500 mt-1">{versionFile.name} ({formatFileSize(versionFile.size)})</p>}
                          </div>
                          {versionUploading && (
                            <div className="mb-2">
                              <div className="w-full bg-gray-200 rounded-full h-2">
                                <div className="h-2 bg-sky-500 rounded-full transition-all" style={{ width: `${versionUploadProgress}%` }}></div>
                              </div>
                              <p className="text-xs text-gray-500 mt-1">{versionUploadProgress}%</p>
                            </div>
                          )}
                          <div className="flex gap-2">
                            <button
                              onClick={handleVersionUpload}
                              disabled={versionUploading}
                              className="px-3 py-1.5 text-xs bg-sky-600 text-white rounded hover:bg-sky-700 transition disabled:opacity-50"
                              data-testid="version-upload-submit"
                            >
                              {versionUploading ? 'Uploading...' : 'Upload Version'}
                            </button>
                            <button
                              onClick={() => { setUploadingVersionFor(null); setVersionFile(null); setVersionTitle(''); setVersionUploadError(''); }}
                              className="px-3 py-1.5 text-xs bg-gray-100 text-gray-700 rounded hover:bg-gray-200 transition"
                              data-testid="version-upload-cancel"
                            >
                              Cancel
                            </button>
                          </div>
                        </div>
                      )}
                    </div>
                  )}
                </div>
              ))}
            </div>
          )}

          {/* Pagination controls (Feature #221) */}
          {totalPages > 1 && (
            <div className="mt-4 flex items-center justify-between border-t border-gray-200 pt-4" data-testid="pagination-controls">
              <div className="text-sm text-gray-600" data-testid="pagination-info">
                Page <span data-testid="current-page">{safePage}</span> of <span data-testid="total-pages">{totalPages}</span>
                {' '}·{' '}
                Showing {((safePage - 1) * RECORDS_PER_PAGE) + 1}–{Math.min(safePage * RECORDS_PER_PAGE, filteredRecords.length)} of {filteredRecords.length} records
              </div>
              <div className="flex items-center gap-2">
                <button
                  onClick={() => setCurrentPage(p => Math.max(1, p - 1))}
                  disabled={safePage <= 1}
                  className="px-3 py-1.5 text-sm border border-gray-300 rounded-lg hover:bg-gray-50 disabled:opacity-40 disabled:cursor-not-allowed transition"
                  data-testid="pagination-prev"
                  aria-label="Previous page"
                >
                  ← Prev
                </button>
                {Array.from({ length: totalPages }, (_, i) => i + 1).map(page => (
                  <button
                    key={page}
                    onClick={() => setCurrentPage(page)}
                    className={`px-3 py-1.5 text-sm border rounded-lg transition ${
                      page === safePage
                        ? 'bg-sky-600 text-white border-sky-600 font-medium'
                        : 'border-gray-300 hover:bg-gray-50 text-gray-700'
                    }`}
                    data-testid={`pagination-page-${page}`}
                    aria-label={`Page ${page}`}
                    aria-current={page === safePage ? 'page' : undefined}
                  >
                    {page}
                  </button>
                ))}
                <button
                  onClick={() => setCurrentPage(p => Math.min(totalPages, p + 1))}
                  disabled={safePage >= totalPages}
                  className="px-3 py-1.5 text-sm border border-gray-300 rounded-lg hover:bg-gray-50 disabled:opacity-40 disabled:cursor-not-allowed transition"
                  data-testid="pagination-next"
                  aria-label="Next page"
                >
                  Next →
                </button>
              </div>
            </div>
          )}
        </div>

        {/* Provider Clinical Notes - Subscriber Approval (Feature #107) */}
        <div className="bg-white rounded-xl shadow-sm border border-gray-200 p-6" data-testid="clinical-notes-section">
          <h2 className="text-xl font-semibold text-gray-900 mb-2">Provider Clinical Notes</h2>
          <p className="text-sm text-gray-500 mb-4">Review and approve clinical notes submitted by your providers. Notes require your approval before being added to your record.</p>
          {loadingClinicalNotes ? (
            <p className="text-gray-400 text-sm">Loading clinical notes...</p>
          ) : clinicalNotes.length === 0 ? (
            <p className="text-gray-500 text-sm py-3 text-center">No clinical notes from providers.</p>
          ) : (
            <div className="space-y-4">
              {clinicalNotes.map(note => (
                <div key={note.id} className={`p-4 rounded-lg border ${note.status === 'pending_approval' ? 'border-amber-300 bg-amber-50' : note.status === 'approved' ? 'border-emerald-300 bg-emerald-50' : 'border-gray-200 bg-gray-50'}`} data-testid={`clinical-note-${note.id}`}>
                  <div className="flex items-start justify-between gap-4">
                    <div className="flex-1">
                      <div className="flex items-center gap-2 mb-1">
                        <span className={`text-xs font-semibold px-2 py-0.5 rounded-full ${note.status === 'pending_approval' ? 'bg-amber-200 text-amber-800' : note.status === 'approved' ? 'bg-emerald-200 text-emerald-800' : 'bg-gray-200 text-gray-700'}`}>
                          {note.status === 'pending_approval' ? '⏳ Pending Approval' : note.status === 'approved' ? '✅ Approved' : '❌ Rejected'}
                        </span>
                        <span className="text-xs text-gray-500">{note.note_type}</span>
                      </div>
                      <p className="text-sm font-medium text-gray-800 mb-1">
                        From: Dr. {note.provider_display_name || [note.provider_first_name, note.provider_last_name].filter(Boolean).join(' ').trim() || 'Your provider'}
                      </p>
                      <p className="text-sm text-gray-700 whitespace-pre-wrap" data-testid={`note-text-${note.id}`}>{note.note_text}</p>
                      <p className="text-xs text-gray-400 mt-2">{new Date(note.created_at).toLocaleString()}</p>
                      {noteActionMsg[note.id] && <p className="text-sm text-emerald-700 mt-2 font-medium">{noteActionMsg[note.id]}</p>}
                      {noteActionError[note.id] && <p className="text-sm text-red-600 mt-2">{noteActionError[note.id]}</p>}
                    </div>
                    {note.status === 'pending_approval' && (
                      <div className="flex flex-col gap-2 flex-shrink-0">
                        <button
                          onClick={() => handleApproveNote(note.id)}
                          data-testid={`approve-note-${note.id}`}
                          className="px-4 py-1.5 text-sm bg-emerald-600 text-white rounded-lg hover:bg-emerald-700 transition font-medium"
                        >
                          ✅ Approve
                        </button>
                        <button
                          onClick={() => handleRejectNote(note.id)}
                          data-testid={`reject-note-${note.id}`}
                          className="px-4 py-1.5 text-sm bg-red-100 text-red-700 rounded-lg hover:bg-red-200 transition font-medium"
                        >
                          ❌ Reject
                        </button>
                      </div>
                    )}
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>

      </main>
    </div>
  );
}

export default Records;
