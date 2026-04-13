import { createContext, useContext, useState, useCallback, type ReactNode } from 'react';
import React from 'react';

export interface AuthState {
  did: string;
  displayName: string;
  role: string;
}

interface AuthContextType {
  auth: AuthState | null;
  isAuthenticated: boolean;
  login: (state: AuthState) => void;
  logout: () => void;
}

const STORAGE_KEY = 'crosschecked_auth';

const AuthContext = createContext<AuthContextType | null>(null);

export function AuthProvider({ children }: { children: ReactNode }) {
  const [auth, setAuth] = useState<AuthState | null>(() => {
    try { const raw = sessionStorage.getItem(STORAGE_KEY); return raw ? JSON.parse(raw) : null; } catch { return null; }
  });

  const login = useCallback((state: AuthState) => {
    sessionStorage.setItem(STORAGE_KEY, JSON.stringify(state));
    setAuth(state);
  }, []);

  const logout = useCallback(() => {
    sessionStorage.removeItem(STORAGE_KEY);
    setAuth(null);
  }, []);

  return React.createElement(AuthContext.Provider, { value: { auth, isAuthenticated: auth !== null, login, logout } }, children);
}

export function useAuth(): AuthContextType {
  const ctx = useContext(AuthContext);
  if (!ctx) throw new Error('useAuth must be used within AuthProvider');
  return ctx;
}
