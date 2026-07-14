import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom';
import { useAuth } from '@/hooks/useAuth';
import Navigation from '@/components/Navigation';
import Landing from '@/pages/Landing';
import Login from '@/pages/Login';
import Dashboard from '@/pages/Dashboard';
import EmergencyPlans from '@/pages/EmergencyPlans';
import IceCard from '@/pages/IceCard';
import PaceNetwork from '@/pages/PaceNetwork';
import GoldenHour from '@/pages/GoldenHour';
import Wellness from '@/pages/Wellness';
import Settings from '@/pages/Settings';

function AuthenticatedLayout() {
  const { auth, logout } = useAuth();

  if (!auth) return <Navigate to="/login" replace />;

  return (
    <div className="flex min-h-screen bg-[#0a1628]">
      <Navigation displayName={auth.displayName} onLogout={logout} />
      <main className="flex-1 overflow-y-auto">
        <Routes>
          <Route path="/dashboard" element={<Dashboard />} />
          <Route path="/plans" element={<EmergencyPlans />} />
          <Route path="/ice-card" element={<IceCard />} />
          <Route path="/pace" element={<PaceNetwork />} />
          <Route path="/golden-hour" element={<GoldenHour />} />
          <Route path="/wellness" element={<Wellness />} />
          <Route path="/settings" element={<Settings />} />
          <Route path="*" element={<Navigate to="/dashboard" replace />} />
        </Routes>
      </main>
    </div>
  );
}

export default function App() {
  const { isAuthenticated } = useAuth();

  return (
    <BrowserRouter>
      <Routes>
        <Route path="/" element={isAuthenticated ? <Navigate to="/dashboard" replace /> : <Landing />} />
        <Route path="/login" element={isAuthenticated ? <Navigate to="/dashboard" replace /> : <Login mode="login" />} />
        <Route path="/register" element={isAuthenticated ? <Navigate to="/dashboard" replace /> : <Login mode="register" />} />
        <Route path="/*" element={<AuthenticatedLayout />} />
      </Routes>
    </BrowserRouter>
  );
}
