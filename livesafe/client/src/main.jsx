import React from 'react';
import ReactDOM from 'react-dom/client';
import { createBrowserRouter, RouterProvider, Outlet, Navigate, useLocation } from 'react-router-dom';

// Feature #405: Register Service Worker for PWA installability
if ('serviceWorker' in navigator) {
  window.addEventListener('load', () => {
    navigator.serviceWorker.register('/sw.js', { scope: '/' })
      .then((registration) => {
        console.log('[PWA] Service Worker registered successfully, scope:', registration.scope);
      })
      .catch((error) => {
        console.warn('[PWA] Service Worker registration failed:', error);
      });
  });
}
import { AuthProvider, useAuth } from './context/AuthContext';
import { NotificationsProvider } from './context/NotificationsContext';
import Landing from './pages/Landing';
import Register from './pages/Register';
import Login from './pages/Login';
import Dashboard from './pages/Dashboard';
import TrusteeAccept from './pages/TrusteeAccept';
import ProviderRegister from './pages/ProviderRegister';
import Records from './pages/Records';
import VerifyEmail from './pages/VerifyEmail';
import Profile from './pages/Profile';
import ProviderAccess from './pages/ProviderAccess';
import OdentityScore from './pages/OdentityScore';
import Card from './pages/Card';
import CredentialVault from './pages/CredentialVault';
import CredentialSettings from './pages/CredentialSettings';
import TrusteeLogin from './pages/TrusteeLogin';
import TrusteeDashboard from './pages/TrusteeDashboard';
import TrusteeSubscriberDetail from './pages/TrusteeSubscriberDetail';
import Notifications from './pages/Notifications';
import Research from './pages/Research';
import ProviderLogin from './pages/ProviderLogin';
import ProviderDashboard from './pages/ProviderDashboard';
import Pace from './pages/Pace';
import Settings from './pages/Settings';
import ScanHistory from './pages/ScanHistory';
import ScanDetail from './pages/ScanDetail';
import AlertHistory from './pages/AlertHistory';
import AuditTrail from './pages/AuditTrail';
import RecordDetail from './pages/RecordDetail';
import AdminDashboard from './pages/AdminDashboard';
import OnboardingWizard from './pages/OnboardingWizard';
import Marketplace from './pages/Marketplace';
import Library from './pages/Library';
import Footer from './components/Footer';
import './styles/index.css';

// Root layout - provides AuthContext and NotificationsContext to all routes
function RootLayout() {
  return (
    <AuthProvider>
      <NotificationsProvider>
        <div className="min-h-screen bg-white font-sans flex flex-col">
          {/* Feature #359: Skip to main content link for keyboard users */}
          <a
            href="#main-content"
            data-testid="skip-nav-link"
            className="sr-only focus:not-sr-only focus:absolute focus:top-2 focus:left-2 focus:z-50 focus:px-4 focus:py-2 focus:bg-sky-600 focus:text-white focus:rounded-lg focus:font-medium focus:shadow-lg focus:outline-none"
          >
            Skip to main content
          </a>
          <div className="flex-1">
            <Outlet />
          </div>
          <Footer />
        </div>
      </NotificationsProvider>
    </AuthProvider>
  );
}

// Protected route wrapper
// Feature #307: saves current location so login can redirect back after auth
function ProtectedRoute({ children }) {
  const { isAuthenticated, loading } = useAuth();
  const location = useLocation();
  if (loading) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-gray-50">
        <div className="text-center">
          <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-sky-500 mx-auto mb-4"></div>
          <p className="text-gray-600">Loading...</p>
        </div>
      </div>
    );
  }
  if (!isAuthenticated) {
    return <Navigate to="/login" state={{ from: location }} replace />;
  }
  return children;
}

// Redirect authenticated users away from auth pages
function PublicRoute({ children }) {
  const { isAuthenticated, loading } = useAuth();
  if (loading) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-gray-50">
        <div className="text-center">
          <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-sky-500 mx-auto mb-4"></div>
          <p className="text-gray-600">Loading...</p>
        </div>
      </div>
    );
  }
  if (isAuthenticated) {
    return <Navigate to="/dashboard" replace />;
  }
  return children;
}

// Provider route guard — requires a valid provider token (separate from subscriber auth)
function ProviderRoute({ children }) {
  const providerToken = localStorage.getItem('livesafe_provider_token');
  if (!providerToken) {
    return <Navigate to="/provider/login" replace />;
  }
  return children;
}

// Subscriber-only route guard — requires subscriber or subscriber_admin role
// Blocks providers, trustees, and other non-subscriber user types from accessing subscriber-only routes
function SubscriberRoute({ children }) {
  const { user, loading } = useAuth();
  if (loading) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-gray-50">
        <div className="text-center">
          <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-sky-500 mx-auto mb-4"></div>
          <p className="text-gray-600">Loading...</p>
        </div>
      </div>
    );
  }
  if (!user) {
    return <Navigate to="/login" replace />;
  }
  if (user.role !== 'subscriber' && user.role !== 'subscriber_admin') {
    return (
      <div className="min-h-screen flex items-center justify-center bg-gray-50" data-testid="pace-access-denied-page">
        <div className="text-center max-w-md">
          <div className="text-6xl mb-4">🚫</div>
          <h1 className="text-3xl font-bold text-gray-900 mb-2" data-testid="pace-access-denied-heading">Access Denied</h1>
          <p className="text-gray-500 mb-6" data-testid="pace-access-denied-message">
            PACE management is only available to subscribers. Please log in with a subscriber account.
          </p>
          <a
            href="/dashboard"
            className="inline-block bg-sky-500 text-white px-6 py-3 rounded-lg font-medium hover:bg-sky-600 transition-colors"
            data-testid="pace-access-denied-dashboard-link"
          >
            Go to Dashboard
          </a>
        </div>
      </div>
    );
  }
  return children;
}

