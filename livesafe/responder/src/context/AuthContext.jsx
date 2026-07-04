import React, { createContext, useContext, useState, useEffect, useCallback } from 'react';
import api from '../services/api';

const AuthContext = createContext(null);

export function AuthProvider({ children }) {
  const [user, setUser] = useState(null);
  const [token, setToken] = useState(localStorage.getItem('livesafe_responder_token'));
  const [loading, setLoading] = useState(true);

  const fetchUser = useCallback(async () => {
    if (!token) {
      setLoading(false);
      return;
    }
    try {
      const response = await api.get('/auth/responder/me');
      setUser(response.data);
    } catch (err) {
      console.error('Failed to fetch responder profile:', err);
      localStorage.removeItem('livesafe_responder_token');
      localStorage.removeItem('livesafe_responder_user');
      setToken(null);
      setUser(null);
    } finally {
      setLoading(false);
    }
  }, [token]);

  useEffect(() => {
    fetchUser();
  }, [fetchUser]);

  const register = async (formData) => {
    const response = await api.post('/auth/responder/register', formData);
    const { user: newUser, token: newToken } = response.data;
    localStorage.setItem('livesafe_responder_token', newToken);
    localStorage.setItem('livesafe_responder_user', JSON.stringify(newUser));
    setToken(newToken);
    setUser(newUser);
    return newUser;
  };

  const login = async (email, password) => {
    const response = await api.post('/auth/responder/login', { email, password });
    const { user: loggedInUser, token: newToken } = response.data;
    localStorage.setItem('livesafe_responder_token', newToken);
    localStorage.setItem('livesafe_responder_user', JSON.stringify(loggedInUser));
    setToken(newToken);
    setUser(loggedInUser);
    return loggedInUser;
  };

  const logout = () => {
    localStorage.removeItem('livesafe_responder_token');
    localStorage.removeItem('livesafe_responder_user');
    setToken(null);
    setUser(null);
  };

  const value = {
    user,
    token,
    loading,
    isAuthenticated: !!user,
    register,
    login,
    logout,
    refreshUser: fetchUser,
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
