import React, { createContext, useContext, useState, useEffect, useCallback } from 'react';
import api from '../services/api';

const AuthContext = createContext(null);
const VIEW_AS_ADMIN_TOKEN_KEY = 'livesafe_admin_token_before_view_as';

export function AuthProvider({ children }) {
  const [user, setUser] = useState(null);
  const [token, setToken] = useState(localStorage.getItem('livesafe_token'));
  const [loading, setLoading] = useState(true);

  const fetchUser = useCallback(async () => {
    if (!token) {
      setLoading(false);
      return;
    }
    try {
      const response = await api.get('/auth/me');
      setUser(response.data);
    } catch (err) {
      console.error('Failed to fetch user:', err);
      localStorage.removeItem('livesafe_token');
      localStorage.removeItem('livesafe_user');
      localStorage.removeItem(VIEW_AS_ADMIN_TOKEN_KEY);
      setToken(null);
      setUser(null);
    } finally {
      setLoading(false);
    }
  }, [token]);

  useEffect(() => {
    fetchUser();
  }, [fetchUser]);

  const register = async (email, password, firstName, lastName, isHero) => {
    const response = await api.post('/auth/register', {
      email,
      password,
      first_name: firstName || undefined,
      last_name: lastName || undefined,
      is_hero: isHero || false,
      is_military: isHero || false,
    });
    const { user: newUser, token: newToken } = response.data;
    localStorage.removeItem(VIEW_AS_ADMIN_TOKEN_KEY);
    localStorage.setItem('livesafe_token', newToken);
    localStorage.setItem('livesafe_user', JSON.stringify(newUser));
    setToken(newToken);
    setUser(newUser);
    return newUser;
  };

  const login = async (email, password) => {
    const response = await api.post('/auth/login', { email, password });
    const { user: loggedInUser, token: newToken } = response.data;
    localStorage.removeItem(VIEW_AS_ADMIN_TOKEN_KEY);
    localStorage.setItem('livesafe_token', newToken);
    localStorage.setItem('livesafe_user', JSON.stringify(loggedInUser));
    setToken(newToken);
    setUser(loggedInUser);
    return loggedInUser;
  };

  const logout = () => {
    localStorage.removeItem('livesafe_token');
    localStorage.removeItem('livesafe_user');
    localStorage.removeItem(VIEW_AS_ADMIN_TOKEN_KEY);
    setToken(null);
    setUser(null);
  };

  const startViewAsRole = async (role) => {
    const adminToken = localStorage.getItem(VIEW_AS_ADMIN_TOKEN_KEY) || token;
    if (!adminToken) {
      throw new Error('Admin session required');
    }
    const response = await api.post('/auth/view-as', { role }, {
      headers: { Authorization: `Bearer ${adminToken}` },
    });
    const { user: viewUser, token: viewToken } = response.data;
    localStorage.setItem(VIEW_AS_ADMIN_TOKEN_KEY, adminToken);
    localStorage.setItem('livesafe_token', viewToken);
    localStorage.setItem('livesafe_user', JSON.stringify(viewUser));
    setToken(viewToken);
    setUser(viewUser);
    return viewUser;
  };

  const stopViewAsRole = async () => {
    const adminToken = localStorage.getItem(VIEW_AS_ADMIN_TOKEN_KEY);
    if (!adminToken) {
      return user;
    }
    localStorage.setItem('livesafe_token', adminToken);
    localStorage.removeItem(VIEW_AS_ADMIN_TOKEN_KEY);
    setToken(adminToken);

    const response = await api.get('/auth/me', {
      headers: { Authorization: `Bearer ${adminToken}` },
    });
    localStorage.setItem('livesafe_user', JSON.stringify(response.data));
    setUser(response.data);
    return response.data;
  };

  const value = {
    user,
    token,
    loading,
    isAuthenticated: !!user,
    isViewingAsRole: Boolean(user?.view_as?.active),
    register,
    login,
    logout,
    refreshUser: fetchUser,
    startViewAsRole,
    stopViewAsRole,
  };

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
}

export function useAuth() {
  const context = useContext(AuthContext);
  if (!context) {
    throw new Error('useAuth must be used within an AuthProvider');
  }
  return context;
}

export default AuthContext;