// Admin route guard — requires subscriber_admin role
function AdminRoute({ children }) {
  const { user, loading } = useAuth();
  if (loading) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-gray-50">
        <div className="text-center">
          <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-sky-500 mx-auto mb-4"></div>
          <p className="text-gray-600">Loading...</p>
        </div>
      </div>
    );
  }
  if (!user) {
    return <Navigate to="/login" replace />;
  }
  if (user.role !== 'subscriber_admin') {
    return (
      <div className="min-h-screen flex items-center justify-center bg-gray-50" data-testid="access-denied-page">
        <div className="text-center max-w-md">
          <div className="text-6xl mb-4">🚫</div>
          <h1 className="text-3xl font-bold text-gray-900 mb-2" data-testid="access-denied-heading">Access Denied</h1>
          <p className="text-gray-500 mb-6" data-testid="access-denied-message">
            You do not have permission to access the admin panel. Admin privileges are required.
          </p>
          <a
            href="/dashboard"
            className="inline-block bg-sky-500 text-white px-6 py-3 rounded-lg font-medium hover:bg-sky-600 transition-colors"
            data-testid="access-denied-dashboard-link"
          >
            Go to Dashboard
          </a>
        </div>
      </div>
    );
  }
  return children;
}

const router = createBrowserRouter(
  [
    { path: '/', element: <Landing /> },
    {
      element: <RootLayout />,
      children: [
        { path: '/register', element: <PublicRoute><Register /></PublicRoute> },
        { path: '/login', element: <PublicRoute><Login /></PublicRoute> },
        { path: '/dashboard', element: <ProtectedRoute><Dashboard /></ProtectedRoute> },
        { path: '/dashboard/health-vault', element: <ProtectedRoute><Records /></ProtectedRoute> },
        { path: '/records', element: <ProtectedRoute><Records /></ProtectedRoute> },
        { path: '/health-vault', element: <ProtectedRoute><Records /></ProtectedRoute> },
        { path: '/health-vault/:recordId', element: <ProtectedRoute><RecordDetail /></ProtectedRoute> },
        { path: '/profile', element: <ProtectedRoute><Profile /></ProtectedRoute> },
        { path: '/provider-access', element: <ProtectedRoute><ProviderAccess /></ProtectedRoute> },
        { path: '/odentity', element: <ProtectedRoute><OdentityScore /></ProtectedRoute> },
        { path: '/card', element: <ProtectedRoute><Card /></ProtectedRoute> },
        { path: '/credentials', element: <ProtectedRoute><CredentialVault /></ProtectedRoute> },
        { path: '/credentials/settings', element: <ProtectedRoute><CredentialSettings /></ProtectedRoute> },
        { path: '/pace', element: <SubscriberRoute><Pace /></SubscriberRoute> },
        { path: '/settings', element: <ProtectedRoute><Settings /></ProtectedRoute> },
        { path: '/notifications', element: <ProtectedRoute><Notifications /></ProtectedRoute> },
        { path: '/research', element: <ProtectedRoute><Research /></ProtectedRoute> },
        { path: '/scan-history', element: <ProtectedRoute><ScanHistory /></ProtectedRoute> },
        { path: '/scan-history/:scanId', element: <ProtectedRoute><ScanDetail /></ProtectedRoute> },
        { path: '/alert-history', element: <ProtectedRoute><AlertHistory /></ProtectedRoute> },
        { path: '/audit-trail', element: <ProtectedRoute><AuditTrail /></ProtectedRoute> },
        { path: '/marketplace', element: <ProtectedRoute><Marketplace /></ProtectedRoute> },
        { path: '/library', element: <ProtectedRoute><Library /></ProtectedRoute> },
        { path: '/verify', element: <VerifyEmail /> },
        { path: '/trustee/accept', element: <TrusteeAccept /> },
        { path: '/invite', element: <TrusteeAccept /> },
        { path: '/trustee/login', element: <TrusteeLogin /> },
        { path: '/trustee/dashboard', element: <TrusteeDashboard /> },
        { path: '/trustee/subscriber/:subscriberDid', element: <TrusteeSubscriberDetail /> },
        { path: '/provider/register', element: <ProviderRegister /> },
        { path: '/provider/login', element: <ProviderLogin /> },
        { path: '/provider/dashboard', element: <ProviderRoute><ProviderDashboard /></ProviderRoute> },
        { path: '/admin', element: <Navigate to="/admin/dashboard" replace /> },
        { path: '/admin/dashboard', element: <AdminRoute><AdminDashboard /></AdminRoute> },
        { path: '/onboarding', element: <ProtectedRoute><OnboardingWizard /></ProtectedRoute> },
        { path: '*', element: <div className="min-h-screen flex items-center justify-center bg-gray-50" data-testid="not-found-page"><div className="text-center"><div className="text-6xl mb-4">🔍</div><h1 className="text-3xl font-bold text-gray-900 mb-2">404 - Page Not Found</h1><p className="text-gray-500 mb-6">The page you're looking for doesn't exist.</p><a href="/" className="inline-block bg-sky-500 text-white px-6 py-3 rounded-lg font-medium hover:bg-sky-600 transition-colors" data-testid="go-home-link">Go to Home</a></div></div> },
      ]
    }
  ],
  { future: { v7_startTransition: true, v7_relativeSplatPath: true } }
);

ReactDOM.createRoot(document.getElementById('root')).render(
  <React.StrictMode>
    <RouterProvider router={router} />
  </React.StrictMode>
);
