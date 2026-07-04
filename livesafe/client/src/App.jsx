import React from 'react';
import { Routes, Route, Navigate } from 'react-router-dom';
import { AuthProvider, useAuth } from './context/AuthContext';
import Home from './pages/Home';
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
import AlertHistory from './pages/AlertHistory';
import AuditTrail from './pages/AuditTrail';
import Marketplace from './pages/Marketplace';
import Library from './pages/Library';

// Protected route wrapper
function ProtectedRoute({ children }) {
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

  if (!isAuthenticated) {
    return <Navigate to="/login" replace />;
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

function App() {
  return (
    <AuthProvider>
      <div className="min-h-screen bg-white font-sans">
        <Routes>
          <Route path="/" element={<Home />} />
          <Route
            path="/register"
            element={
              <PublicRoute>
                <Register />
              </PublicRoute>
            }
          />
          <Route
            path="/login"
            element={
              <PublicRoute>
                <Login />
              </PublicRoute>
            }
          />
          <Route
            path="/dashboard"
            element={
              <ProtectedRoute>
                <Dashboard />
              </ProtectedRoute>
            }
          />
          <Route
            path="/records"
            element={
              <ProtectedRoute>
                <Records />
              </ProtectedRoute>
            }
          />
          <Route
            path="/profile"
            element={
              <ProtectedRoute>
                <Profile />
              </ProtectedRoute>
            }
          />
          <Route
            path="/provider-access"
            element={
              <ProtectedRoute>
                <ProviderAccess />
              </ProtectedRoute>
            }
          />
          <Route
            path="/odentity"
            element={
              <ProtectedRoute>
                <OdentityScore />
              </ProtectedRoute>
            }
          />
          <Route
            path="/card"
            element={
              <ProtectedRoute>
                <Card />
              </ProtectedRoute>
            }
          />
          <Route
            path="/credentials"
            element={
              <ProtectedRoute>
                <CredentialVault />
              </ProtectedRoute>
            }
          />
          <Route
            path="/credentials/settings"
            element={
              <ProtectedRoute>
                <CredentialSettings />
              </ProtectedRoute>
            }
          />
          <Route
            path="/pace"
            element={
              <ProtectedRoute>
                <Pace />
              </ProtectedRoute>
            }
          />
          <Route
            path="/settings"
            element={
              <ProtectedRoute>
                <Settings />
              </ProtectedRoute>
            }
          />
          <Route
            path="/health-vault"
            element={
              <ProtectedRoute>
                <Records />
              </ProtectedRoute>
            }
          />
          <Route
            path="/scan-history"
            element={
              <ProtectedRoute>
                <ScanHistory />
              </ProtectedRoute>
            }
          />
          <Route
            path="/alert-history"
            element={
              <ProtectedRoute>
                <AlertHistory />
              </ProtectedRoute>
            }
          />
          <Route
            path="/audit-trail"
            element={
              <ProtectedRoute>
                <AuditTrail />
              </ProtectedRoute>
            }
          />
          <Route
            path="/marketplace"
            element={
              <ProtectedRoute>
                <Marketplace />
              </ProtectedRoute>
            }
          />
          <Route
            path="/library"
            element={
              <ProtectedRoute>
                <Library />
              </ProtectedRoute>
            }
          />
          <Route path="/verify" element={<VerifyEmail />} />
          <Route
            path="/notifications"
            element={
              <ProtectedRoute>
                <Notifications />
              </ProtectedRoute>
            }
          />
          <Route
            path="/research"
            element={
              <ProtectedRoute>
                <Research />
              </ProtectedRoute>
            }
          />
          <Route path="/trustee/accept" element={<TrusteeAccept />} />
          <Route path="/invite" element={<TrusteeAccept />} />
          <Route path="/trustee/login" element={<TrusteeLogin />} />
          <Route path="/trustee/dashboard" element={<TrusteeDashboard />} />
          <Route path="/trustee/subscriber/:subscriberDid" element={<TrusteeSubscriberDetail />} />
          <Route path="/provider/register" element={<ProviderRegister />} />
          <Route path="/provider/login" element={<ProviderLogin />} />
          <Route path="/provider/dashboard" element={<ProviderDashboard />} />
          <Route path="*" element={<div className="min-h-screen flex items-center justify-center"><p className="text-gray-500 text-xl">404 - Page Not Found</p></div>} />
        </Routes>
      </div>
    </AuthProvider>
  );
}

export default App;
