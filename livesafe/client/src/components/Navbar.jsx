import React, { useState } from 'react';
import { Link, useLocation, useNavigate } from 'react-router-dom';
import { useAuth } from '../context/AuthContext';
import { useNotifications } from '../context/NotificationsContext';

const NAV_LINKS = [
  { label: 'Dashboard', path: '/dashboard' },
  { label: 'Health Vault', path: '/records' },
  { label: 'PACE', path: '/pace' },
  { label: 'Card', path: '/card' },
  { label: 'Marketplace', path: '/marketplace' },
  { label: 'Library', path: '/library' },
  { label: 'Audit Trail', path: '/audit-trail' },
  { label: 'Settings', path: '/settings' },
];

function Navbar() {
  const { user, logout, isViewingAsRole, stopViewAsRole } = useAuth();
  const location = useLocation();
  const navigate = useNavigate();
  const { unreadCount } = useNotifications();
  const [mobileMenuOpen, setMobileMenuOpen] = useState(false);

  const handleLogout = () => {
    logout();
    navigate('/login');
    setMobileMenuOpen(false);
  };

  const handleExitViewAs = async () => {
    await stopViewAsRole();
    navigate('/admin/dashboard');
    setMobileMenuOpen(false);
  };

  const closeMobileMenu = () => setMobileMenuOpen(false);

  return (
    <nav className="bg-white shadow-sm border-b border-gray-200" data-testid="main-navbar">
      {isViewingAsRole && (
        <div className="bg-amber-50 border-b border-amber-200" data-testid="view-as-banner">
          <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-2 flex flex-col sm:flex-row sm:items-center sm:justify-between gap-2">
            <p className="text-sm font-medium text-amber-900">
              Viewing as {user?.view_as?.role || user?.role}. Your real role is {user?.view_as?.actual_role || 'subscriber_admin'}.
            </p>
            <button
              onClick={handleExitViewAs}
              className="w-fit px-3 py-1.5 text-sm font-medium text-amber-900 border border-amber-300 rounded-md hover:bg-amber-100 transition"
              data-testid="exit-view-as-btn"
            >
              Exit View As
            </button>
          </div>
        </div>
      )}
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
        <div className="flex justify-between items-center h-16">
          {/* Logo */}
          <div className="flex items-center gap-8">
            <Link to="/" className="text-xl font-bold text-sky-700 hover:text-sky-800 transition" data-testid="logo-link">
              LiveSafe<span className="text-emerald-500">.ai</span>
            </Link>

            {/* Nav Links - desktop only (lg+ breakpoint to avoid tablet overflow) */}
            <div className="hidden lg:flex items-center gap-1">
              {NAV_LINKS.map(link => {
                const isActive = location.pathname === link.path ||
                  (link.path === '/records' && location.pathname.startsWith('/records')) ||
                  (link.path === '/settings' && (location.pathname === '/settings' || location.pathname === '/profile' || location.pathname.startsWith('/credentials/settings')));
                return (
                  <Link
                    key={link.path}
                    to={link.path}
                    className={`px-3 py-2 rounded-md text-sm font-medium transition ${
                      isActive
                        ? 'bg-sky-50 text-sky-700 font-semibold'
                        : 'text-gray-600 hover:text-sky-700 hover:bg-gray-50'
                    }`}
                  >
                    {link.label}
                  </Link>
                );
              })}
            </div>
          </div>

          {/* Right side - desktop */}
          <div className="flex items-center gap-4">
            <span className="hidden lg:block text-sm text-gray-500">{user?.email}</span>

            {/* Notifications link with unread badge (Feature #294) */}
            {user && (
              <Link
                to="/notifications"
                className={`relative px-3 py-2 rounded-md text-sm font-medium transition ${
                  location.pathname === '/notifications'
                    ? 'bg-sky-50 text-sky-700 font-semibold'
                    : 'text-gray-600 hover:text-sky-700 hover:bg-gray-50'
                }`}
                data-testid="notifications-nav-link"
                aria-label={unreadCount > 0 ? `Notifications (${unreadCount} unread)` : 'Notifications'}
              >
                🔔
                {unreadCount > 0 && (
                  <span
                    className="absolute -top-1 -right-1 inline-flex items-center justify-center h-5 min-w-[1.25rem] px-1 rounded-full text-xs font-bold bg-red-500 text-white leading-none"
                    data-testid="notification-badge"
                    aria-label={`${unreadCount} unread notifications`}
                  >
                    {unreadCount > 99 ? '99+' : unreadCount}
                  </span>
                )}
              </Link>
            )}

            <button
              onClick={handleLogout}
              className="hidden lg:block px-3 py-2 text-sm text-gray-600 hover:text-red-600 transition rounded-md hover:bg-red-50"
            >
              Sign Out
            </button>

            {/* Hamburger menu button - shown below lg breakpoint */}
            <button
              onClick={() => setMobileMenuOpen(prev => !prev)}
              className="lg:hidden inline-flex items-center justify-center p-2 rounded-md text-gray-500 hover:text-sky-700 hover:bg-gray-100 transition focus:outline-none focus:ring-2 focus:ring-sky-500 focus:ring-inset"
              aria-expanded={mobileMenuOpen}
              aria-label="Toggle navigation menu"
              data-testid="hamburger-menu-btn"
            >
              {mobileMenuOpen ? (
                /* X icon when open */
                <svg className="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor" aria-hidden="true">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
                </svg>
              ) : (
                /* Hamburger icon when closed */
                <svg className="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor" aria-hidden="true">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 6h16M4 12h16M4 18h16" />
                </svg>
              )}
            </button>
          </div>
        </div>
      </div>

      {/* Mobile/tablet menu dropdown */}
      {mobileMenuOpen && (
        <div className="lg:hidden border-t border-gray-200 bg-white" data-testid="mobile-menu">
          <div className="px-2 pt-2 pb-3 space-y-1">
            {NAV_LINKS.map(link => {
              const isActive = location.pathname === link.path ||
                (link.path === '/records' && location.pathname.startsWith('/records')) ||
                (link.path === '/settings' && (location.pathname === '/settings' || location.pathname === '/profile' || location.pathname.startsWith('/credentials/settings')));
              return (
                <Link
                  key={link.path}
                  to={link.path}
                  onClick={closeMobileMenu}
                  className={`block px-3 py-2 rounded-md text-base font-medium transition ${
                    isActive
                      ? 'bg-sky-50 text-sky-700 font-semibold'
                      : 'text-gray-600 hover:text-sky-700 hover:bg-gray-50'
                  }`}
                  data-testid={`mobile-nav-${link.label.toLowerCase().replace(/\s+/g, '-')}`}
                >
                  {link.label}
                </Link>
              );
            })}
          </div>
          {user && (
            <div className="px-2 pt-0 pb-3 border-t border-gray-100">
              <div className="px-3 py-2 text-sm text-gray-500">{user.email}</div>
              <Link
                to="/notifications"
                onClick={closeMobileMenu}
                className="block px-3 py-2 rounded-md text-base font-medium text-gray-600 hover:text-sky-700 hover:bg-gray-50 transition"
                data-testid="mobile-nav-notifications"
              >
                🔔 Notifications
                {unreadCount > 0 && (
                  <span className="ml-2 inline-flex items-center justify-center h-5 min-w-[1.25rem] px-1 rounded-full text-xs font-bold bg-red-500 text-white">
                    {unreadCount > 99 ? '99+' : unreadCount}
                  </span>
                )}
              </Link>
              <button
                onClick={handleLogout}
                className="block w-full text-left px-3 py-2 rounded-md text-base font-medium text-gray-600 hover:text-red-600 hover:bg-red-50 transition"
                data-testid="mobile-sign-out-btn"
              >
                Sign Out
              </button>
            </div>
          )}
        </div>
      )}
    </nav>
  );
}

export default Navbar;
