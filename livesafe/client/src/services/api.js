import axios from 'axios';

// Default timeout: 30 seconds for production API calls
const DEFAULT_TIMEOUT_MS = 30000;

const api = axios.create({
  baseURL: '/api',
  timeout: DEFAULT_TIMEOUT_MS,
  headers: {
    'Content-Type': 'application/json',
  },
});

// Add auth token to requests (don't override if already set by caller)
api.interceptors.request.use((config) => {
  if (!config.headers.Authorization) {
    const token = localStorage.getItem('livesafe_token');
    if (token) {
      config.headers.Authorization = `Bearer ${token}`;
    }
  }
  return config;
});

// Handle auth errors and timeout errors
api.interceptors.response.use(
  (response) => response,
  (error) => {
    // Handle request timeout (ECONNABORTED = axios timeout)
    if (error.code === 'ECONNABORTED' || error.message?.includes('timeout')) {
      const timeoutError = new Error('Request timed out. Please check your connection and try again.');
      timeoutError.isTimeout = true;
      timeoutError.originalConfig = error.config;
      // Dispatch global event so any listener can show timeout UI
      if (typeof window !== 'undefined') {
        window.dispatchEvent(new CustomEvent('api-timeout', {
          detail: {
            url: error.config?.url,
            method: error.config?.method,
            retryFn: () => api(error.config),
          },
        }));
      }
      return Promise.reject(timeoutError);
    }

    if (error.response?.status === 401) {
      localStorage.removeItem('livesafe_token');
      localStorage.removeItem('livesafe_user');
      localStorage.removeItem('livesafe_admin_token_before_view_as');
      // Only redirect if not already on auth/login pages
      const path = window.location.pathname;
      const isOnAuthPage = path.includes('/login') || path.includes('/register') ||
        path.includes('/verify') || path.includes('/accept');
      if (!isOnAuthPage) {
        window.location.href = '/login';
      }
    }

    // Handle service unavailable (DB down, etc.)
    if (error.response?.status === 503) {
      const serviceError = new Error(
        error.response.data?.error || 'Service temporarily unavailable. Please try again shortly.'
      );
      serviceError.isServiceUnavailable = true;
      serviceError.status = 503;
      return Promise.reject(serviceError);
    }

    return Promise.reject(error);
  }
);

export { DEFAULT_TIMEOUT_MS };
export default api;
