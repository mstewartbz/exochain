/**
 * LiveSafe.ai Service Worker
 * Provides offline capability and caching for PWA installability
 */

const CACHE_NAME = 'livesafe-v1';
const STATIC_CACHE_NAME = 'livesafe-static-v1';

// Resources to cache on install (app shell)
const APP_SHELL = [
  '/',
  '/manifest.json',
  '/icon-192x192.png',
  '/icon-512x512.png',
  '/apple-touch-icon.png'
];

// Install event - cache the app shell
self.addEventListener('install', (event) => {
  console.log('[ServiceWorker] Install');
  event.waitUntil(
    caches.open(STATIC_CACHE_NAME).then((cache) => {
      console.log('[ServiceWorker] Caching app shell');
      return cache.addAll(APP_SHELL).catch((err) => {
        console.warn('[ServiceWorker] Failed to cache some resources:', err);
      });
    })
  );
  // Skip waiting so the new SW activates immediately
  self.skipWaiting();
});

// Activate event - clean up old caches
self.addEventListener('activate', (event) => {
  console.log('[ServiceWorker] Activate');
  event.waitUntil(
    caches.keys().then((cacheNames) => {
      return Promise.all(
        cacheNames
          .filter((name) => name !== CACHE_NAME && name !== STATIC_CACHE_NAME)
          .map((name) => {
            console.log('[ServiceWorker] Removing old cache:', name);
            return caches.delete(name);
          })
      );
    })
  );
  // Claim all clients so SW controls pages immediately
  self.clients.claim();
});

// Fetch event - network-first for API calls, cache-first for static assets
self.addEventListener('fetch', (event) => {
  const url = new URL(event.request.url);

  // Skip non-GET requests
  if (event.request.method !== 'GET') {
    return;
  }

  // Skip API calls - always use network for API
  if (url.pathname.startsWith('/api/')) {
    return;
  }

  // Skip cross-origin requests
  if (url.origin !== self.location.origin) {
    return;
  }

  // For navigation requests (HTML pages), use network-first with cache fallback
  if (event.request.mode === 'navigate') {
    event.respondWith(
      fetch(event.request)
        .then((response) => {
          // Cache the successful response
          if (response.ok) {
            const responseClone = response.clone();
            caches.open(CACHE_NAME).then((cache) => {
              cache.put(event.request, responseClone);
            });
          }
          return response;
        })
        .catch(() => {
          // Offline fallback - serve cached homepage
          return caches.match('/') || caches.match(event.request);
        })
    );
    return;
  }

  // For static assets (images, scripts, etc.), use cache-first with network fallback
  if (
    url.pathname.endsWith('.png') ||
    url.pathname.endsWith('.jpg') ||
    url.pathname.endsWith('.svg') ||
    url.pathname.endsWith('.ico') ||
    url.pathname === '/manifest.json'
  ) {
    event.respondWith(
      caches.match(event.request).then((cachedResponse) => {
        if (cachedResponse) {
          return cachedResponse;
        }
        return fetch(event.request).then((response) => {
          if (response.ok) {
            const responseClone = response.clone();
            caches.open(STATIC_CACHE_NAME).then((cache) => {
              cache.put(event.request, responseClone);
            });
          }
          return response;
        });
      })
    );
    return;
  }
});

// Listen for messages from the app
self.addEventListener('message', (event) => {
  if (event.data && event.data.type === 'SKIP_WAITING') {
    self.skipWaiting();
  }
});
