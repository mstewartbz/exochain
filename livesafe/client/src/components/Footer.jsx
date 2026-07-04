import React from 'react';
import { Link } from 'react-router-dom';
import { useAuth } from '../context/AuthContext';

const FOOTER_LINKS = [
  {
    heading: 'Platform',
    links: [
      { label: 'Home', path: '/' },
      { label: 'Dashboard', path: '/dashboard', requiresAuth: true },
      { label: 'Health Vault', path: '/records', requiresAuth: true },
      { label: 'P.A.C.E. Safety Circle', path: '/pace', requiresAuth: true },
    ],
  },
  {
    heading: 'Account',
    links: [
      { label: 'Sign In', path: '/login', guestOnly: true },
      { label: 'Create Account', path: '/register', guestOnly: true },
      { label: 'Profile', path: '/profile', requiresAuth: true },
      { label: 'Settings', path: '/settings', requiresAuth: true },
    ],
  },
  {
    heading: 'Tools',
    links: [
      { label: 'Emergency Card', path: '/card', requiresAuth: true },
      { label: 'Audit Trail', path: '/audit-trail', requiresAuth: true },
      { label: 'Notifications', path: '/notifications', requiresAuth: true },
      { label: 'Credentials', path: '/credentials', requiresAuth: true },
    ],
  },
];

function Footer() {
  const { isAuthenticated } = useAuth();

  return (
    <footer className="bg-gray-900 text-gray-300 mt-auto" data-testid="app-footer">
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-12">
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-8">
          {/* Brand column */}
          <div>
            <Link
              to="/"
              className="text-xl font-bold text-white hover:text-sky-400 transition"
              data-testid="footer-logo-link"
            >
              LiveSafe<span className="text-emerald-400">.ai</span>
            </Link>
            <p className="mt-3 text-sm text-gray-400 leading-relaxed">
              Emergency access and consent-bound health records. LiveSafe shows
              current setup, invitation, and delivery states inside the app.
            </p>
          </div>

          {/* Link columns */}
          {FOOTER_LINKS.map(({ heading, links }) => (
            <div key={heading}>
              <h3 className="text-sm font-semibold text-white uppercase tracking-wider mb-3">
                {heading}
              </h3>
              <ul className="space-y-2" role="list">
                {links.map(({ label, path, requiresAuth, guestOnly }) => {
                  // Hide auth-required links for guests and guest-only links for logged-in users
                  if (requiresAuth && !isAuthenticated) return null;
                  if (guestOnly && isAuthenticated) return null;
                  return (
                    <li key={path}>
                      <Link
                        to={path}
                        className="text-sm text-gray-400 hover:text-white transition"
                        data-testid={`footer-link-${label.toLowerCase().replace(/\s+/g, '-')}`}
                      >
                        {label}
                      </Link>
                    </li>
                  );
                })}
              </ul>
            </div>
          ))}
        </div>

        {/* Bottom bar */}
        <div className="mt-10 pt-6 border-t border-gray-700 flex flex-col sm:flex-row justify-between items-center gap-4">
          <p className="text-sm text-gray-500">
            &copy; {new Date().getFullYear()} LiveSafe.ai — All rights reserved.
          </p>
          <div className="flex gap-6">
            <Link
              to="/login"
              className="text-sm text-gray-500 hover:text-white transition"
              data-testid="footer-link-sign-in-bottom"
            >
              Sign In
            </Link>
            <Link
              to="/register"
              className="text-sm text-gray-500 hover:text-white transition"
              data-testid="footer-link-register-bottom"
            >
              Register
            </Link>
          </div>
        </div>
      </div>
    </footer>
  );
}

export default Footer;
