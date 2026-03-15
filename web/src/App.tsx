import { Routes, Route } from 'react-router-dom'
import { Layout } from './components/Layout'
import { DashboardPage } from './pages/DashboardPage'
import { DecisionDetailPage } from './pages/DecisionDetailPage'
import { CreateDecisionPage } from './pages/CreateDecisionPage'
import { DelegationsPage } from './pages/DelegationsPage'
import { AuditTrailPage } from './pages/AuditTrailPage'

export default function App() {
  return (
    <Routes>
      <Route element={<Layout />}>
        <Route path="/" element={<DashboardPage />} />
        <Route path="/decisions/:id" element={<DecisionDetailPage />} />
        <Route path="/decisions/new" element={<CreateDecisionPage />} />
        <Route path="/delegations" element={<DelegationsPage />} />
        <Route path="/audit" element={<AuditTrailPage />} />
      </Route>
    </Routes>
  )
}
