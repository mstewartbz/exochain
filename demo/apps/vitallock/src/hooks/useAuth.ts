import { useState, useCallback } from 'react';

export interface AuthState {
  did: string;
  displayName: string;
  ed25519PublicHex: string;
  ed25519SecretHex: string;
  x25519PublicHex: string;
  x25519SecretHex: string;
}

const STORAGE_KEY = 'vitallock_auth';

function loadAuth(): AuthState | null {
  try {
    const raw = sessionStorage.getItem(STORAGE_KEY);
    return raw ? JSON.parse(raw) : null;
  } catch {
    return null;
  }
}

export function useAuth() {
  const [auth, setAuth] = useState<AuthState | null>(loadAuth);

  const login = useCallback((state: AuthState) => {
    sessionStorage.setItem(STORAGE_KEY, JSON.stringify(state));
    setAuth(state);
  }, []);

  const logout = useCallback(() => {
    sessionStorage.removeItem(STORAGE_KEY);
    setAuth(null);
  }, []);

  return {
    auth,
    isAuthenticated: auth !== null,
    login,
    logout,
  };
}
