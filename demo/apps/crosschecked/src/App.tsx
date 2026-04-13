import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom';
import { useAuth } from '@/hooks/useAuth';
import Navigation from '@/components/Navigation';
import Landing from '@/pages/Landing';
import Login from '@/pages/Login';
import Dashboard from '@/pages/Dashboard';
import SubmitProposal from '@/pages/SubmitProposal';
import ProposalDetail from '@/pages/ProposalDetail';
import Council from '@/pages/Council';
import Evidence from '@/pages/Evidence';
import Keys from '@/pages/Keys';
import Settings from '@/pages/Settings';

function AuthenticatedLayout() {
  return (
    <div className="flex h-screen overflow-hidden">
      <Navigation />
      <main className="flex-1 overflow-y-auto bg-xc-navy">
        <Routes>
          <Route path="dashboard" element={<Dashboard />} />
          <Route path="proposals/new" element={<SubmitProposal />} />
          <Route path="proposals/:id" element={<ProposalDetail />} />
          <Route path="council" element={<Council />} />
          <Route path="evidence" element={<Evidence />} />
          <Route path="keys" element={<Keys />} />
          <Route path="settings" element={<Settings />} />
          <Route path="*" element={<Navigate to="dashboard" replace />} />
        </Routes>
      </main>
    </div>
  );
}

function AppRoutes() {
  const { isAuthenticated } = useAuth();

  return (
    <Routes>
      <Route path="/" element={isAuthenticated ? <Navigate to="/dashboard" replace /> : <Landing />} />
      <Route path="/login" element={<Login mode="login" />} />
      <Route path="/register" element={<Login mode="register" />} />
      {isAuthenticated ? (
        <Route path="/*" element={<AuthenticatedLayout />} />
      ) : (
        <Route path="*" element={<Navigate to="/" replace />} />
      )}
    </Routes>
  );
}

export default function App() {
  return (
    <BrowserRouter>
      <AppRoutes />
    </BrowserRouter>
  );
}
